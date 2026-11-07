//! Commands, main game loop, gallery and pet creation.

use std::io::{self, Write};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use crate::pet::{is_leave_word, Pet, BABY_SECS, EGG_SECS};
use crate::pixel::{ANSI_ROWS, ASCII_W, SCREEN_W, canvas_lines, render_scene};
use crate::rng::Rng;
use crate::species::{Mood, Reaction, Species, Stage};
use crate::sprites::render_pet;
use crate::term::{BOLD, RESET, ascii, fx, set_ascii, term_size, to_ascii};
use crate::ui::{build_screen, build_screen_ansi, draw, draw_plain};

// ───────────────────────── Commands & game loop ─────────────────────────

pub enum Command {
    Msg(String),
    Talk,
    Fact,
    Mode(Option<bool>),
    Duck(Option<bool>),
    Quit,
    Reset,
    Nothing,
}

pub enum Exit {
    Quit,
    Reset,
}

pub fn handle_command(line: &str, pet: &mut Pet, rng: &mut Rng) -> Command {
    let cmd = line.trim().to_lowercase();
    match cmd.as_str() {
        "" => Command::Nothing,
        "f" | "feed" | "eat" => Command::Msg(pet.feed(rng)),
        "p" | "play" => Command::Msg(pet.play()),
        "s" | "pet" | "cuddle" => Command::Msg(pet.pet_it()),
        "c" | "w" | "wash" | "bath" | "clean" => Command::Msg(pet.wash()),
        "z" | "sleep" | "wake" => Command::Msg(pet.toggle_sleep()),
        "m" | "medicine" | "heal" => Command::Msg(pet.medicine()),
        "t" | "talk" | "speak" => Command::Talk,
        "i" | "info" | "fact" => Command::Fact,
        "?" | "h" | "help" => Command::Msg(
            "d = rubber duck: explain your code to your pet. i = fact. 'reset' = new pet.".to_string(),
        ),
        "d" | "duck" | "explain" | "rubber" => Command::Duck(None),
        "done" | "end" | "stop" => Command::Duck(Some(false)),
        "a" | "mode" => Command::Mode(None),
        "ascii" => Command::Mode(Some(true)),
        "ansi" => Command::Mode(Some(false)),
        "q" | "quit" | "exit" => Command::Quit,
        "reset" => Command::Msg("Really start over? Type 'reset yes' to confirm.".to_string()),
        "reset yes" => Command::Reset,
        _ => {
            let short: String = cmd.chars().take(20).collect();
            Command::Msg(format!("Unknown command '{}'. Type '?' for help.", short))
        }
    }
}

/// What the game says when the pet reaches a new life stage.
fn stage_message(pet: &Pet, stage: Stage) -> String {
    match stage {
        Stage::Baby => format!("The egg hatched! Welcome, {} the {}!", pet.name, pet.species.label()),
        Stage::Adult => format!("{} has grown up! Congratulations!", pet.name),
        Stage::Egg => String::new(),
    }
}

