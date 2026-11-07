//! Terminal Pet – a Tamagotchi for the command line.
//!
//! Standard library only, no external crates.
//! Controls: letter + Enter (e.g. `f` to feed).
//! The pet lives in real time and is saved to ~/.terminal_pet.save.

mod rng;
mod species;
mod term;
mod pet;
mod sprites;
mod pixel;
mod ui;
mod game;

use std::fs;
use std::io::{self, BufRead};
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::thread;

use game::{create_pet, game_loop, run_gallery, Exit};
use pet::Pet;
use pixel::{ANSI_ROWS, SCREEN_W, TRUECOLOR};
use rng::Rng;
use species::Species;
use term::{ascii, fmt_duration, fx, save_path, set_ascii, term_size};

fn main() {
    let colorterm = std::env::var("COLORTERM").unwrap_or_default();
    if colorterm.contains("truecolor") || colorterm.contains("24bit") || std::env::var("WT_SESSION").is_ok() {
        TRUECOLOR.store(true, Ordering::Relaxed);
    }
    let mut gallery = false;
    let mut pause = false;
    let mut filter: Vec<Species> = Vec::new();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--gallery" | "--test" => gallery = true,
            "--pause" => pause = true,
            "--ascii" => set_ascii(true),
            "--ansi" => set_ascii(false),
            "--truecolor" => TRUECOLOR.store(true, Ordering::Relaxed),
            "--256" => TRUECOLOR.store(false, Ordering::Relaxed),
            "-h" | "--help" => {
                println!("Usage: terminal-pet [--ansi | --ascii] [--truecolor | --256]");
                println!("       terminal-pet --gallery [species ...] [--pause]");
                println!("  --gallery            show every pet in every situation");
                println!("  --gallery penguin    only penguins");
                println!("  --gallery fox frog   several species (any order)");
                println!("  --pause              wait for Enter between species");
                println!("  --ansi               pixel-art scenes (default)");
                println!("  --ascii              plain 7-bit text");
                println!("Species: jellyfish frog fox glimmi penguin");
                println!("In game: a=mode  d=duck  i=info/fact  ?=help  q=quit");
                return;
            }
            other => {
                if let Some(sp) = Species::from_key(&other.to_lowercase()) {
                    if !filter.contains(&sp) {
                        filter.push(sp);
                    }
                }
            }
        }
    }
    if gallery {
        run_gallery(&filter, pause);
        return;
    }

    let (tx, rx) = mpsc::channel::<String>();
    thread::spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(l) => {
                    if tx.send(l).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let mut rng = Rng::new();

    loop {
        let (mut pet, welcome) = match Pet::load() {
            Some((mut p, away)) => {
                p.catch_up(away);
                let msg = if away > 60 {
                    format!("Welcome back! You were gone for {}.", fmt_duration(away as f64))
                } else {
                    "Nice to have you back!".to_string()
                };
                (p, msg)
            }
            None => match create_pet(&rx) {
                Some(p) => (p, "An egg! Pet it ('s') to make it hatch faster.".to_string()),
                None => return,
            },
        };

        pet.save();
        match game_loop(&mut pet, &rx, &mut rng, welcome) {
            Exit::Quit => break,
            Exit::Reset => {
                let _ = fs::remove_file(save_path());
            }
        }
    }

    let (cols, rows) = term_size();
    let pad = " ".repeat(cols.saturating_sub(SCREEN_W) / 2);
    if ascii() {
        println!();
    } else {
        let row_off = rows.saturating_sub(ANSI_ROWS + 2) / 2;
        print!("\x1b[{};1H\x1b[J", row_off + ANSI_ROWS + 2);
    }
    println!("{}{}", pad, fx("Saved. See you soon – your pet is waiting for you!".to_string()));
}
