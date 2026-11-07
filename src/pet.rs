//! The pet: needs, actions, speech, rubber-duck mode, save/load.

use std::collections::HashMap;
use std::fs;

use crate::rng::Rng;
use crate::species::{Mood, Reaction, Species, Stage};
use crate::term::{now_secs, save_path};

/// Seconds until the egg hatches.
pub const EGG_SECS: f64 = 25.0;
/// Seconds the pet stays a baby.
pub const BABY_SECS: f64 = 20.0 * 60.0;
/// Maximum number of seconds of offline time that gets simulated.
pub const MAX_OFFLINE_SECS: f64 = 8.0 * 3600.0;
/// How fast needs drop while the program is closed.
pub const OFFLINE_RATE: f64 = 0.4;

pub struct Pet {
    pub name: String,
    pub species: Species,
    pub age: f64,    // seconds since the egg
    pub food: f64,   // fullness 0..100
    pub joy: f64,    // mood 0..100
    pub energy: f64, // energy 0..100
    pub clean: f64,  // cleanliness 0..100
    pub health: f64, // health 1..100 (the pet never dies!)
    pub asleep: bool,
    pub idle: f64,   // seconds since last attention
    pub care: f64,   // care points → "bond"
    pub reaction: Reaction,
    pub reaction_left: f64,
    pub duck: bool, // rubber-duck mode is on (not saved)
}

impl Pet {
    pub fn new_egg(name: String, species: Species) -> Pet {
        Pet {
            name,
            species,
            age: 0.0,
            food: 80.0,
            joy: 80.0,
            energy: 90.0,
            clean: 100.0,
            health: 100.0,
            asleep: false,
            idle: 0.0,
            care: 0.0,
            reaction: Reaction::None,
            reaction_left: 0.0,
            duck: false,
        }
    }

    pub fn stage(&self) -> Stage {
        if self.age < EGG_SECS {
            Stage::Egg
        } else if self.age < EGG_SECS + BABY_SECS {
            Stage::Baby
        } else {
            Stage::Adult
        }
    }

    pub fn egg_progress(&self) -> f64 {
        (self.age / EGG_SECS).min(1.0)
    }

    pub fn mood(&self) -> Mood {
        if self.stage() == Stage::Egg {
            return Mood::Egg;
        }
        if self.asleep {
            return Mood::Sleeping;
        }
        if self.health < 30.0 {
            return Mood::Sick;
        }
        if self.food < 25.0 {
            return Mood::Hungry;
        }
        if self.energy < 20.0 {
            return Mood::Tired;
        }
        if self.clean < 25.0 {
            return Mood::Grumpy;
        }
        if self.idle > 180.0 && self.joy < 60.0 {
            return Mood::Lonely;
        }
        if self.joy < 30.0 {
            return Mood::Sad;
        }
        if self.joy > 85.0 && self.food > 60.0 && self.energy > 50.0 && self.clean > 50.0 {
            return Mood::Ecstatic;
        }
        if self.joy > 55.0 {
            return Mood::Happy;
        }
        Mood::Neutral
    }

    // ── Time passes ──

    pub fn tick(&mut self, dt: f64) -> Option<Stage> {
        self.advance(dt, 1.0)
    }

    /// Advances `dt` seconds. `rate` scales how fast needs drop.
    /// Returns the new life stage if it just changed.
    pub fn advance(&mut self, dt: f64, rate: f64) -> Option<Stage> {
        let before = self.stage();
        self.age += dt;
        self.idle += dt;
        if self.reaction_left > 0.0 {
            self.reaction_left -= dt;
            if self.reaction_left <= 0.0 {
                self.reaction = Reaction::None;
            }
        }
        if before != Stage::Egg {
            self.decay(dt * rate);
        }
        let after = self.stage();
        if after != before {
            Some(after)
        } else {
            None
        }
    }

    pub fn decay(&mut self, dt: f64) {
        let (rf, rj, re, rc) = self.species.rates();
        if self.asleep {
            self.energy += 0.30 * dt;
            self.food -= 0.015 * rf * dt;
            if self.energy >= 100.0 {
                self.asleep = false;
            }
        } else {
            self.food -= 0.05 * rf * dt;
            self.joy -= 0.04 * rj * dt;
            self.energy -= 0.03 * re * dt;
            self.clean -= 0.02 * rc * dt;
            if self.idle > 120.0 {
                self.joy -= 0.03 * dt; // loneliness
            }
            if self.energy <= 2.0 {
                self.asleep = true; // fell asleep from exhaustion
            }
        }

        let mut hp = 0.0;
        if self.food < 10.0 {
            hp -= 0.08;
        }
        if self.clean < 10.0 {
            hp -= 0.04;
        }
        if hp == 0.0 && self.food > 40.0 && self.clean > 40.0 {
            hp = 0.03; // slow recovery
        }
        self.health += hp * dt;
        self.fix_ranges();
    }

