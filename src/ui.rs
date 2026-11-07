//! Screen builders and terminal drawing (ASCII + ANSI).

use std::io::{self, Write};

use crate::pet::{Pet, BABY_SECS, EGG_SECS};
use crate::pixel::{
    ANSI_ROWS, canvas_lines, fg_u8, bg_u8, mix, lit, render_scene, rgb_u8, species_rgb, Rgb,
};
use crate::species::{Reaction, Stage};
use crate::sprites::render_pet;
use crate::term::{BOLD, DIM, RESET, UI_ROWS, fmt_duration, to_ascii};

// ── the ANSI layout: header, scene + side panel, message, key hints ──

pub fn pad(s: &str, w: usize) -> String {
    let n = s.chars().count();
    if n >= w {
        s.chars().take(w).collect()
    } else {
        format!("{}{}", s, " ".repeat(w - n))
    }
}

pub fn wrap(text: &str, width: usize, max_lines: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        let wl = word.chars().count();
        let cl = cur.chars().count();
        if cl == 0 {
            cur = word.to_string();
        } else if cl + 1 + wl <= width {
            cur.push(' ');
            cur.push_str(word);
        } else {
            lines.push(cur);
            cur = word.to_string();
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines.truncate(max_lines);
    lines
}

pub fn header_bar(text: &str, c0: Rgb, c1: Rgb, width: usize) -> String {
    let chars: Vec<char> = pad(text, width).chars().collect();
    let mut s = String::from("\x1b[1;97m");
    for (i, ch) in chars.iter().enumerate() {
        let f = i as f32 / (width.max(2) - 1) as f32;
        s.push_str(&bg_u8(rgb_u8(mix(c0, c1, f))));
        s.push(*ch);
    }
    s.push_str(RESET);
    s
}

pub fn grad_bar(v: f64, w: usize) -> String {
    let filled = (((v / 100.0) * w as f64).round() as usize).min(w);
    let vf = (v / 100.0) as f32;
    let base: Rgb = if vf < 0.5 {
        mix((235.0, 70.0, 70.0), (245.0, 205.0, 60.0), vf * 2.0)
    } else {
        mix((245.0, 205.0, 60.0), (70.0, 220.0, 110.0), (vf - 0.5) * 2.0)
    };
    let mut s = String::new();
    for i in 0..w {
        if i < filled {
            let c = lit(base, 0.18 - 0.30 * i as f32 / w as f32);
            s.push_str(&fg_u8(rgb_u8(c)));
            s.push('█');
        } else {
            s.push_str(&fg_u8((58, 58, 78)));
            s.push('░');
        }
    }
    s.push_str(RESET);
    s
}

pub fn stat_row(label: &str, v: f64) -> String {
    format!("{}{:<8}{} {} {:>3}%", DIM, label, RESET, grad_bar(v, 11), v.round() as i64)
}

pub fn keycap(k: &str, label: &str) -> String {
    format!("\x1b[48;5;238m\x1b[1;97m {} \x1b[0m \x1b[37m{}\x1b[0m   ", k, label)
}

pub fn build_screen_ansi(pet: &Pet, t: f32, thought: &str, message: &str) -> Vec<String> {
    let mood = pet.mood();
    let stage = pet.stage();
    let sc = species_rgb(pet.species);
    let fgc = fg_u8(rgb_u8(sc));
    let scene = render_scene(pet.species, stage, mood, pet.reaction, t, pet.egg_progress() as f32);
    let cv = canvas_lines(&scene);

    let mut v: Vec<String> = Vec::with_capacity(ANSI_ROWS);

    // header
    let next = match stage {
        Stage::Egg => format!("hatches in {}", fmt_duration(EGG_SECS - pet.age)),
        Stage::Baby => format!("adult in {}", fmt_duration(EGG_SECS + BABY_SECS - pet.age)),
        Stage::Adult => "fully grown".to_string(),
    };
    let title = format!(
        " ◆ {} the {} ◆  {} · age {} · {}",
        pet.name,
        pet.species.label(),
        stage.label(),
        fmt_duration(pet.age),
        next
    );
    let dark: Rgb = (0.0, 0.0, 0.0);
    v.push(header_bar(&title, mix(sc, dark, 0.55), mix(sc, dark, 0.88), 78));

    // side panel (14 rows)
    let mut panel: Vec<String> = Vec::with_capacity(14);
    let name_part: String = pet.name.chars().take(14).collect();
    let dashes = 23usize.saturating_sub(name_part.chars().count());
    panel.push(format!("{}┌─ {} {}┐{}", fgc, name_part, "─".repeat(dashes), RESET));
    let lines = wrap(thought, 24, 3);
    for i in 0..3 {
        let text = if i < lines.len() { lines[i].as_str() } else { "" };
        panel.push(format!("{}│{} {} {}│{}", fgc, RESET, pad(text, 24), fgc, RESET));
    }
    panel.push(format!("{}└{}┘{}", fgc, "─".repeat(26), RESET));
    panel.push(String::new());
    if stage == Stage::Egg {
        panel.push(stat_row("Hatching", pet.egg_progress() * 100.0));
        for _ in 0..4 {
            panel.push(String::new());
        }
    } else {
        panel.push(stat_row("Food", pet.food));
        panel.push(stat_row("Mood", pet.joy));
        panel.push(stat_row("Energy", pet.energy));
        panel.push(stat_row("Hygiene", pet.clean));
        panel.push(stat_row("Health", pet.health));
    }
    panel.push(String::new());
    let feeling = match pet.reaction {
        Reaction::Listening => "listening (duck)",
        Reaction::Eureka => "EUREKA!",
        _ => mood.label(),
    };
    panel.push(format!("{}Feeling{}  {}{}{}", DIM, RESET, BOLD, feeling, RESET));
    let level = ((pet.care / 8.0) as usize).min(5);
    panel.push(format!(
        "{}Bond{}     {}{}{}{}{}",
        DIM,
        RESET,
        fg_u8((255, 90, 125)),
        "♥".repeat(level),
        fg_u8((90, 70, 90)),
        "♡".repeat(5 - level),
        RESET
    ));

    // scene (left) + panel (right)
    for r in 0..cv.len() {
        let p = if r < panel.len() { panel[r].as_str() } else { "" };
        v.push(format!("{} {}", cv[r], p)); // 50 + 1 + 28 = 79 columns (never touches the last column)
    }

    // message + key hints
    v.push(format!("  {}»{} {}", fgc, RESET, message));
    if pet.duck {
        v.push(format!(
            " {}{}{}",
            keycap("done", "leave duck mode"),
            keycap("/f", "feed"),
            keycap("/q", "quit game")
        ));
        v.push(format!(
            "{}  Everything you type goes to {}. Commands start with a slash.{}",
            DIM, pet.name, RESET
        ));
    } else {
        v.push(format!(
            " {}{}{}{}{}",
            keycap("f", "feed"),
            keycap("p", "play"),
            keycap("s", "pet"),
            keycap("c", "wash"),
            keycap("d", "duck")
        ));
        v.push(format!(
            " {}{}{}{}{}{}",
            keycap("z", "sleep/wake"),
            keycap("m", "medicine"),
            keycap("t", "talk"),
            keycap("i", "info"),
            keycap("a", "mode"),
            keycap("q", "quit")
        ));
    }

    v.truncate(ANSI_ROWS);
    while v.len() < ANSI_ROWS {
        v.push(String::new());
    }
    v
}

// ───────────────────────── User interface ─────────────────────────

pub fn bar(label: &str, v: f64) -> String {
    let w: usize = 20;
    let filled = (((v / 100.0) * w as f64).round() as usize).min(w);
    let color = if v > 60.0 {
        "\x1b[32m"
    } else if v > 30.0 {
        "\x1b[33m"
    } else {
        "\x1b[31m"
    };
    format!(
        "  {:<10} {}{}{}{}{} {:>3}%",
        label,
        color,
        "█".repeat(filled),
        "\x1b[90m",
        "░".repeat(w - filled),
        RESET,
        v.round() as i64
    )
}

pub fn build_screen(pet: &Pet, ticks: u64, thought: &str, message: &str) -> Vec<String> {
    let mood = pet.mood();
    let color = pet.species.color();
    let mut v: Vec<String> = Vec::with_capacity(UI_ROWS);

    // 1: header
    v.push(format!(
        "{}{} ═══ {} · {} · {} · Age {} ═══{}",
        BOLD,
        color,
        pet.name,
        pet.species.label(),
        pet.stage().label(),
        fmt_duration(pet.age),
        RESET
    ));

    // 2: speech bubble
    if thought.is_empty() {
        v.push(String::new());
    } else {
        v.push(format!("  {}»{}«{}", color, thought, RESET));
    }

    // 3–12: pet
    v.extend(render_pet(pet, ticks));

    // 13–17: stats
    if pet.stage() == Stage::Egg {
        v.push(bar("Hatching", pet.egg_progress() * 100.0));
        for _ in 0..4 {
            v.push(String::new());
        }
    } else {
        v.push(bar("Fullness", pet.food));
        v.push(bar("Mood", pet.joy));
        v.push(bar("Energy", pet.energy));
        v.push(bar("Hygiene", pet.clean));
        v.push(bar("Health", pet.health));
    }

    // 18: status
    let level = ((pet.care / 8.0) as usize).min(5);
    let hearts = format!("{}{}", "♥".repeat(level), "♡".repeat(5 - level));
    let next = match pet.stage() {
        Stage::Egg => format!("hatches in {}", fmt_duration(EGG_SECS - pet.age)),
        Stage::Baby => format!(
            "adult in {}",
            fmt_duration(EGG_SECS + BABY_SECS - pet.age)
        ),
        Stage::Adult => "fully grown".to_string(),
    };
    v.push(format!(
        "  Mood: {}{}{}   Bond: {}   {}",
        BOLD,
        match pet.reaction {
            Reaction::Listening => "listening (duck)",
            Reaction::Eureka => "EUREKA!",
            _ => mood.label(),
        },
        RESET,
        hearts,
        next
    ));

    // 19: last message
    v.push(format!("  {}", message));

    // 20–21: menu
    if pet.duck {
        v.push(format!(
            "{}  DUCK MODE: type 'done' (or /done) to leave. Commands start with /  e.g. /f{}",
            DIM, RESET
        ));
        v.push(format!(
            "{}  Everything else you type is an explanation for {}.{}",
            DIM, pet.name, RESET
        ));
    } else {
        v.push(format!(
            "{}  [f] feed  [p] play  [s] pet  [c] wash  [d] duck{}",
            DIM, RESET
        ));
        v.push(format!(
            "{}  [z] sleep/wake  [m] medicine  [t] talk  [i] info  [a] mode  [q] quit{}",
            DIM, RESET
        ));
    }

    v.truncate(UI_ROWS);
    while v.len() < UI_ROWS {
        v.push(String::new());
    }
    v
}

/// Draws the UI at the given offset (that's what centers it). With `full == true` the screen
/// is cleared and the input line is reset too; otherwise the cursor and any half-typed text
/// stay untouched (cursor save/restore).
pub fn draw(lines: &[String], full: bool, col_off: usize, row_off: usize, prompt: &str) {
    let mut out = String::new();
    if full {
        out.push_str("\x1b[2J");
    } else {
        out.push_str("\x1b7");
    }
    for (i, l) in lines.iter().enumerate() {
        out.push_str(&format!(
            "\x1b[{};{}H{}\x1b[0m\x1b[K",
            row_off + i + 1,
            col_off + 1,
            l
        ));
    }
    if full {
        out.push_str(&format!(
            "\x1b[{};{}H{}",
            row_off + lines.len() + 1,
            col_off + 1,
            prompt
        ));
    } else {
        out.push_str("\x1b8");
    }
    let mut so = io::stdout();
    let _ = so.write_all(out.as_bytes());
    let _ = so.flush();
}

/// ASCII mode: prints the screen as plain text (no cursor control, no colors),
/// shifted to the right so that it sits in the middle of the terminal.
pub fn draw_plain(lines: &[String], col_off: usize, prompt: &str) {
    let pad = " ".repeat(col_off);
    let mut out = String::from("\n");
    for l in lines {
        let plain = to_ascii(l);
        let plain = plain.trim_end();
        if !plain.is_empty() {
            out.push_str(&pad);
            out.push_str(plain);
            out.push('\n');
        }
    }
    out.push_str(&pad);
    out.push_str(prompt);
    let mut so = io::stdout();
    let _ = so.write_all(out.as_bytes());
    let _ = so.flush();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::species::Species;

    fn pet_aged(sp: Species, age: f64) -> Pet {
        let mut p = Pet::new_egg("Tester".to_string(), sp);
        p.age = age;
        p
    }

    #[test]
    fn pad_pads_and_truncates_by_characters() {
        assert_eq!(pad("ab", 4), "ab  ");
        assert_eq!(pad("abcdef", 3), "abc");
        assert_eq!(pad("ñandú", 7).chars().count(), 7);
    }

    #[test]
    fn wrap_respects_the_width_and_the_line_limit() {
        let lines = wrap("one two three four five six seven", 9, 3);
        assert_eq!(lines, vec!["one two", "three", "four five"]);
        assert!(wrap("", 10, 3).is_empty());
        assert_eq!(wrap("a b c", 10, 3), vec!["a b c"]);
        assert!(wrap("averyveryverylongword", 5, 3).len() == 1, "long words are not split");
    }

    #[test]
    fn bars_fill_proportionally() {
        assert_eq!(bar("Food", 50.0).matches('█').count(), 10);
        assert_eq!(bar("Food", 0.0).matches('█').count(), 0);
        assert_eq!(bar("Food", 100.0).matches('░').count(), 0);
        assert_eq!(grad_bar(100.0, 11).matches('█').count(), 11);
        assert_eq!(grad_bar(0.0, 11).matches('░').count(), 11);
        assert_eq!(grad_bar(50.0, 10).matches('█').count(), 5);
    }

    #[test]
    fn ansi_screen_has_a_fixed_height_and_shows_the_name() {
        for sp in Species::ALL {
            for age in [1.0, 100.0, 5000.0] {
                let p = pet_aged(sp, age);
                let screen = build_screen_ansi(&p, 1.0, "hello there", "a message");
                assert_eq!(screen.len(), ANSI_ROWS);
                assert!(to_ascii(&screen[0]).contains("Tester"), "header of {}", sp.label());
            }
        }
    }

    #[test]
    fn ansi_screen_lines_fit_the_declared_width() {
        for sp in Species::ALL {
            for age in [1.0, 100.0, 5000.0] {
                let mut p = pet_aged(sp, age);
                p.name = "Fourteen-Chars".to_string(); // the longest name create_pet allows
                for line in build_screen_ansi(&p, 1.0, "a thought", "msg") {
                    let visible = to_ascii(&line);
                    assert!(
                        visible.chars().count() <= crate::pixel::SCREEN_W,
                        "{} columns: {}",
                        visible.chars().count(),
                        visible
                    );
                }
            }
        }
    }

    #[test]
    fn ascii_screen_has_a_fixed_height_and_converts_to_plain_text() {
        for sp in Species::ALL {
            for age in [1.0, 100.0, 5000.0] {
                let p = pet_aged(sp, age);
                let screen = build_screen(&p, 3, "hello there", "a message");
                assert_eq!(screen.len(), UI_ROWS);
                for line in &screen {
                    assert!(to_ascii(line).is_ascii());
                }
            }
        }
    }

    #[test]
    fn duck_mode_screens_explain_how_to_leave() {
        let mut p = pet_aged(Species::Frog, 100.0);
        p.duck = true;
        let plain = build_screen(&p, 0, "", "");
        assert!(plain.iter().any(|l| l.contains("'done'")));
        let ansi = build_screen_ansi(&p, 0.0, "", "");
        assert!(ansi.iter().any(|l| to_ascii(l).contains("leave duck mode")));

        p.duck = false;
        let ansi = build_screen_ansi(&p, 0.0, "", "");
        assert!(ansi.iter().all(|l| !to_ascii(l).contains("leave duck mode")));
    }

    #[test]
    fn the_bond_meter_grows_with_care() {
        let mut p = pet_aged(Species::Fox, 100.0);
        let hearts = |p: &Pet| build_screen(p, 0, "", "").iter().map(|l| l.matches('♥').count()).sum::<usize>();
        assert_eq!(hearts(&p), 0);
        p.care = 17.0; // two full hearts
        assert_eq!(hearts(&p), 2);
        p.care = 1000.0; // capped at five
        assert_eq!(hearts(&p), 5);
    }
}