pub fn game_loop(pet: &mut Pet, rx: &Receiver<String>, rng: &mut Rng, welcome: String) -> Exit {
    let mut message = welcome;
    let mut thought = pet.speech(rng);
    let mut thought_timer = 10.0_f64;
    let mut autosave = 0.0_f64;
    let mut ticks: u64 = 0;
    let mut last = Instant::now();
    let start = Instant::now();
    let mut full = true;
    let mut dirty = true;
    let mut duck = false; // rubber-duck mode: everything you type is an explanation
    let mut duck_lines: u32 = 0;
    let mut term = term_size(); // (columns, rows) - used to center the UI
    let mut size_timer = 0.0_f64;

    if !ascii() {
        print!("\x1b[2J");
    }

    loop {
        // 1) Handle input
        loop {
            match rx.try_recv() {
                Ok(line) => {
                    full = true;
                    dirty = true;
                    let stage_before = pet.stage();
                    let trimmed = line.trim().to_string();
                    // In duck mode every line is an explanation, except lines starting with '/'.
                    let command: Option<String> = if duck {
                        // plain words like "done" or "exit" leave the duck mode as well
                        if is_leave_word(&trimmed.to_lowercase()) {
                            Some("done".to_string())
                        } else {
                            trimmed.strip_prefix('/').map(|r| r.to_string())
                        }
                    } else {
                        Some(trimmed.clone())
                    };
                    match command {
                        None => {
                            // you are explaining something to your pet
                            if trimmed.is_empty() {
                                thought = "Take your time. I'm listening.".to_string();
                                thought_timer = 20.0;
                            } else {
                                duck_lines += 1;
                                let (reply, eureka) = pet.duck_listen(&trimmed, rng, duck_lines);
                                thought = reply;
                                thought_timer = 25.0;
                                message = if eureka {
                                    format!("Eureka! {} helped you debug (just by listening).", pet.name)
                                } else {
                                    format!("{} listens... ({} line(s) explained)", pet.name, duck_lines)
                                };
                            }
                        }
                        Some(cmd) => match handle_command(&cmd, pet, rng) {
                            Command::Msg(m) => {
                                message = m;
                                if let Some(s) = pet.reaction_speech(rng) {
                                    thought = s;
                                    thought_timer = 10.0;
                                }
                            }
                            Command::Talk => {
                                pet.idle = 0.0;
                                if pet.stage() != Stage::Egg && !pet.asleep {
                                    pet.joy = (pet.joy + 2.0).min(100.0);
                                    pet.care += 0.2;
                                }
                                thought = pet.speech(rng);
                                thought_timer = 14.0;
                                message = format!("You chat with {}.", pet.name);
                            }
                            Command::Fact => {
                                pet.idle = 0.0;
                                let fact = pet.species.random_fact(rng);
                                thought = format!("{} {}", pet.species.sound(), fact);
                                thought_timer = 20.0;
                                message = format!("{} tells you something about itself.", pet.name);
                            }
                            Command::Mode(m) => {
                                let want_ascii = m.unwrap_or(!ascii());
                                set_ascii(want_ascii);
                                if want_ascii {
                                    message = "Display: ASCII (plain text, Enter = refresh)".to_string();
                                } else {
                                    message = "Display: ANSI (colors & live animation)".to_string();
                                    print!("\x1b[2J");
                                }
                            }
                            Command::Duck(m) => {
                                let want = m.unwrap_or(!duck);
                                if want && !duck {
                                    if pet.stage() == Stage::Egg {
                                        message = "The egg is too sleepy to listen. Wait until it hatches!".to_string();
                                    } else {
                                        duck = true;
                                        duck_lines = 0;
                                        pet.asleep = false;
                                        pet.idle = 0.0;
                                        message = format!("Duck mode! Explain your problem to {}. /done = finish.", pet.name);
                                        thought = "I'm all ears. Start from the beginning!".to_string();
                                        thought_timer = 25.0;
                                    }
                                } else if !want && duck {
                                    duck = false;
                                    if pet.reaction == Reaction::Listening {
                                        pet.reaction = Reaction::None;
                                    }
                                    message = format!("Duck mode off. {} listened to {} line(s).", pet.name, duck_lines);
                                    thought = "That was fun! Come back anytime.".to_string();
                                    thought_timer = 15.0;
                                    if duck_lines >= 3 {
                                        pet.care += 1.0;
                                        pet.joy = (pet.joy + 8.0).min(100.0);
                                    }
                                } else if want {
                                    message = "Already in duck mode. Just keep talking!".to_string();
                                } else {
                                    message = "Not in duck mode. Start it with 'd'.".to_string();
                                }
                            }
                            Command::Quit => {
                                pet.save();
                                return Exit::Quit;
                            }
                            Command::Reset => return Exit::Reset,
                            Command::Nothing => {}
                        },
                    }
                    // Petting a warm egg can make it hatch right here, between two time ticks.
                    let stage_now = pet.stage();
                    if stage_now != stage_before {
                        message = stage_message(pet, stage_now);
                        thought = pet.speech(rng);
                        thought_timer = 12.0;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    pet.save();
                    return Exit::Quit;
                }
            }
        }

        // 2) Time passes
        let now = Instant::now();
        let dt = now.duration_since(last).as_secs_f64().min(5.0);
        last = now;

        if let Some(stage) = pet.tick(dt) {
            message = stage_message(pet, stage);
            thought = pet.speech(rng);
            thought_timer = 12.0;
            dirty = true;
        }

        // in duck mode the pet keeps listening (and counts it as attention)
        if duck {
            pet.idle = 0.0;
            if pet.reaction == Reaction::None && !pet.asleep && pet.stage() != Stage::Egg {
                pet.reaction = Reaction::Listening;
                pet.reaction_left = 5.0;
            }
        }

        pet.duck = duck; // lets the screen show how to leave

        thought_timer -= dt;
        if thought_timer <= 0.0 {
            thought = if duck {
                rng.pick(&[
                    "Mhm... I'm listening.",
                    "Take your time.",
                    "Go on, I'm all ears.",
                    "...and then what happened?",
                ])
                .to_string()
            } else {
                pet.speech(rng)
            };
            thought_timer = 10.0 + rng.range(10) as f64;
        }

        autosave += dt;
        if autosave > 10.0 {
            pet.save();
            autosave = 0.0;
        }

        // follow terminal resizes (keeps the UI centered)
        size_timer += dt;
        if size_timer > 1.5 {
            size_timer = 0.0;
            let now_size = term_size();
            if now_size != term {
                term = now_size;
                if !ascii() {
                    print!("\x1b[2J");
                }
                full = true;
                dirty = true;
            }
        }

        // 3) Draw (centered)
        let (cols, rows) = term;
        let prompt = if duck { "explain> " } else { "> " };
        if ascii() {
            if dirty {
                let screen = build_screen(pet, ticks, &thought, &message);
                draw_plain(&screen, cols.saturating_sub(ASCII_W) / 2, prompt);
                dirty = false;
            }
        } else {
            let t = start.elapsed().as_secs_f32();
            let screen = build_screen_ansi(pet, t, &thought, &message);
            let col_off = cols.saturating_sub(SCREEN_W) / 2;
            let row_off = rows.saturating_sub(ANSI_ROWS + 2) / 2;
            draw(&screen, full, col_off, row_off, prompt);
        }
        full = false;
        ticks += 1;
        thread::sleep(Duration::from_millis(if ascii() { 250 } else { 120 }));
    }
}

// ───────────────────────── Gallery / self-test: every pet in every situation ─────────────────────────

/// One situation: a species at a life stage, in a mood, maybe reacting to something.
#[derive(Clone, Copy)]
pub struct Situation {
    species: Species,
    stage: Stage,
    mood: Mood,
    reaction: Reaction,
    progress: f32, // hatching progress, only used for eggs
}

impl Situation {
    pub fn caption(&self) -> String {
        if self.stage == Stage::Egg {
            format!(
                "{} / Egg / hatching {}%",
                self.species.label(),
                (self.progress * 100.0).round() as i32
            )
        } else {
            format!(
                "{} / {} / {} / {}",
                self.species.label(),
                self.stage.label(),
                self.mood.label(),
                self.reaction.label()
            )
        }
    }
}

/// Every species x (3 egg stages + 2 life stages x (10 moods + sleeping-and-loved + 7 reactions)),
/// i.e. 39 situations per species.
/// `filter` empty = all species. Otherwise only the listed ones (order preserved).
pub fn all_situations(filter: &[Species]) -> Vec<Situation> {
    let moods = [
        Mood::Sleeping,
        Mood::Sick,
        Mood::Hungry,
        Mood::Tired,
        Mood::Grumpy,
        Mood::Sad,
        Mood::Lonely,
        Mood::Neutral,
        Mood::Happy,
        Mood::Ecstatic,
    ];
    let reactions = [
        Reaction::Eating,
        Reaction::Playing,
        Reaction::Loved,
        Reaction::Washing,
        Reaction::Medicine,
        Reaction::Listening,
        Reaction::Eureka,
    ];
    let species_list: Vec<Species> = if filter.is_empty() {
        Species::ALL.to_vec()
    } else {
        filter.to_vec()
    };
    let mut out: Vec<Situation> = Vec::new();
    for sp in species_list {
        for prog in [0.1_f32, 0.6, 0.95] {
            out.push(Situation { species: sp, stage: Stage::Egg, mood: Mood::Egg, reaction: Reaction::None, progress: prog });
        }
        for stage in [Stage::Baby, Stage::Adult] {
            for m in moods {
                out.push(Situation { species: sp, stage, mood: m, reaction: Reaction::None, progress: 1.0 });
            }
            out.push(Situation { species: sp, stage, mood: Mood::Sleeping, reaction: Reaction::Loved, progress: 1.0 });
            for r in reactions {
                out.push(Situation { species: sp, stage, mood: Mood::Happy, reaction: r, progress: 1.0 });
            }
        }
    }
    out
}

/// Builds a pet whose stats produce exactly the requested stage, mood and reaction.
pub fn pet_for(sit: &Situation) -> Pet {
    let mut p = Pet::new_egg("Gallery".to_string(), sit.species);
    p.age = match sit.stage {
        Stage::Egg => sit.progress as f64 * EGG_SECS,
        Stage::Baby => EGG_SECS + 60.0,
        Stage::Adult => EGG_SECS + BABY_SECS + 60.0,
    };
    p.food = 70.0;
    p.joy = 70.0;
    p.energy = 70.0;
    p.clean = 70.0;
    p.health = 100.0;
    p.idle = 0.0;
    match sit.mood {
        Mood::Sleeping => p.asleep = true,
        Mood::Sick => p.health = 10.0,
        Mood::Hungry => p.food = 10.0,
        Mood::Tired => p.energy = 10.0,
        Mood::Grumpy => p.clean = 10.0,
        Mood::Sad => p.joy = 10.0,
        Mood::Lonely => {
            p.joy = 40.0;
            p.idle = 300.0;
        }
        Mood::Neutral => p.joy = 45.0,
        Mood::Happy => p.joy = 70.0,
        Mood::Ecstatic => {
            p.joy = 95.0;
            p.food = 90.0;
            p.energy = 90.0;
            p.clean = 90.0;
        }
        Mood::Egg => {}
    }
    p.reaction = sit.reaction;
    p.reaction_left = 1.0e6;
    p
}

/// Prints every pet in every situation once (for eyeballing / tests).
/// Honors the display mode: pixel art in ANSI mode, text sprites with --ascii.
/// Tip: `terminal-pet --gallery | less -R`, or `--gallery penguin`, or `--gallery --pause`.
pub fn run_gallery(filter: &[Species], pause: bool) {
    let list = all_situations(filter);
    let total = list.len();
    let mut last: Option<Species> = None;
    for (i, sit) in list.iter().enumerate() {
        if pause && last.is_some() && last != Some(sit.species) {
            print!("-- press Enter for the next species --");
            let _ = io::stdout().flush();
            let mut dummy = String::new();
            let _ = io::stdin().read_line(&mut dummy);
        }
        last = Some(sit.species);
        let pet = pet_for(sit);
        let t = 1.0 + i as f32 * 0.37;
        println!(
            "{}",
            fx(format!("{}== {}/{}  {} =={}", BOLD, i + 1, total, sit.caption(), RESET))
        );
        if ascii() {
            for l in render_pet(&pet, (t * 4.0) as u64) {
                let plain = to_ascii(&l);
                if !plain.trim().is_empty() {
                    println!("{}", plain.trim_end());
                }
            }
        } else {
            let scene = render_scene(sit.species, sit.stage, sit.mood, sit.reaction, t, sit.progress);
            for l in canvas_lines(&scene) {
                println!("{}", l);
            }
        }
        println!();
    }
    println!("{} situations shown.", total);
}

// ───────────────────────── Start ─────────────────────────

pub fn ask(rx: &Receiver<String>, prompt: &str) -> Option<String> {
    print!("{}", prompt);
    let _ = io::stdout().flush();
    rx.recv().ok().map(|s| s.trim().to_string())
}

pub fn create_pet(rx: &Receiver<String>) -> Option<Pet> {
    // center the welcome screen as well
    let (cols, rows) = term_size();
    let pad = " ".repeat(cols.saturating_sub(72) / 2);
    let top = rows.saturating_sub(13) / 2;
    if !ascii() {
        print!("\x1b[2J\x1b[H");
    }
    print!("{}", "\n".repeat(top));
    println!("{}{}", pad, fx(format!("{}{}  ~ TERMINAL PET ~{}", BOLD, "\x1b[96m", RESET)));
    println!();
    println!("{}  Welcome! Which creature should hatch for you?", pad);
    println!();
    for (i, s) in Species::ALL.iter().enumerate() {
        println!(
            "{}{}",
            pad,
            fx(format!(
                "   {}{}) {:<9}{} {}",
                s.color(),
                i + 1,
                s.label(),
                RESET,
                s.blurb()
            ))
        );
    }
    println!();

    let species = loop {
        let input = ask(rx, &format!("{}  Your choice (1-{}): ", pad, Species::ALL.len()))?.to_lowercase();
        if let Ok(n) = input.parse::<usize>() {
            if (1..=Species::ALL.len()).contains(&n) {
                break Species::ALL[n - 1];
            }
        }
        if let Some(s) = Species::from_key(&input) {
            break s;
        }
        println!("{}  Please enter a number from 1 to {}.", pad, Species::ALL.len());
    };

    let raw = ask(rx, &format!("{}  What should your {} be called? ", pad, species.label()))?;
    let cleaned: String = raw.chars().filter(|c| !c.is_control()).take(14).collect();
    let mut name = cleaned.trim().to_string();
    if name.is_empty() {
        name = species.label().to_string();
    }
    Some(Pet::new_egg(name, species))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pixel::PH;
    use crate::term::{SPRITE_ROWS, UI_ROWS};

    fn adult(sp: Species) -> Pet {
        let mut p = Pet::new_egg("Test".to_string(), sp);
        p.age = EGG_SECS + BABY_SECS + 1.0;
        p
    }

    // ── gallery ──

    #[test]
    fn gallery_has_39_situations_per_species() {
        assert_eq!(all_situations(&[]).len(), Species::ALL.len() * 39);
        for sp in Species::ALL {
            let list = all_situations(&[sp]);
            assert_eq!(list.len(), 39);
            assert!(list.iter().all(|s| s.species == sp));
        }
        let two = all_situations(&[Species::Fox, Species::Penguin]);
        assert_eq!(two.len(), 78);
        assert!(two[0].species == Species::Fox && two[77].species == Species::Penguin);
    }

    #[test]
    fn gallery_covers_every_stage_mood_and_reaction() {
        let list = all_situations(&[Species::Frog]);
        for stage in [Stage::Egg, Stage::Baby, Stage::Adult] {
            assert!(list.iter().any(|s| s.stage == stage));
        }
        for mood in [
            Mood::Egg, Mood::Sleeping, Mood::Sick, Mood::Hungry, Mood::Tired, Mood::Grumpy,
            Mood::Sad, Mood::Lonely, Mood::Neutral, Mood::Happy, Mood::Ecstatic,
        ] {
            assert!(list.iter().any(|s| s.mood == mood), "missing mood {}", mood.label());
        }
        for reaction in [
            Reaction::None, Reaction::Eating, Reaction::Playing, Reaction::Loved, Reaction::Washing,
            Reaction::Medicine, Reaction::Listening, Reaction::Eureka,
        ] {
            assert!(list.iter().any(|s| s.reaction == reaction), "missing reaction {}", reaction.label());
        }
    }

    #[test]
    fn captions_describe_the_situation() {
        let list = all_situations(&[Species::Penguin]);
        assert_eq!(list[0].caption(), "Penguin / Egg / hatching 10%");
        assert_eq!(list[3].caption(), "Penguin / Baby / asleep / -");
        assert_eq!(list[38].caption(), "Penguin / Adult / happy / eureka");
    }

    #[test]
    fn every_situation_builds_a_pet_with_matching_stage_mood_and_reaction() {
        for sit in all_situations(&[]) {
            let pet = pet_for(&sit);
            assert!(pet.species == sit.species, "{}", sit.caption());
            assert!(pet.stage() == sit.stage, "stage: {}", sit.caption());
            assert!(pet.mood() == sit.mood, "mood: {}", sit.caption());
            assert!(pet.reaction == sit.reaction, "reaction: {}", sit.caption());
        }
    }

    #[test]
    fn every_situation_renders_in_both_display_modes() {
        for sit in all_situations(&[]) {
            let pet = pet_for(&sit);
            // different animation phases
            for t in [0.0_f32, 0.37, 1.7, 9.99] {
                // pixel art
                let scene = render_scene(sit.species, sit.stage, sit.mood, sit.reaction, t, sit.progress);
                assert_eq!(canvas_lines(&scene).len(), PH / 2, "{}", sit.caption());
                // full ANSI screen
                let screen = build_screen_ansi(&pet, t, "I feel great!", "message");
                assert_eq!(screen.len(), ANSI_ROWS, "{}", sit.caption());
            }
            // ASCII sprite and full ASCII screen
            for ticks in [0_u64, 3, 22, 23] {
                let rows = render_pet(&pet, ticks);
                assert_eq!(rows.len(), SPRITE_ROWS + 1, "{}", sit.caption());
                assert!(
                    rows[1..].iter().any(|r| !to_ascii(r).trim().is_empty()),
                    "empty sprite: {}",
                    sit.caption()
                );
                assert_eq!(build_screen(&pet, ticks, "I feel great!", "message").len(), UI_ROWS);
            }
        }
    }

    #[test]
    fn pixel_scenes_differ_between_species_and_moods() {
        let a = canvas_lines(&render_scene(Species::Penguin, Stage::Adult, Mood::Happy, Reaction::None, 1.0, 1.0));
        let b = canvas_lines(&render_scene(Species::Fox, Stage::Adult, Mood::Happy, Reaction::None, 1.0, 1.0));
        let c = canvas_lines(&render_scene(Species::Penguin, Stage::Adult, Mood::Sad, Reaction::None, 1.0, 1.0));
        assert!(a != b, "each species has its own scene");
        assert!(a != c, "the mood changes the picture");
    }

    // ── commands ──

    #[test]
    fn life_stage_changes_are_announced() {
        let p = adult(Species::Penguin);
        assert!(stage_message(&p, Stage::Baby).contains("hatched"));
        assert!(stage_message(&p, Stage::Baby).contains("Test the Penguin"));
        assert!(stage_message(&p, Stage::Adult).contains("grown up"));
        assert!(stage_message(&p, Stage::Egg).is_empty());
    }

    #[test]
    fn command_aliases() {
        let mut rng = Rng::new();
        let mut pet = adult(Species::Frog);

        for alias in ["f", "feed", "eat", "  F  "] {
            pet.food = 10.0;
            assert!(matches!(handle_command(alias, &mut pet, &mut rng), Command::Msg(_)));
            assert!(pet.food > 10.0, "'{alias}' should feed");
        }
        for alias in ["p", "play"] {
            pet.joy = 10.0;
            handle_command(alias, &mut pet, &mut rng);
            assert!(pet.joy > 10.0, "'{alias}' should play");
        }
        for alias in ["s", "pet", "cuddle"] {
            pet.joy = 10.0;
            handle_command(alias, &mut pet, &mut rng);
            assert!(pet.joy > 10.0, "'{alias}' should pet");
        }
        for alias in ["c", "w", "wash", "bath", "clean"] {
            pet.clean = 10.0;
            handle_command(alias, &mut pet, &mut rng);
            assert!(pet.clean == 100.0, "'{alias}' should wash");
        }
        for alias in ["m", "medicine", "heal"] {
            pet.health = 10.0;
            handle_command(alias, &mut pet, &mut rng);
            assert!(pet.health > 10.0, "'{alias}' should heal");
        }
        for alias in ["z", "sleep"] {
            pet.asleep = false;
            pet.energy = 30.0;
            handle_command(alias, &mut pet, &mut rng);
            assert!(pet.asleep, "'{alias}' should put the pet to bed");
        }
        handle_command("wake", &mut pet, &mut rng);
        assert!(!pet.asleep);

        for alias in ["t", "talk", "speak"] {
            assert!(matches!(handle_command(alias, &mut pet, &mut rng), Command::Talk));
        }
        for alias in ["i", "info", "fact"] {
            assert!(matches!(handle_command(alias, &mut pet, &mut rng), Command::Fact));
        }
        for alias in ["d", "duck", "explain", "rubber"] {
            assert!(matches!(handle_command(alias, &mut pet, &mut rng), Command::Duck(None)));
        }
        for alias in ["done", "end", "stop"] {
            assert!(matches!(handle_command(alias, &mut pet, &mut rng), Command::Duck(Some(false))));
        }
        for alias in ["a", "mode"] {
            assert!(matches!(handle_command(alias, &mut pet, &mut rng), Command::Mode(None)));
        }
        assert!(matches!(handle_command("ascii", &mut pet, &mut rng), Command::Mode(Some(true))));
        assert!(matches!(handle_command("ansi", &mut pet, &mut rng), Command::Mode(Some(false))));
        for alias in ["q", "quit", "exit"] {
            assert!(matches!(handle_command(alias, &mut pet, &mut rng), Command::Quit));
        }
        assert!(matches!(handle_command("", &mut pet, &mut rng), Command::Nothing));
        assert!(matches!(handle_command("   ", &mut pet, &mut rng), Command::Nothing));
    }

    #[test]
    fn help_and_unknown_commands_answer_with_a_message() {
        let mut rng = Rng::new();
        let mut pet = adult(Species::Fox);
        for alias in ["?", "h", "help"] {
            match handle_command(alias, &mut pet, &mut rng) {
                Command::Msg(m) => assert!(m.contains("rubber duck")),
                _ => panic!("'{alias}' should print the help"),
            }
        }
        match handle_command("xyzzy", &mut pet, &mut rng) {
            Command::Msg(m) => assert!(m.contains("Unknown command 'xyzzy'")),
            _ => panic!("unknown commands should answer with a message"),
        }
        match handle_command("this is a very long unknown command", &mut pet, &mut rng) {
            Command::Msg(m) => assert!(m.contains("'this is a very long '") || m.contains("'this is a very long")),
            _ => panic!(),
        }
    }

    #[test]
    fn reset_needs_a_confirmation() {
        let mut rng = Rng::new();
        let mut pet = adult(Species::Glimmi);
        match handle_command("reset", &mut pet, &mut rng) {
            Command::Msg(m) => assert!(m.contains("reset yes")),
            _ => panic!("'reset' alone must only ask"),
        }
        assert!(matches!(handle_command("RESET YES", &mut pet, &mut rng), Command::Reset));
    }

    // ── start ──

    #[test]
    fn create_pet_accepts_numbers_and_names_and_cleans_the_name() {
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        // 9 is invalid and asked again; the species can also be typed; control characters
        // are dropped and the name is cut to 14 characters
        for line in ["9", "penguin", "Sir Waddles-the-Magnificent \u{7}"] {
            tx.send(line.to_string()).unwrap();
        }
        let pet = create_pet(&rx).expect("a pet");
        assert!(pet.species == Species::Penguin);
        assert_eq!(pet.name, "Sir Waddles-th");
        assert!(pet.stage() == Stage::Egg);

        // choice by number, empty name = species name
        for line in ["2", ""] {
            tx.send(line.to_string()).unwrap();
        }
        let pet = create_pet(&rx).expect("a pet");
        assert!(pet.species == Species::Frog);
        assert_eq!(pet.name, "Frog");
    }

    #[test]
    fn create_pet_gives_up_when_the_input_closes() {
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        drop(tx);
        assert!(create_pet(&rx).is_none());
    }
}