    /// Simulates the time the program wasn't running.
    pub fn catch_up(&mut self, away_secs: u64) {
        let mut left = (away_secs as f64).min(MAX_OFFLINE_SECS);
        while left > 0.0 {
            let step = left.min(30.0);
            self.advance(step, OFFLINE_RATE);
            left -= step;
        }
    }

    pub fn fix_ranges(&mut self) {
        self.food = self.food.clamp(0.0, 100.0);
        self.joy = self.joy.clamp(0.0, 100.0);
        self.energy = self.energy.clamp(0.0, 100.0);
        self.clean = self.clean.clamp(0.0, 100.0);
        self.health = self.health.clamp(1.0, 100.0);
    }

    // ── Actions ──

    pub fn touch(&mut self, r: Reaction, secs: f64) {
        self.reaction = r;
        self.reaction_left = secs;
        self.idle = 0.0;
        self.fix_ranges();
    }

    pub fn blocked(&self, egg_msg: &str) -> Option<String> {
        if self.stage() == Stage::Egg {
            return Some(egg_msg.to_string());
        }
        if self.asleep {
            return Some(format!("{} is asleep right now. Wake it with 'z'.", self.name));
        }
        None
    }

    pub fn feed(&mut self, rng: &mut Rng) -> String {
        if let Some(m) = self.blocked("An egg can't eat yet :)") {
            return m;
        }
        if self.food > 92.0 {
            self.joy -= 3.0;
            self.fix_ranges();
            return format!("{} is stuffed and refuses any more.", self.name);
        }
        self.food += 30.0;
        self.clean -= 3.0;
        self.care += 1.0;
        self.touch(Reaction::Eating, 4.0);
        let base = format!("{} munches on {}. Yum!", self.name, self.species.food());
        if rng.chance(0.35) {
            let fact = self.species.random_fact(rng);
            format!("{}\n  * {} {}", base, self.species.sound(), fact)
        } else {
            base
        }
    }

    pub fn play(&mut self) -> String {
        if let Some(m) = self.blocked("The egg can't play yet. Try petting it!") {
            return m;
        }
        if self.energy < 15.0 {
            return format!("{} is too tired to play.", self.name);
        }
        if self.food < 15.0 {
            return format!("{} is too hungry to play.", self.name);
        }
        self.joy += 25.0;
        self.energy -= 12.0;
        self.food -= 6.0;
        self.clean -= 5.0;
        self.care += 1.0;
        self.touch(Reaction::Playing, 4.0);
        let how = match self.species {
            Species::Jellyfish => "syncs up with an invisible second jellyfish – cooperative hunting!",
            Species::Frog => "hops around and wipes its feet clean – keeps the toe pads sticky",
            Species::Fox => "does a vole jump – head-first into the imaginary snow!",
            Species::Glimmi => "chases its own sparkle through the air",
            Species::Penguin => "plays 'first in' – shoves an invisible friend off the ice edge",
        };
        format!("{} {}!", self.name, how)
    }

    pub fn pet_it(&mut self) -> String {
        if self.stage() == Stage::Egg {
            self.age += 3.0;
            self.care += 0.5;
            self.touch(Reaction::Loved, 3.0);
            return "You warm the egg... it wiggles!".to_string();
        }
        self.joy += if self.asleep { 3.0 } else { 10.0 };
        self.care += 0.5;
        self.touch(Reaction::Loved, 4.0);
        if self.asleep {
            format!("{} hums contentedly in its sleep.", self.name)
        } else {
            format!("{} enjoys the cuddles.", self.name)
        }
    }

    pub fn wash(&mut self) -> String {
        if let Some(m) = self.blocked("The egg is clean enough :)") {
            return m;
        }
        self.clean = 100.0;
        self.care += 1.0;
        let (dj, text) = match self.species {
            Species::Jellyfish => (3.0, "releases a shimmering mucus cloud – its skin cleans itself"),
            Species::Frog => (8.0, "wipes its skin with its front legs, spreading moisture"),
            Species::Fox => (-6.0, "scratches fiercely with a hind paw and licks its fur – no water, please!"),
            Species::Glimmi => (4.0, "rearranges its sparks – that is how a Glimmi preens"),
            Species::Penguin => (6.0, "spreads oil from its uropygial gland through the plumage – keeps it waterproof"),
        };
        self.joy += dj;
        self.touch(Reaction::Washing, 4.0);
        format!("{} {}.", self.name, text)
    }

    pub fn medicine(&mut self) -> String {
        if let Some(m) = self.blocked("The egg isn't sick.") {
            return m;
        }
        if self.health > 70.0 {
            return format!("{} is perfectly healthy and needs no medicine.", self.name);
        }
        let (disease, cure) = self.species.disease_and_cure();
        self.health += 45.0;
        self.joy -= 5.0;
        self.touch(Reaction::Medicine, 3.0);
        format!(
            "{} had {}. Treatment: {}. {} feels better!",
            self.name, disease, cure, self.name
        )
    }

