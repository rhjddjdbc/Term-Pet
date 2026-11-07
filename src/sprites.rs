//! ASCII sprite templates and text-mode pet rendering.

use crate::pet::Pet;
use crate::species::{Mood, Reaction, Species, Stage};
use crate::term::{DIM, MARGIN, RAINBOW, RESET, SPRITE_ROWS};

// ───────────────────────── Sprites ─────────────────────────
// Placeholders in the templates:  @ = left eye   # = right eye   % = mouth
// (So these three characters must not appear anywhere else in the templates.)

pub const EGG_0: &str = r"
       _..._
     .'     '.
    /  .   .  \
   |    . .    |
   |           |
    \         /
     '-.___.-'
";

pub const EGG_1: &str = r"
       _..._
     .'     '.
    /  .   .  \
   |   /\  /\  |
   |  /  \/  \ |
    \         /
     '-.___.-'
";

pub const EGG_2: &str = r"
       _/\/\_
     .'/    \'.
    / /  ..  \ \
   |  \ \  / /  |
   |   \ \/ /   |
    \   \  /   /
     '-.__\/_.-'
";

pub const JELLY_BABY_A: &str = r"
    .--~~--.
   ( @    # )
   (   %    )
    '------'
     ) ( ) (
";

pub const JELLY_BABY_B: &str = r"
    .--~~--.
   ( @    # )
   (   %    )
    '------'
     ( ) ( )
";

pub const JELLY_ADULT_A: &str = r"
      .-~~~~~~-.
    .'          '.
   /    @    #    \
  |       %        |
   \              /
    '-..______..-'
     ) ( ) ( ) (
     ( ) ( ) ( )
";

pub const JELLY_ADULT_B: &str = r"
      .-~~~~~~-.
    .'          '.
   /    @    #    \
  |       %        |
   \              /
    '-..______..-'
     ( ) ( ) ( )
     ) ( ) ( ) (
";

pub const FROG_BABY_A: &str = r"
   .-.   .-.
  ( @ )_( # )
  '-.-'''-.-'
   (   %   )
   /_|   |_\
";

pub const FROG_BABY_B: &str = r"
   .-.   .-.
  ( @ )_( # )
  '-.-'''-.-'
   ( ( % ) )
   _/|   |\_
";

pub const FROG_ADULT_A: &str = r"
    .-.       .-.
   ( @ )     ( # )
  .'-'-._____.-'-'.
 /                 \
 |        %        |
 \   .---------.   /
  '-/           \-'
   /_|_|     |_|_\
";

pub const FROG_ADULT_B: &str = r"
    .-.       .-.
   ( @ )     ( # )
  .'-'-._____.-'-'.
 /                 \
 |     (  %  )     |
 \   .---------.   /
  '-/           \-'
    /_|_|   |_|_\
";

pub const FOX_BABY_A: &str = r"
   /\     /\
  /  \___/  \
 (  @     #  )
  \    %    /
   '-(___)-'__,
";

pub const FOX_BABY_B: &str = r"
   /\     /\
  /  \___/  \
 (  @     #  )
  \    %    /
   '-(___)-'_,'
";

pub const FOX_ADULT_A: &str = r"
   /\         /\
  /  \_______/  \
 /    @     #    \
 \       %       /
  '-.   ___   .-'
    /  '---'  \     ,--.
   (  |     |  )---'  _\ \
    '-'-----'-'-----`--'
";

pub const FOX_ADULT_B: &str = r"
   /\         /\
  /  \_______/  \
 /    @     #    \
 \       %       /
  '-.   ___   .-'
    /  '---'  \    ,--.
   (  |     |  )--'  _/ /
    '-'-----'-'----`--'
";

pub const GLIMMI_BABY_A: &str = r"
      *  .
       \ |
     .-~~~-.
    ( @   # )
    (   %   )
     '-...-'
";

pub const GLIMMI_BABY_B: &str = r"
      .  *
       \ |
     .-~~~-.
    ( @   # )
    (   %   )
     '-...-'
";

pub const GLIMMI_ADULT_A: &str = r"
        *   .   *
         \  |  /
      .-~~~~~~~~-.
     /  @      #  \
    |       %      |
     \    '--'    /
    ~~'-.______.-'~~
        /_/  \_\
";

pub const GLIMMI_ADULT_B: &str = r"
        .   *   .
         \  |  /
      .-~~~~~~~~-.
     /  @      #  \
    |       %      |
     \    '--'    /
    ~~'-.______.-'~~
        /_/  \_\
";

pub const PENGUIN_BABY_A: &str = r"
       .-.
     ( @ # )
     ( <%> )
    /'.___.'\
    |  | |  |
     \_____/
     /_\ /_\
";

pub const PENGUIN_BABY_B: &str = r"
       .-.
     ( @ # )
     ( <%> )
    \'.___.'/
    |  | |  |
     \_____/
     /_\ /_\
";

pub const PENGUIN_ADULT_A: &str = r"
      .---.
     / @ # \
     \ <%> /
    .-'---'-.
   / |     | \
   | |     | |
    \ \___/ /
    /_\   /_\
";

pub const PENGUIN_ADULT_B: &str = r"
      .---.
     / @ # \
     \ <%> /
    .-'---'-.
   \ |     | /
    \|     |/ 
    \ \___/ /
    /_\   /_\
";

pub fn egg_template(progress: f64) -> &'static str {
    if progress < 0.5 {
        EGG_0
    } else if progress < 0.85 {
        EGG_1
    } else {
        EGG_2
    }
}

pub fn template(species: Species, stage: Stage, alt: bool) -> &'static str {
    let pair: [&'static str; 2] = match (species, stage) {
        (_, Stage::Egg) => [EGG_0, EGG_0],
        (Species::Jellyfish, Stage::Baby) => [JELLY_BABY_A, JELLY_BABY_B],
        (Species::Jellyfish, Stage::Adult) => [JELLY_ADULT_A, JELLY_ADULT_B],
        (Species::Frog, Stage::Baby) => [FROG_BABY_A, FROG_BABY_B],
        (Species::Frog, Stage::Adult) => [FROG_ADULT_A, FROG_ADULT_B],
        (Species::Fox, Stage::Baby) => [FOX_BABY_A, FOX_BABY_B],
        (Species::Fox, Stage::Adult) => [FOX_ADULT_A, FOX_ADULT_B],
        (Species::Glimmi, Stage::Baby) => [GLIMMI_BABY_A, GLIMMI_BABY_B],
        (Species::Glimmi, Stage::Adult) => [GLIMMI_ADULT_A, GLIMMI_ADULT_B],
        (Species::Penguin, Stage::Baby) => [PENGUIN_BABY_A, PENGUIN_BABY_B],
        (Species::Penguin, Stage::Adult) => [PENGUIN_ADULT_A, PENGUIN_ADULT_B],
    };
    pair[alt as usize]
}

/// Facial expression: (left eye, right eye, mouth)
pub fn face(mood: Mood, reaction: Reaction, ticks: u64) -> (char, char, char) {
    let frame = (ticks / 2) % 2;
    let blink = ticks % 24 >= 22;

    if mood == Mood::Sleeping {
        return ('-', '-', if reaction == Reaction::Loved { 'u' } else { '.' });
    }
    match reaction {
        Reaction::Eating => return ('^', '^', if frame == 0 { 'o' } else { '-' }),
        Reaction::Playing => return ('*', '*', 'D'),
        Reaction::Loved => return ('^', '^', 'u'),
        Reaction::Washing => return ('>', '<', 'o'),
        Reaction::Medicine => return ('>', '<', '~'),
        Reaction::Listening => return ('o', 'o', if frame == 0 { '-' } else { 'o' }),
        Reaction::Eureka => return ('*', '*', 'D'),
        Reaction::None => {}
    }

    let open = if blink { '-' } else { 'o' };
    let sleepy = if frame == 0 { '-' } else { 'o' };
    match mood {
        Mood::Egg => ('o', 'o', '-'),
        Mood::Sleeping => ('-', '-', '.'),
        Mood::Sick => ('x', 'x', '~'),
        Mood::Hungry => (open, open, if frame == 0 { 'o' } else { 'O' }),
        Mood::Tired => (sleepy, sleepy, '.'),
        Mood::Grumpy => ('>', '<', '~'),
        Mood::Sad => ('T', 'T', 'n'),
        Mood::Lonely => (open, open, '_'),
        Mood::Neutral => (open, open, '-'),
        Mood::Happy => ('^', '^', 'u'),
        Mood::Ecstatic => ('*', '*', 'D'),
    }
}

/// Little symbols above the pet (Zzz, hearts, notes ...).
pub fn emote_line(mood: Mood, reaction: Reaction, ticks: u64) -> String {
    let f = ((ticks / 2) % 4) as usize;
    let frames: [&str; 4] = match reaction {
        Reaction::Eating => ["    nom nom", "   ~nom nom~", "    nom nom", "   ~nom nom~"],
        Reaction::Playing => ["  * ! *", "  ! * !", "  * ! *", "  ! * !"],
        Reaction::Loved => ["  ♥    ♥", "    ♥    ♥", "  ♥  ♥", "    ♥  ♥  ♥"],
        Reaction::Washing => ["  o  O  o", " O  o  O", "  o  O  o", " O  o  O"],
        Reaction::Medicine => ["  +  +", " +  +  +", "  +  +", " +  +  +"],
        Reaction::Listening => ["  .", "  . .", "  . . .", "  . ."],
        Reaction::Eureka => ["  *!*", " * ! * ", "  *!*", " * ! * "],
        Reaction::None => match mood {
            Mood::Sleeping => ["    z", "    z Z", "    z Z z", "      Z z"],
            Mood::Sick => ["  ~ ~ ~", " ~ ~ ~", "  ~ ~ ~", " ~ ~ ~"],
            Mood::Hungry => ["  ?", "  ? ?", "  ?", "  ? ? ?"],
            Mood::Tired => ["  . . .", "  . .", "  .", "  . ."],
            Mood::Grumpy => ["  grr!", "  #!?", "  grr!", "  !!!"],
            Mood::Sad | Mood::Lonely => ["  ,", "  , ,", "  ,", "  . ,"],
            Mood::Happy => ["  ♪", "    ♪", "  ♪  ♪", "    ♪"],
            Mood::Ecstatic => ["  * ♥ *", " * ♪ * ♥", "  ♥ * ♪", " ♪ * ♥ *"],
            _ => ["", "", "", ""],
        },
    };
    format!("{}{}", " ".repeat(MARGIN), frames[f])
}

pub fn pet_color(pet: &Pet, mood: Mood, ticks: u64) -> String {
    match mood {
        Mood::Sleeping => format!("{}{}", DIM, pet.species.color()),
        Mood::Sick => "\x1b[38;5;106m".to_string(),
        Mood::Sad | Mood::Lonely => "\x1b[94m".to_string(),
        Mood::Grumpy => "\x1b[91m".to_string(),
        Mood::Ecstatic if pet.species == Species::Glimmi => {
            RAINBOW[(ticks as usize) % RAINBOW.len()].to_string()
        }
        _ => pet.species.color().to_string(),
    }
}

/// Returns exactly SPRITE_ROWS + 1 rows (1 emote row + sprite area).
pub fn render_pet(pet: &Pet, ticks: u64) -> Vec<String> {
    let mood = pet.mood();
    let stage = pet.stage();
    let alt = (ticks / 2) % 2 == 1;
    let (el, er, m) = face(mood, pet.reaction, ticks);
    let tpl = if stage == Stage::Egg {
        egg_template(pet.egg_progress())
    } else {
        template(pet.species, stage, alt)
    };

    let mut rows: Vec<String> = tpl
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            l.replace('@', &el.to_string())
                .replace('#', &er.to_string())
                .replace('%', &m.to_string())
        })
        .collect();

    // The egg wiggles shortly before hatching.
    if stage == Stage::Egg && pet.egg_progress() > 0.5 && alt {
        for r in rows.iter_mut() {
            r.insert(0, ' ');
        }
    }

    // Jellyfish and Glimmi bob up and down.
    let floats = stage != Stage::Egg && matches!(pet.species, Species::Jellyfish | Species::Glimmi);
    let bob = if floats && alt { 1 } else { 0 };

    let color = pet_color(pet, mood, ticks);
    let mut out: Vec<String> = Vec::with_capacity(SPRITE_ROWS + 1);
    out.push(format!("{}{}{}", color, emote_line(mood, pet.reaction, ticks), RESET));

    let top_pad = SPRITE_ROWS.saturating_sub(rows.len() + bob);
    for _ in 0..top_pad {
        out.push(String::new());
    }
    for r in rows {
        out.push(format!("{}{}{}{}", " ".repeat(MARGIN), color, r, RESET));
    }
    while out.len() < SPRITE_ROWS + 1 {
        out.push(String::new());
    }
    out.truncate(SPRITE_ROWS + 1);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_template_has_exactly_one_of_each_face_placeholder() {
        for sp in Species::ALL {
            for stage in [Stage::Baby, Stage::Adult] {
                for alt in [false, true] {
                    let t = template(sp, stage, alt);
                    for ph in ['@', '#', '%'] {
                        assert_eq!(
                            t.matches(ph).count(),
                            1,
                            "{} {} (frame {}) needs exactly one '{}'",
                            sp.label(),
                            stage.label(),
                            if alt { "B" } else { "A" },
                            ph
                        );
                    }
                    let rows = t.lines().filter(|l| !l.trim().is_empty()).count();
                    assert!(
                        rows < SPRITE_ROWS,
                        "{} {} has {} rows",
                        sp.label(),
                        stage.label(),
                        rows
                    );
                }
            }
        }
    }

    #[test]
    fn templates_are_pure_ascii() {
        for sp in Species::ALL {
            for stage in [Stage::Egg, Stage::Baby, Stage::Adult] {
                for alt in [false, true] {
                    assert!(template(sp, stage, alt).is_ascii(), "{} {}", sp.label(), stage.label());
                }
            }
        }
    }

    #[test]
    fn egg_template_follows_the_hatching_progress() {
        assert!(egg_template(0.0) == EGG_0);
        assert!(egg_template(0.6) == EGG_1);
        assert!(egg_template(0.95) == EGG_2);
    }

    #[test]
    fn faces_match_the_mood_and_reactions_override_it() {
        assert!(face(Mood::Sleeping, Reaction::None, 0) == ('-', '-', '.'));
        assert!(face(Mood::Sleeping, Reaction::Loved, 0) == ('-', '-', 'u'));
        assert!(face(Mood::Happy, Reaction::None, 0) == ('^', '^', 'u'));
        assert!(face(Mood::Ecstatic, Reaction::None, 0) == ('*', '*', 'D'));
        assert!(face(Mood::Sick, Reaction::None, 0).0 == 'x');
        assert!(face(Mood::Sad, Reaction::Eureka, 0) == ('*', '*', 'D'));
    }

    #[test]
    fn render_pet_has_a_fixed_height_and_no_placeholders_left() {
        for sp in Species::ALL {
            // egg (hatching 4% / 80%), baby, adult
            for age in [1.0, 20.0, 100.0, 5000.0] {
                let mut p = Pet::new_egg("T".to_string(), sp);
                p.age = age;
                for ticks in 0..30 {
                    let rows = render_pet(&p, ticks);
                    assert_eq!(rows.len(), SPRITE_ROWS + 1, "{} age {} tick {}", sp.label(), age, ticks);
                    // row 0 is the emote line; the rest is the sprite
                    for r in &rows[1..] {
                        assert!(!r.contains(['@', '#', '%']), "placeholder left in '{r}'");
                    }
                }
            }
        }
    }

    #[test]
    fn emote_lines_are_indented_by_the_margin() {
        for mood in [Mood::Sleeping, Mood::Sick, Mood::Hungry, Mood::Happy, Mood::Ecstatic] {
            assert!(emote_line(mood, Reaction::None, 0).starts_with(&" ".repeat(MARGIN)));
        }
    }
}
