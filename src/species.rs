//! Species, life stages, moods, reactions and the modular fact database.

use crate::rng::Rng;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Species {
    Jellyfish,
    Frog,
    Fox,
    Glimmi,
    Penguin,
}

impl Species {
    pub const ALL: [Species; 5] = [
        Species::Jellyfish,
        Species::Frog,
        Species::Fox,
        Species::Glimmi,
        Species::Penguin,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Species::Jellyfish => "Jellyfish",
            Species::Frog => "Frog",
            Species::Fox => "Fox",
            Species::Glimmi => "Glimmi",
            Species::Penguin => "Penguin",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Species::Jellyfish => "jellyfish",
            Species::Frog => "frog",
            Species::Fox => "fox",
            Species::Glimmi => "glimmi",
            Species::Penguin => "penguin",
        }
    }

    /// Old German save-file keys, so existing saves keep working.
    pub fn legacy_key(self) -> &'static str {
        match self {
            Species::Jellyfish => "qualle",
            Species::Frog => "frosch",
            Species::Fox => "fuchs",
            Species::Glimmi => "glimmi",
            Species::Penguin => "pinguin",
        }
    }

    pub fn from_key(k: &str) -> Option<Species> {
        Species::ALL
            .iter()
            .copied()
            .find(|s| s.key() == k || s.legacy_key() == k)
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Species::Jellyfish => "calm & floaty, loves water, sleeps a lot",
            Species::Frog => "greedy, loves to croak, adores baths",
            Species::Fox => "playful & clever, hates baths, gets bored fast",
            Species::Glimmi => "made-up light creature, very sensitive, sparkles when happy",
            Species::Penguin => "chubby waddler, loves fish, slides on its belly",
        }
    }

    pub fn sound(self) -> &'static str {
        match self {
            Species::Jellyfish => "*blub*",
            Species::Frog => "*ribbit*",
            Species::Fox => "*yip*",
            Species::Glimmi => "*pling*",
            Species::Penguin => "*honk*",
        }
    }

    pub fn food(self) -> &'static str {
        match self {
            Species::Jellyfish => "plankton",
            Species::Frog => "flies",
            Species::Fox => "berries",
            Species::Glimmi => "stardust",
            Species::Penguin => "fish",
        }
    }

    pub fn dream(self) -> &'static str {
        match self {
            Species::Jellyfish => "Zzz... plankton... so much plankton... Zzz",
            Species::Frog => "Zzz... flies... fat juicy flies... Zzz",
            Species::Fox => "Zzz... run, run... catch the rabbit... Zzz",
            Species::Glimmi => "Zzz... stars... so many sparkly stars... Zzz",
            Species::Penguin => "Zzz... fish... so many silvery fish... Zzz",
        }
    }

    pub fn color(self) -> &'static str {
        match self {
            Species::Jellyfish => "\x1b[96m",
            Species::Frog => "\x1b[92m",
            Species::Fox => "\x1b[38;5;208m",
            Species::Glimmi => "\x1b[95m",
            Species::Penguin => "\x1b[38;5;117m",
        }
    }

    pub fn quirks(self) -> [&'static str; 3] {
        match self {
            Species::Jellyfish => [
                "I'm just floating along...",
                "The water is so clear today.",
                "I have no brain, but I have feelings!",
            ],
            Species::Frog => [
                "It's about to rain, I can feel it in my legs.",
                "Just practicing my mega jump.",
                "Got a lily pad for me?",
            ],
            Species::Fox => [
                "I heard something in the bushes...",
                "I'm clever. And fluffy. And clever.",
                "Wanna dig something up?",
            ],
            Species::Glimmi => [
                "My sparks are dancing extra bright today!",
                "I think I just heard a star...",
                "I'm just sparkling away.",
            ],
            Species::Penguin => [
                "I waddle, therefore I am.",
                "Is it cold? I honestly can't tell.",
                "Wanna slide across the ice together?",
            ],
        }
    }

    /// How fast (hunger, boredom, tiredness, dirt) things drop for this species.
    pub fn rates(self) -> (f64, f64, f64, f64) {
        match self {
            Species::Jellyfish => (0.8, 0.7, 1.2, 0.5),
            Species::Frog => (1.4, 1.0, 1.0, 1.0),
            Species::Fox => (1.0, 1.5, 0.9, 1.2),
            Species::Glimmi => (0.9, 1.1, 1.0, 0.6),
            Species::Penguin => (1.1, 0.9, 1.0, 0.7),
        }
    }

    // ── Facts (modular knowledge base) ──

    pub fn biology_facts(self) -> &'static [&'static str] {
        match self {
            Species::Jellyfish => &[
                "Jellyfish have no brain – just a nerve net that can still learn.",
                "A jellyfish is 95–98% water.",
                "Jellyfish have lived in the oceans for 500 million years – longer than dinosaurs.",
                "Box jellyfish have 24 eyes, some with lens, cornea and retina.",
                "Some jellyfish can reverse aging after injury – they are considered 'immortal'.",
            ],
            Species::Frog => &[
                "Tadpoles are vegetarians – only adult frogs become hunters.",
                "Frogs swallow by sinking their eyeballs to push food down their throat.",
                "A Goliath frog can grow as large as a house cat.",
                "Frogs drink through their skin – mostly the belly.",
                "The golden poison frog is only 45 mm long, yet its venom could kill 10 people.",
            ],
            Species::Fox => &[
                "Although canids, foxes can partially retract their claws like cats.",
                "The arctic fox can withstand temperatures of -80 °C.",
                "The gray fox is the only wild canid that can climb trees.",
                "Red foxes reach speeds of up to 60 km/h.",
                "Foxes have vertical pupils – another cat-like trait among dogs.",
            ],
            Species::Penguin => &[
                "An emperor penguin can dive to 535 m – deeper than any other bird.",
                "Penguins have a body temperature of about 39 °C – that would be a fever for us.",
                "Emperor penguin males incubate the egg for 2 months and lose half their body weight.",
                "A colony of 20,000 penguins needs roughly 20 tonnes of food every day.",
                "Penguins 'fly' underwater – their wings became flippers.",
            ],
            Species::Glimmi => &[
                "A Glimmi can make up to 1000 light points dance at the same time.",
                "Glimmis feed on stardust – it tastes of vanilla.",
                "Glimmis clean themselves by rearranging their sparks.",
            ],
        }
    }

    pub fn behavior_facts(self) -> &'static [&'static str] {
        match self {
            Species::Jellyfish => &[
                "Some jellyfish form small groups of 2–3 and match swimming speed – possibly cooperative hunting.",
                "The upside-down jellyfish (Cassiopea) falls into a sleep-like state at night.",
                "Jellyfish repair small wounds in minutes without scars.",
                "They sense the world in a full 360 degrees.",
            ],
            Species::Frog => &[
                "Frogs only notice things that move – but they see in every direction.",
                "They wipe their skin with their front legs to spread moisture and remove dirt.",
                "Sticky toe pads clean themselves simply by walking – shear and mucus do the work.",
                "In 2001, 35 tadpoles flew into space for zero-gravity research.",
            ],
            Species::Fox => &[
                "Foxes hear mice under the snow and dive in head-first – the famous 'vole jump'.",
                "Young foxes spend about 13% of their time playing, with over 20 kinds of play.",
                "Foxes cache food by burying it for later.",
                "They live in small family groups; both parents care for the kits.",
            ],
            Species::Penguin => &[
                "Adélie penguins give each other stones as 'engagement rings'.",
                "Penguins preen each other and return the favor at once – it strengthens bonds.",
                "They play 'first in': shove each other off the ice edge into the water.",
                "Belly-sliding lets them race across ice at up to 60 km/h.",
            ],
            Species::Glimmi => &[
                "Glimmis play by chasing their own sparkle and reflections.",
                "When happy, a Glimmi's sparks dance brighter than usual.",
            ],
        }
    }

    pub fn fun_facts(self) -> &'static [&'static str] {
        match self {
            Species::Jellyfish => &[
                "Some jellyfish even eat other jellyfish.",
                "Nematocysts fire their stinging thread at 40,000 times the acceleration of gravity.",
                "The Portuguese man o' war can kill a human in about 5 minutes.",
            ],
            Species::Frog => &[
                "Frog skin contains natural antibiotic peptides.",
                "The largest frog (Goliath) has a body about 30 cm long.",
                "Frogs are cold-blooded: cold makes them sluggish, warmth makes them lively.",
            ],
            Species::Fox => &[
                "The fennec fox uses its huge ears to dump heat in the desert.",
                "Foxes are the only canids that can partially retract their claws.",
                "Kits open their eyes after about 2 weeks and leave the den at 3–4 weeks.",
            ],
            Species::Penguin => &[
                "Emperor penguins are the only penguins that breed in the Antarctic winter.",
                "Their black-and-white plumage is camouflage: dark back blends with the deep, white belly with the bright sky.",
                "King penguins can swallow up to 2,000 fish in a single day.",
            ],
            Species::Glimmi => &[
                "Glimmis rearrange their sparks to stay clean – their own kind of preening.",
                "A happy Glimmi can light up a whole corner of the terminal.",
            ],
        }
    }

    /// Pick a random fact from biology / behavior / fun.
    pub fn random_fact(self, rng: &mut Rng) -> String {
        let pool = match rng.range(3) {
            0 => self.biology_facts(),
            1 => self.behavior_facts(),
            _ => self.fun_facts(),
        };
        pool[rng.range(pool.len())].to_string()
    }

    /// Species-specific disease name + suggested cure (for medicine flavor text).
    pub fn disease_and_cure(self) -> (&'static str, &'static str) {
        match self {
            Species::Jellyfish => ("a bell lesion", "iodine baths and better water quality"),
            Species::Frog => ("a ranavirus infection", "warmth (26–30 °C) and electrolyte baths"),
            Species::Fox => ("sarcoptic mange", "ivermectin"),
            Species::Penguin => ("aspergillosis", "itraconazole – carefully dosed"),
            Species::Glimmi => ("a spark jam", "a little moonlight"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Egg,
    Baby,
    Adult,
}

impl Stage {
    pub fn label(self) -> &'static str {
        match self {
            Stage::Egg => "Egg",
            Stage::Baby => "Baby",
            Stage::Adult => "Adult",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mood {
    Egg,
    Sleeping,
    Sick,
    Hungry,
    Tired,
    Grumpy,
    Sad,
    Lonely,
    Neutral,
    Happy,
    Ecstatic,
}

impl Mood {
    pub fn label(self) -> &'static str {
        match self {
            Mood::Egg => "still in the egg",
            Mood::Sleeping => "asleep",
            Mood::Sick => "sick",
            Mood::Hungry => "hungry",
            Mood::Tired => "tired",
            Mood::Grumpy => "grumpy",
            Mood::Sad => "sad",
            Mood::Lonely => "lonely",
            Mood::Neutral => "relaxed",
            Mood::Happy => "happy",
            Mood::Ecstatic => "ecstatic",
        }
    }
}

/// Short-lived reaction to an action (briefly overrides the face).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Reaction {
    None,
    Eating,
    Playing,
    Loved,
    Washing,
    Medicine,
    Listening, // rubber-duck mode: the pet listens to you
    Eureka,    // you found the bug!
}

impl Reaction {
    pub fn label(self) -> &'static str {
        match self {
            Reaction::None => "-",
            Reaction::Eating => "eating",
            Reaction::Playing => "playing",
            Reaction::Loved => "loved",
            Reaction::Washing => "washing",
            Reaction::Medicine => "medicine",
            Reaction::Listening => "listening (duck)",
            Reaction::Eureka => "eureka",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_trip_and_legacy_keys_still_load() {
        for sp in Species::ALL {
            assert!(Species::from_key(sp.key()) == Some(sp));
            assert!(Species::from_key(sp.legacy_key()) == Some(sp));
        }
        // saves written by the German-language version
        assert!(Species::from_key("qualle") == Some(Species::Jellyfish));
        assert!(Species::from_key("frosch") == Some(Species::Frog));
        assert!(Species::from_key("fuchs") == Some(Species::Fox));
        assert!(Species::from_key("pinguin") == Some(Species::Penguin));
        assert!(Species::from_key("dragon").is_none());
    }

    #[test]
    fn every_species_has_complete_texts_and_facts() {
        for sp in Species::ALL {
            assert!(!sp.label().is_empty());
            assert!(!sp.blurb().is_empty());
            assert!(sp.sound().starts_with('*') && sp.sound().ends_with('*'));
            assert!(!sp.food().is_empty());
            assert!(sp.dream().starts_with("Zzz"));
            assert!(sp.quirks().iter().all(|q| !q.is_empty()));
            assert!(!sp.biology_facts().is_empty());
            assert!(!sp.behavior_facts().is_empty());
            assert!(!sp.fun_facts().is_empty());
            let (disease, cure) = sp.disease_and_cure();
            assert!(!disease.is_empty() && !cure.is_empty());
            let (hunger, boredom, tiredness, dirt) = sp.rates();
            assert!(hunger > 0.0 && boredom > 0.0 && tiredness > 0.0 && dirt > 0.0);
        }
    }

    #[test]
    fn random_fact_always_returns_text() {
        let mut rng = Rng::new();
        for sp in Species::ALL {
            for _ in 0..30 {
                assert!(!sp.random_fact(&mut rng).trim().is_empty());
            }
        }
    }

    #[test]
    fn labels_are_unique() {
        for (i, a) in Species::ALL.iter().enumerate() {
            for b in Species::ALL.iter().skip(i + 1) {
                assert!(a.label() != b.label() && a.key() != b.key());
            }
        }
    }
}