    pub fn toggle_sleep(&mut self) -> String {
        if self.stage() == Stage::Egg {
            return "The egg is already asleep all the time.".to_string();
        }
        if self.asleep {
            self.asleep = false;
            self.idle = 0.0;
            if self.energy < 60.0 {
                self.joy -= 4.0;
                self.fix_ranges();
                return format!("{} gets woken up and is still pretty groggy.", self.name);
            }
            return format!("{} stretches and yawns.", self.name);
        }
        if self.energy > 85.0 {
            return format!("{} is wide awake and doesn't want to sleep.", self.name);
        }
        self.asleep = true;
        format!("{} curls up. Good night!", self.name)
    }

    // ── Speaking ──

    /// A sentence matching the current mood.
    pub fn speech(&self, rng: &mut Rng) -> String {
        let mood = self.mood();
        let food = self.species.food();
        let text: String = match mood {
            Mood::Egg => rng
                .pick(&["...", "*knock knock*", "*wiggle wiggle*", "It's nice and warm in here..."])
                .to_string(),
            Mood::Sleeping => self.species.dream().to_string(),
            Mood::Sick => rng
                .pick(&[
                    "I feel so queasy... medicine please?",
                    "I don't feel good at all...",
                    "Cough... I'm so dizzy.",
                ])
                .to_string(),
            Mood::Hungry => match rng.range(3) {
                0 => format!("My tummy is growling! Got any {} for me?", food),
                1 => "Hungry... hungry... HUNGRY!".to_string(),
                _ => format!("I'm already dreaming of {}...", food),
            },
            Mood::Tired => rng
                .pick(&[
                    "*yawn* I'm sooo tired...",
                    "My eyes are about to close...",
                    "Time for a nap?",
                ])
                .to_string(),
            Mood::Grumpy => rng
                .pick(&[
                    "I feel dirty! Ugh!",
                    "Yuck, I desperately need a bath.",
                    "Hmph. It's gross in here.",
                ])
                .to_string(),
            Mood::Sad => rng
                .pick(&[
                    "I'm so bored...",
                    "Will you play with me? Please?",
                    "I'm sad...",
                ])
                .to_string(),
            Mood::Lonely => rng
                .pick(&[
                    "Where have you been so long?",
                    "I missed you...",
                    "Hello? Is anybody there?",
                ])
                .to_string(),
            Mood::Neutral => {
                let q = self.species.quirks();
                if rng.chance(0.5) {
                    rng.pick(&q).to_string()
                } else {
                    rng.pick(&[
                        "Hm, what are we doing today?",
                        "I'm watching everything closely.",
                        "All quiet here.",
                    ])
                    .to_string()
                }
            }
            Mood::Happy => {
                let q = self.species.quirks();
                if rng.chance(0.4) {
                    rng.pick(&q).to_string()
                } else {
                    rng.pick(&[
                        "I'm feeling great!",
                        "You're the best!",
                        "Today is a good day!",
                    ])
                    .to_string()
                }
            }
            Mood::Ecstatic => rng
                .pick(&[
                    "I'M SO HAPPY!!!",
                    "This is the best day ever!",
                    "I love you sooo much!",
                ])
                .to_string(),
        };

        if mood != Mood::Sleeping && mood != Mood::Egg && rng.chance(0.4) {
            format!("{} {}", self.species.sound(), text)
        } else {
            text
        }
    }

    /// A sentence reacting to the last action (if one is running).
    pub fn reaction_speech(&self, rng: &mut Rng) -> Option<String> {
        if self.reaction == Reaction::None {
            return None;
        }
        if self.stage() == Stage::Egg {
            return Some("*knock knock*".to_string());
        }
        if self.asleep {
            return Some("Zzz... mhhh...".to_string());
        }
        let text: String = match self.reaction {
            Reaction::None => return None,
            Reaction::Eating => format!("Yum! {} tastes so good!", self.species.food()),
            Reaction::Playing => rng
                .pick(&[
                    "Woohoo! Again, again!",
                    "Haha, catch me if you can!",
                    "This is so much fun!",
                ])
                .to_string(),
            Reaction::Loved => rng
                .pick(&["Ohh, that feels good...", "I love you!", "More of that, please!"])
                .to_string(),
            Reaction::Washing => match self.species {
                Species::Fox => "No, no, no! No water!".to_string(),
                Species::Frog => "Splashing is the best!".to_string(),
                Species::Penguin => "Splash! Water is home!".to_string(),
                _ => "Ahh, nice and fresh!".to_string(),
            },
            Reaction::Medicine => "Bleeegh... tastes awful!".to_string(),
            Reaction::Listening => "Mhm... go on.".to_string(),
            Reaction::Eureka => "Eureka! You did it!".to_string(),
        };
        Some(format!("{} {}", self.species.sound(), text))
    }

    // ── Rubber-duck debugging ──

