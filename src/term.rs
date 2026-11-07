//! Terminal helpers: ANSI codes, ASCII mode, size probing, formatting.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

// ───────────────────────── Terminal helpers ─────────────────────────

pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
pub const RAINBOW: [&str; 6] = [
    "\x1b[91m", "\x1b[93m", "\x1b[92m", "\x1b[96m", "\x1b[94m", "\x1b[95m",
];

/// Rows of the UI (without the input line).
pub const UI_ROWS: usize = 21;
/// Height of the sprite area.
pub const SPRITE_ROWS: usize = 9;
/// Left margin of the sprites.
pub const MARGIN: usize = 4;

// ───────────────────────── Display mode: ANSI or ASCII ─────────────────────────
// ANSI : colors, in-place animation (cursor control) and Unicode symbols (default).
// ASCII: plain 7-bit text, no escape codes at all. Works in any terminal.
//        The screen is printed below the previous one and refreshes when you press Enter.

pub static ASCII_MODE: AtomicBool = AtomicBool::new(false);

pub fn ascii() -> bool {
    ASCII_MODE.load(Ordering::Relaxed)
}

pub fn set_ascii(on: bool) {
    ASCII_MODE.store(on, Ordering::Relaxed);
}

/// Converts a styled/Unicode string to plain 7-bit ASCII (strips ANSI escapes, maps symbols).
pub fn to_ascii(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\x1b' => {
                if chars.peek() == Some(&'[') {
                    chars.next();
                    while let Some(&n) = chars.peek() {
                        chars.next();
                        if ('@'..='~').contains(&n) {
                            break;
                        }
                    }
                } else {
                    chars.next();
                }
            }
            '█' => out.push('#'),
            '░' => out.push('.'),
            '═' => out.push('='),
            '♥' => out.push('*'),
            '♡' => out.push('.'),
            '♪' => out.push('~'),
            '»' | '«' => out.push('"'),
            '·' => out.push('|'),
            '–' | '—' => out.push('-'),
            c if c.is_ascii() => out.push(c),
            _ => out.push('?'),
        }
    }
    out
}

/// In ASCII mode: strip everything fancy. In ANSI mode: leave the text as it is.
pub fn fx(s: String) -> String {
    if ascii() {
        to_ascii(&s)
    } else {
        s
    }
}

#[cfg(unix)]
pub fn probe_size() -> Option<(usize, usize)> {
    let tty = fs::File::open("/dev/tty").ok()?;
    let out = std::process::Command::new("stty")
        .arg("size")
        .stdin(tty)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let nums: Vec<usize> = text
        .split_whitespace()
        .filter_map(|x| x.parse::<usize>().ok())
        .collect();
    if nums.len() == 2 && nums[0] > 0 && nums[1] > 0 {
        Some((nums[1], nums[0]))
    } else {
        None
    }
}

/// Terminal size as (columns, rows) on Windows (via `mode con`, English or German).
#[cfg(not(unix))]
pub fn probe_size() -> Option<(usize, usize)> {
    let out = std::process::Command::new("cmd")
        .args(["/C", "mode con"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut cols = 0usize;
    let mut rows = 0usize;
    for line in text.lines() {
        let low = line.to_lowercase();
        let n = line
            .split(':')
            .nth(1)
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or(0);
        if low.contains("columns") || low.contains("spalten") {
            cols = n;
        } else if low.contains("lines") || low.contains("zeilen") {
            rows = n;
        }
    }
    if cols > 0 && rows > 0 {
        Some((cols, rows))
    } else {
        None
    }
}

/// (columns, rows) of the terminal, with sane fallbacks.
pub fn term_size() -> (usize, usize) {
    if let Some(sz) = probe_size() {
        return sz;
    }
    let cols = std::env::var("COLUMNS").ok().and_then(|v| v.parse::<usize>().ok()).unwrap_or(80);
    let rows = std::env::var("LINES").ok().and_then(|v| v.parse::<usize>().ok()).unwrap_or(24);
    (cols, rows)
}


pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn save_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".terminal_pet.save")
}

pub fn fmt_duration(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    if s >= 3600 {
        format!("{}h {:02}m", s / 3600, (s % 3600) / 60)
    } else if s >= 60 {
        format!("{}m {:02}s", s / 60, s % 60)
    } else {
        format!("{}s", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_ascii_strips_escapes_and_maps_symbols() {
        let out = to_ascii("\x1b[1;31m♥ ░█ ═ » · – \x1b[0mok");
        assert_eq!(out, "* .# = \" | - ok");
    }

    #[test]
    fn to_ascii_output_is_always_7_bit() {
        assert!(to_ascii("héllo ▀ ☃ ♪ ♡ \x1b[38;2;1;2;3m").is_ascii());
    }

    #[test]
    fn fmt_duration_uses_seconds_minutes_and_hours() {
        assert_eq!(fmt_duration(-5.0), "0s");
        assert_eq!(fmt_duration(59.9), "59s");
        assert_eq!(fmt_duration(61.0), "1m 01s");
        assert_eq!(fmt_duration(3725.0), "1h 02m");
    }

    #[test]
    fn term_size_always_returns_something_usable() {
        let (cols, rows) = term_size();
        assert!(cols > 0 && rows > 0);
    }
}