    /// You explain your problem, the pet listens and answers with a gentle question
    /// (like a rubber duck, it never solves anything - you do). Returns (reply, eureka).
    pub fn duck_listen(&mut self, line: &str, rng: &mut Rng, count: u32) -> (String, bool) {
        let l = line.to_lowercase();
        let negated = has_any(&l, &["not ", "n't", "doesnt", "isnt", "never", "still "]);
        let eureka = !negated
            && has_any(
                &l,
                &[
                    "aha", "got it", "found it", "found the", "fixed", "works now", "now it works",
                    "it works", "solved", "that's it", "thats it", "i get it", "figured it",
                    "of course", "understand now", "ahh", "eureka",
                ],
            );

        let text: String = if eureka {
            rng.pick(&[
                "You found it! I knew you could!",
                "Eureka! Great debugging!",
                "See? Explaining out loud works every time!",
            ])
            .to_string()
        } else if has_any(&l, &["thank", "thx"]) {
            rng.pick(&[
                "Anytime! I love listening to you.",
                "You're welcome! Snack time now?",
                "Happy to help, even if I only listened.",
            ])
            .to_string()
        } else if has_any(&l, &["ugh", "frustrat", "hate", "annoy", "tired", "give up", "stupid", "hours"]) {
            rng.pick(&[
                "Deep breath. We'll get there together.",
                "Take a short break? Bugs hate fresh eyes.",
                "You're doing better than you think.",
            ])
            .to_string()
        } else if has_any(&l, &["segfault", "panic", "exception", "error", "crash", "traceback", "stack trace", "fail"]) {
            rng.pick(&[
                "What does the error say, word for word?",
                "Which line does the error point to?",
                "Did the error change after your last edit?",
            ])
            .to_string()
        } else if has_any(&l, &["null", "none", "nil", "undefined", "empty", "missing"]) {
            rng.pick(&[
                "Can something be empty there? Who fills it?",
                "What did you expect it to hold right there?",
                "Where does that value come from originally?",
            ])
            .to_string()
        } else if has_any(&l, &["borrow", "lifetime", "ownership", "moved", "mutable", "reference"]) {
            rng.pick(&[
                "Who owns that value right there?",
                "Does anyone still need it after the move?",
                "Could you restructure who holds it?",
            ])
            .to_string()
        } else if has_any(&l, &["loop", "iterate", "for ", "while", "recurs"]) {
            rng.pick(&[
                "How often does it run? What about the last round?",
                "What are the values in the very first iteration?",
                "Is there an off-by-one hiding in there?",
            ])
            .to_string()
        } else if has_any(&l, &["index", "array", "vector", "vec", "list", "bounds", "slice"]) {
            rng.pick(&[
                "Can the index ever equal the length?",
                "How long is it right before that line?",
                "Does counting start at 0 or at 1 here?",
            ])
            .to_string()
        } else if has_any(&l, &["type", "cast", "convert", "parse", "string", "integer", "float"]) {
            rng.pick(&[
                "What type is it really at that point?",
                "Is anything converted silently on the way?",
                "How could you print its real type?",
            ])
            .to_string()
        } else if has_any(&l, &["thread", "async", "await", "race", "deadlock", "mutex", "lock", "concurren"]) {
            rng.pick(&[
                "Which part runs first? Are you sure?",
                "Who else touches that data at the same time?",
                "What if the order were flipped?",
            ])
            .to_string()
        } else if has_any(&l, &["test", "assert", "expected"]) {
            rng.pick(&[
                "What exactly did you expect, and what did you get?",
                "Does the test check what you think it checks?",
                "Can you make the test smaller?",
            ])
            .to_string()
        } else if has_any(&l, &["sometimes", "random", "flaky", "only on", "works on my"]) {
            rng.pick(&[
                "What is different when it fails? Input, timing, order?",
                "Can you make it fail every single time?",
                "What changed since it last worked?",
            ])
            .to_string()
        } else if has_any(&l, &["git", "merge", "conflict", "branch", "commit"]) {
            rng.pick(&[
                "Which version do you actually want to keep?",
                "What did the last good commit look like?",
                "Could a diff show you the surprise?",
            ])
            .to_string()
        } else if has_any(&l, &["sql", "query", "database", "select ", "join"]) {
            rng.pick(&[
                "What rows does the query return when run by hand?",
                "Does the join duplicate or drop rows?",
                "What does the smallest version of it return?",
            ])
            .to_string()
        } else if has_any(&l, &["regex", "pattern", "match"]) {
            rng.pick(&[
                "Try it on a tiny example. What does it match?",
                "Could it also match the empty string?",
                "Which part of the pattern is greedy?",
            ])
            .to_string()
        } else if has_any(&l, &["bracket", "paren", "semicolon", "syntax", "indent", "brace"]) {
            rng.pick(&[
                "Count the brackets. Do they all pair up?",
                "What is on the line just before the reported one?",
                "Is anything indented differently than it looks?",
            ])
            .to_string()
        } else if has_any(&l, &["why", "how come", "no idea", "don't know", "dont know", "stuck", "confus"]) {
            rng.pick(&[
                "What is the last thing you are sure about?",
                "If you had to guess, what would be the cause?",
                "Let's go back one step. What happens right before?",
            ])
            .to_string()
        } else if has_any(&l, &["function", "method", "class", "struct", "module"]) {
            rng.pick(&[
                "What goes in, and what should come out?",
                "Does it do just one thing?",
                "Who calls it, and with what?",
            ])
            .to_string()
        } else if has_any(&l, &["variable", "value", "state", "cache", "stale"]) {
            rng.pick(&[
                "What is its value right before that line?",
                "Is it the new value, or an old copy?",
                "Who else can change it?",
            ])
            .to_string()
        } else if has_any(&l, &["print", "log", "debug", "breakpoint"]) {
            rng.pick(&[
                "What did the output show? What did you expect?",
                "Can you print one step earlier?",
                "Where does the value first look wrong?",
            ])
            .to_string()
        } else if has_any(&l, &["if ", "else", "condition", "true", "false"]) {
            rng.pick(&[
                "Walk through the condition with a real value.",
                "Is it true when you think it is?",
                "What if both branches are wrong?",
            ])
            .to_string()
        } else if has_any(&l, &[";", "{", "}", "=>", "->", "()", "fn ", "def ", "let ", "const ", "return"]) {
            rng.pick(&[
                "I can see the code! What should this line do?",
                "And what does that line actually do?",
                "Read that line out loud, piece by piece.",
            ])
            .to_string()
        } else if count >= 6 && rng.chance(0.4) {
            rng.pick(&[
                "You've explained a lot. Getting closer?",
                "Say it in one sentence. What is wrong?",
                "Did anything you said sound off to you?",
            ])
            .to_string()
        } else {
            rng.pick(&[
                "Mhm... go on.",
                "I see... and then what happens?",
                "Interesting. What did you expect instead?",
                "Could you explain that last part again, slowly?",
                "Which part surprised you the most?",
                "Okay... and what does the next line do?",
            ])
            .to_string()
        };

        self.asleep = false;
        if eureka {
            self.joy += 20.0;
            self.care += 2.0;
            self.touch(Reaction::Eureka, 6.0);
            let fact = self.species.random_fact(rng);
            let bonus = format!("{}\n  * By the way: {}", text, fact);
            self.fix_ranges();
            return (bonus, true);
        } else {
            self.joy += 1.0;
            self.care += 0.2;
            self.touch(Reaction::Listening, 8.0);
        }
        self.fix_ranges();

        if rng.chance(0.3) {
            (format!("{} {}", self.species.sound(), text), false)
        } else {
            (text, false)
        }
    }

    // ── Saving & loading ──

    pub fn save(&self) {
        let data = format!(
            "species={}\nname={}\nage={:.1}\nfood={:.1}\njoy={:.1}\nenergy={:.1}\nclean={:.1}\nhealth={:.1}\nasleep={}\ncare={:.1}\nsaved_at={}\n",
            self.species.key(),
            self.name,
            self.age,
            self.food,
            self.joy,
            self.energy,
            self.clean,
            self.health,
            self.asleep,
            self.care,
            now_secs()
        );
        let _ = fs::write(save_path(), data);
    }

    /// Loads the pet and also returns how many seconds passed since the last save.
    pub fn load() -> Option<(Pet, u64)> {
        let text = fs::read_to_string(save_path()).ok()?;
        let mut map: HashMap<String, String> = HashMap::new();
        for line in text.lines() {
            if let Some((k, v)) = line.split_once('=') {
                map.insert(k.trim().to_string(), v.to_string());
            }
        }
        let species = Species::from_key(map.get("species")?.trim())?;
        let name = map.get("name")?.clone();
        let saved_at = map
            .get("saved_at")
            .and_then(|v| v.trim().parse::<u64>().ok())
            .unwrap_or_else(now_secs);
        let pet = Pet {
            name,
            species,
            age: get_num(&map, "age", 0.0),
            food: get_num(&map, "food", 80.0),
            joy: get_num(&map, "joy", 80.0),
            energy: get_num(&map, "energy", 80.0),
            clean: get_num(&map, "clean", 100.0),
            health: get_num(&map, "health", 100.0),
            asleep: map.get("asleep").map(|v| v.trim() == "true").unwrap_or(false),
            idle: 0.0,
            care: get_num(&map, "care", 0.0),
            reaction: Reaction::None,
            reaction_left: 0.0,
            duck: false,
        };
        let away = now_secs().saturating_sub(saved_at);
        Some((pet, away))
    }
}

/// Words that end the rubber-duck mode when typed on their own.
pub fn is_leave_word(low: &str) -> bool {
    matches!(low, "done" | "exit" | "quit" | "leave" | "stop" | "bye" | "end")
}

/// True if `text` contains any of the `words`.
pub fn has_any(text: &str, words: &[&str]) -> bool {
    words.iter().any(|w| text.contains(*w))
}

/// Reads a number from the save file, falling back to `default`.
fn get_num(map: &HashMap<String, String>, key: &str, default: f64) -> f64 {
    map.get(key)
        .and_then(|v| v.trim().parse::<f64>().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adult(sp: Species) -> Pet {
        let mut p = Pet::new_egg("Test".to_string(), sp);
        p.age = EGG_SECS + BABY_SECS + 1.0;
        p
    }

    // ── life cycle ──

    #[test]
    fn life_cycle_stages() {
        let mut p = Pet::new_egg("Test".to_string(), Species::Fox);
        assert!(p.stage() == Stage::Egg && p.mood() == Mood::Egg);
        p.age = EGG_SECS;
        assert!(p.stage() == Stage::Baby);
        p.age = EGG_SECS + BABY_SECS;
        assert!(p.stage() == Stage::Adult);
    }

    #[test]
    fn tick_reports_a_stage_change_only_once() {
        let mut p = Pet::new_egg("Test".to_string(), Species::Frog);
        assert!(p.tick(EGG_SECS + 1.0) == Some(Stage::Baby));
        assert!(p.tick(1.0).is_none());
    }

    #[test]
    fn eggs_do_not_get_hungry() {
        let mut p = Pet::new_egg("Test".to_string(), Species::Jellyfish);
        let food = p.food;
        p.advance(10.0, 1.0);
        assert!(p.food == food);
    }

    #[test]
    fn petting_an_egg_speeds_up_hatching() {
        let mut p = Pet::new_egg("Test".to_string(), Species::Glimmi);
        let msg = p.pet_it();
        assert!((p.age - 3.0).abs() < 1e-9);
        assert!(!msg.is_empty());
        assert!(p.reaction == Reaction::Loved);
    }

    #[test]
    fn eggs_cannot_eat_play_wash_or_take_medicine() {
        let mut p = Pet::new_egg("Test".to_string(), Species::Penguin);
        let mut rng = Rng::new();
        let (food, joy, clean, care) = (p.food, p.joy, p.clean, p.care);
        assert!(p.feed(&mut rng).contains("egg"));
        assert!(p.play().contains("egg"));
        assert!(p.wash().contains("egg"));
        assert!(p.medicine().contains("egg"));
        assert!(p.toggle_sleep().contains("egg"));
        assert!(p.food == food && p.joy == joy && p.clean == clean && p.care == care);
        assert!(p.reaction == Reaction::None);
    }

    // ── actions ──

    #[test]
    fn feeding_raises_food_and_a_stuffed_pet_refuses() {
        let mut p = adult(Species::Frog);
        let mut rng = Rng::new();
        p.food = 40.0;
        p.feed(&mut rng);
        assert!((p.food - 70.0).abs() < 1e-9);
        assert!(p.reaction == Reaction::Eating);

        p.food = 95.0;
        let joy = p.joy;
        let msg = p.feed(&mut rng);
        assert!(msg.contains("stuffed"));
        assert!(p.food == 95.0 && p.joy < joy);
    }

    #[test]
    fn playing_needs_energy_and_food() {
        let mut p = adult(Species::Fox);
        p.energy = 10.0;
        assert!(p.play().contains("too tired"));
        p.energy = 80.0;
        p.food = 10.0;
        assert!(p.play().contains("too hungry"));
        p.food = 80.0;
        let (joy, energy) = (p.joy, p.energy);
        p.play();
        assert!(p.joy > joy && p.energy < energy);
        assert!(p.reaction == Reaction::Playing);
    }

    #[test]
    fn sleeping_pets_cannot_be_fed_and_wake_up_groggy() {
        let mut p = adult(Species::Fox);
        let mut rng = Rng::new();
        p.energy = 30.0;
        assert!(p.toggle_sleep().contains("Good night"));
        assert!(p.asleep && p.mood() == Mood::Sleeping);
        assert!(p.feed(&mut rng).contains("asleep"));
        let joy = p.joy;
        assert!(p.toggle_sleep().contains("groggy"));
        assert!(!p.asleep && p.joy < joy);
    }

    #[test]
    fn a_wide_awake_pet_does_not_want_to_sleep() {
        let mut p = adult(Species::Penguin);
        p.energy = 95.0;
        assert!(p.toggle_sleep().contains("wide awake"));
        assert!(!p.asleep);
    }

    #[test]
    fn medicine_is_only_given_when_needed() {
        let mut p = adult(Species::Jellyfish);
        assert!(p.medicine().contains("healthy"));
        p.health = 20.0;
        assert!(p.mood() == Mood::Sick);
        p.medicine();
        assert!((p.health - 65.0).abs() < 1e-9);
        assert!(p.reaction == Reaction::Medicine);
    }

    #[test]
    fn washing_cleans_the_pet_but_annoys_the_fox() {
        let mut fox = adult(Species::Fox);
        fox.clean = 30.0;
        let joy = fox.joy;
        fox.wash();
        assert!(fox.clean == 100.0 && fox.joy < joy);

        let mut frog = adult(Species::Frog);
        frog.clean = 30.0;
        let joy = frog.joy;
        frog.wash();
        assert!(frog.clean == 100.0 && frog.joy > joy);
    }

    // ── moods ──

    #[test]
    fn mood_priorities() {
        let mut p = adult(Species::Fox);
        p.joy = 70.0;
        assert!(p.mood() == Mood::Happy);
        p.joy = 95.0;
        assert!(p.mood() == Mood::Ecstatic);
        p.food = 10.0;
        assert!(p.mood() == Mood::Hungry, "hunger beats ecstasy");
        p.health = 5.0;
        assert!(p.mood() == Mood::Sick, "sickness beats hunger");
        p.asleep = true;
        assert!(p.mood() == Mood::Sleeping, "sleep beats everything");
    }

    #[test]
    fn lonely_needs_both_neglect_and_low_mood() {
        let mut p = adult(Species::Glimmi);
        p.joy = 40.0;
        p.idle = 200.0;
        assert!(p.mood() == Mood::Lonely);
        p.joy = 80.0;
        assert!(p.mood() != Mood::Lonely);
        p.joy = 20.0;
        p.idle = 0.0;
        assert!(p.mood() == Mood::Sad);
    }

    // ── time ──

    #[test]
    fn needs_decay_but_health_never_drops_below_one() {
        let mut p = adult(Species::Frog);
        let (food, joy, energy, clean) = (p.food, p.joy, p.energy, p.clean);
        p.decay(60.0);
        assert!(p.food < food && p.joy < joy && p.energy < energy && p.clean < clean);

        p.food = 0.0;
        p.clean = 0.0;
        p.health = 1.0;
        for _ in 0..1000 {
            p.decay(100.0);
        }
        assert!(p.health >= 1.0, "the pet must never die");
        assert!(p.food >= 0.0 && p.joy >= 0.0 && p.energy >= 0.0 && p.clean >= 0.0);
    }

    #[test]
    fn sleeping_restores_energy_and_wakes_up_when_full() {
        let mut p = adult(Species::Penguin);
        p.asleep = true;
        p.energy = 50.0;
        p.decay(100.0);
        assert!(p.energy > 50.0);
        p.decay(10_000.0);
        assert!(!p.asleep && p.energy == 100.0);
    }

    #[test]
    fn an_exhausted_pet_falls_asleep() {
        let mut p = adult(Species::Fox);
        p.energy = 2.5;
        p.decay(100.0);
        assert!(p.asleep);
    }

    #[test]
    fn catch_up_ages_the_pet_but_is_capped() {
        let mut long_trip = adult(Species::Fox);
        let mut max_trip = adult(Species::Fox);
        let start = long_trip.age;
        long_trip.catch_up(10 * 24 * 3600);
        max_trip.catch_up(8 * 3600);
        assert!((long_trip.age - (start + MAX_OFFLINE_SECS)).abs() < 1e-6);
        assert!((long_trip.age - max_trip.age).abs() < 1e-6);
    }

    #[test]
    fn offline_time_drains_needs_slower_than_real_time() {
        let mut live = adult(Species::Frog);
        let mut away = adult(Species::Frog);
        live.advance(600.0, 1.0);
        away.catch_up(600);
        assert!(away.food > live.food);
    }

    #[test]
    fn reactions_expire() {
        let mut p = adult(Species::Glimmi);
        p.touch(Reaction::Playing, 2.0);
        p.advance(1.0, 1.0);
        assert!(p.reaction == Reaction::Playing);
        p.advance(1.5, 1.0);
        assert!(p.reaction == Reaction::None);
    }

    // ── speech ──

    #[test]
    fn every_mood_has_something_to_say() {
        let mut rng = Rng::new();
        for sp in Species::ALL {
            let mut p = adult(sp);
            for (food, joy, energy, clean, health, asleep, idle) in [
                (80.0, 80.0, 90.0, 100.0, 100.0, false, 0.0),
                (80.0, 95.0, 90.0, 100.0, 100.0, false, 0.0),
                (10.0, 80.0, 90.0, 100.0, 100.0, false, 0.0),
                (80.0, 80.0, 10.0, 100.0, 100.0, false, 0.0),
                (80.0, 80.0, 90.0, 10.0, 100.0, false, 0.0),
                (80.0, 40.0, 90.0, 100.0, 100.0, false, 300.0),
                (80.0, 10.0, 90.0, 100.0, 100.0, false, 0.0),
                (80.0, 80.0, 90.0, 100.0, 5.0, false, 0.0),
                (80.0, 80.0, 90.0, 100.0, 100.0, true, 0.0),
            ] {
                p.food = food;
                p.joy = joy;
                p.energy = energy;
                p.clean = clean;
                p.health = health;
                p.asleep = asleep;
                p.idle = idle;
                for _ in 0..10 {
                    assert!(!p.speech(&mut rng).trim().is_empty());
                }
            }
        }
        let egg = Pet::new_egg("Egg".to_string(), Species::Fox);
        assert!(!egg.speech(&mut rng).trim().is_empty());
    }

    // ── rubber-duck mode ──

    #[test]
    fn duck_mode_celebrates_a_eureka_and_ignores_negations() {
        let mut rng = Rng::new();
        let mut p = adult(Species::Penguin);
        let (joy, care) = (p.joy, p.care);
        let (reply, eureka) = p.duck_listen("Aha, found it! The index was off by one.", &mut rng, 1);
        assert!(eureka);
        assert!(reply.contains("By the way"), "a eureka comes with a fact");
        assert!(p.reaction == Reaction::Eureka);
        assert!(p.joy > joy && p.care >= care + 2.0);

        let (_, eureka) = p.duck_listen("It still doesn't work, I haven't found it", &mut rng, 2);
        assert!(!eureka);
    }

    #[test]
    fn duck_mode_asks_a_question_that_fits_the_topic() {
        let mut rng = Rng::new();
        let mut p = adult(Species::Fox);
        let cases: [(&str, &[&str]); 5] = [
            (
                "The program has a segfault",
                &[
                    "What does the error say, word for word?",
                    "Which line does the error point to?",
                    "Did the error change after your last edit?",
                ],
            ),
            (
                "my loop never ends",
                &[
                    "How often does it run? What about the last round?",
                    "What are the values in the very first iteration?",
                    "Is there an off-by-one hiding in there?",
                ],
            ),
            (
                "thanks a lot",
                &[
                    "Anytime! I love listening to you.",
                    "You're welcome! Snack time now?",
                    "Happy to help, even if I only listened.",
                ],
            ),
            (
                "the merge conflict is confusing",
                &[
                    "Which version do you actually want to keep?",
                    "What did the last good commit look like?",
                    "Could a diff show you the surprise?",
                ],
            ),
            (
                "hmm",
                &[
                    "Mhm... go on.",
                    "I see... and then what happens?",
                    "Interesting. What did you expect instead?",
                    "Could you explain that last part again, slowly?",
                    "Which part surprised you the most?",
                    "Okay... and what does the next line do?",
                ],
            ),
        ];
        for (input, expected) in cases {
            for n in 1..=5 {
                let (reply, eureka) = p.duck_listen(input, &mut rng, n);
                assert!(!eureka, "'{input}' is not a eureka");
                // the reply may start with the species' sound
                assert!(
                    expected.iter().any(|e| reply.ends_with(e)),
                    "unexpected reply to '{input}': {reply}"
                );
            }
        }
    }

    #[test]
    fn listening_makes_the_pet_a_bit_happier() {
        let mut rng = Rng::new();
        let mut p = adult(Species::Frog);
        p.joy = 50.0;
        p.idle = 100.0;
        p.duck_listen("so the function returns early", &mut rng, 1);
        assert!(p.joy > 50.0 && p.idle == 0.0);
        assert!(p.reaction == Reaction::Listening);
    }

    #[test]
    fn leave_words_end_duck_mode() {
        for w in ["done", "exit", "quit", "leave", "stop", "bye", "end"] {
            assert!(is_leave_word(w), "{w}");
        }
        for w in ["", "dones", "q", "why does it crash", "/done"] {
            assert!(!is_leave_word(w), "{w}");
        }
    }

    // ── saving ──

    /// The only test that touches the file system: it redirects HOME to a temporary
    /// directory first, so a real save file is never read or overwritten.
    #[test]
    fn save_and_load_round_trip_in_a_temporary_home() {
        let dir = std::env::temp_dir().join(format!("terminal-pet-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        std::env::set_var("HOME", &dir);
        std::env::set_var("USERPROFILE", &dir);
        assert!(save_path().starts_with(&dir), "must not touch the real save file");

        assert!(Pet::load().is_none(), "no save yet");

        let mut p = adult(Species::Penguin);
        p.name = "Pingu".to_string();
        p.food = 42.5;
        p.joy = 61.0;
        p.energy = 33.3;
        p.clean = 77.7;
        p.health = 88.0;
        p.care = 12.5;
        p.asleep = true;
        p.save();

        let (q, away) = Pet::load().expect("the save file should load");
        assert_eq!(q.name, "Pingu");
        assert!(q.species == Species::Penguin);
        assert!((q.age - p.age).abs() < 0.06);
        assert!((q.food - 42.5).abs() < 0.06 && (q.joy - 61.0).abs() < 0.06);
        assert!((q.energy - 33.3).abs() < 0.06 && (q.clean - 77.7).abs() < 0.06);
        assert!((q.health - 88.0).abs() < 0.06 && (q.care - 12.5).abs() < 0.06);
        assert!(q.asleep);
        assert!(q.stage() == Stage::Adult);
        assert!(away < 5, "just saved, so almost no time has passed");

        // a save from the German-language version still loads; missing numbers get defaults
        fs::write(save_path(), "species=pinguin\nname=Old\nsaved_at=0\n").unwrap();
        let (old, away) = Pet::load().expect("legacy save should load");
        assert!(old.species == Species::Penguin);
        assert!(old.food == 80.0 && old.clean == 100.0 && !old.asleep);
        assert!(away > 1_000_000);

        // garbage is ignored instead of crashing
        fs::write(save_path(), "this is not a save file").unwrap();
        assert!(Pet::load().is_none());

        let _ = fs::remove_dir_all(&dir);
    }
}
