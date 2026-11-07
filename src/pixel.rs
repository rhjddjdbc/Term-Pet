//! Pixel-art engine for ANSI mode: layer, creatures, backgrounds, scene.

use std::f32::consts::PI;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::species::{Mood, Reaction, Species, Stage};
use crate::term::{RESET};

// ───────────────────────── Pixel-art engine (ANSI mode) ─────────────────────────
// The scene is drawn as a tiny image (PW x PH "pixels") and printed with half-block
// characters (▀): every terminal cell shows TWO pixels (foreground = top, background = bottom).
// Colors are true-color when the terminal supports it, otherwise mapped to the 256-color palette.

pub const PW: usize = 50;
pub const PH: usize = 28;
pub const PWI: i32 = PW as i32;
pub const PHI: i32 = PH as i32;
pub const PWF: f32 = PW as f32;

/// Rows of the ANSI layout (header + 14 scene rows + message + 2 menu rows).
pub const ANSI_ROWS: usize = 18;
/// Width of the ANSI layout (50 scene + 1 gap + 28 panel).
pub const SCREEN_W: usize = 79;
/// Rough width of the plain-text layout (used for centering).
pub const ASCII_W: usize = 70;

pub type Rgb = (f32, f32, f32);

pub static TRUECOLOR: AtomicBool = AtomicBool::new(false);

pub fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    (
        a.0 + (b.0 - a.0) * t,
        a.1 + (b.1 - a.1) * t,
        a.2 + (b.2 - a.2) * t,
    )
}

/// k > 0 lightens, k < 0 darkens (towards a purple shadow tint).
pub fn lit(c: Rgb, k: f32) -> Rgb {
    if k >= 0.0 {
        mix(c, (255.0, 255.0, 255.0), k)
    } else {
        mix(c, (20.0, 12.0, 50.0), -k)
    }
}

pub fn hsv(h: f32, s: f32, v: f32) -> Rgb {
    let h = h - h.floor();
    let i = (h * 6.0) as i32;
    let f = h * 6.0 - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    let (r, g, b) = match i % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    (r * 255.0, g * 255.0, b * 255.0)
}

/// Cheap deterministic pseudo-random number in 0..1.
pub fn hashf(n: u32) -> f32 {
    let mut x = n.wrapping_mul(0x9E37_79B1);
    x ^= x >> 15;
    x = x.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 13;
    ((x >> 8) & 0xFFFF) as f32 / 65535.0
}

pub fn ri(x: f32) -> i32 {
    x.round() as i32
}

pub fn species_rgb(sp: Species) -> Rgb {
    match sp {
        Species::Jellyfish => (150.0, 140.0, 255.0),
        Species::Frog => (110.0, 220.0, 120.0),
        Species::Fox => (250.0, 140.0, 50.0),
        Species::Glimmi => (210.0, 140.0, 255.0),
        Species::Penguin => (90.0, 170.0, 255.0),
    }
}

pub fn egg_base(sp: Species) -> Rgb {
    match sp {
        Species::Jellyfish => (205.0, 238.0, 255.0),
        Species::Frog => (205.0, 236.0, 170.0),
        Species::Fox => (250.0, 218.0, 175.0),
        Species::Glimmi => (232.0, 205.0, 255.0),
        Species::Penguin => (226.0, 236.0, 246.0),
    }
}

/// A layer of RGB pixels with alpha.
pub struct Layer {
    c: Vec<Rgb>,
    a: Vec<f32>,
}

impl Layer {
    pub fn new() -> Layer {
        Layer {
            c: vec![(0.0, 0.0, 0.0); PW * PH],
            a: vec![0.0; PW * PH],
        }
    }

    pub fn alpha(&self, x: i32, y: i32) -> f32 {
        if x < 0 || y < 0 || x >= PWI || y >= PHI {
            0.0
        } else {
            self.a[y as usize * PW + x as usize]
        }
    }

    /// Alpha-composites one pixel over what is already there.
    pub fn put(&mut self, x: i32, y: i32, col: Rgb, alpha: f32) {
        if x < 0 || y < 0 || x >= PWI || y >= PHI || alpha <= 0.0 {
            return;
        }
        let i = y as usize * PW + x as usize;
        let a0 = self.a[i];
        if alpha >= 0.999 {
            self.c[i] = col;
            self.a[i] = 1.0;
            return;
        }
        let oa = alpha + a0 * (1.0 - alpha);
        let c0 = self.c[i];
        let k0 = a0 * (1.0 - alpha);
        self.c[i] = (
            (col.0 * alpha + c0.0 * k0) / oa,
            (col.1 * alpha + c0.1 * k0) / oa,
            (col.2 * alpha + c0.2 * k0) / oa,
        );
        self.a[i] = oa;
    }
}

pub fn plot(l: &mut Layer, x: f32, y: f32, col: Rgb, alpha: f32) {
    l.put(x.floor() as i32, y.floor() as i32, col, alpha);
}

/// Toon-style light bands for a sphere lit from the upper left.
pub fn shade_k(u: f32, v: f32) -> f32 {
    let z = (1.0 - u * u - v * v).max(0.0).sqrt();
    let k = -0.45 * u - 0.55 * v + 0.70 * z;
    if k > 0.65 {
        0.30
    } else if k > 0.25 {
        0.0
    } else if k > -0.10 {
        -0.20
    } else {
        -0.42
    }
}

#[allow(clippy::too_many_arguments)]
pub fn ellipse(
    l: &mut Layer,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    base: Rgb,
    alpha: f32,
    shade: bool,
    vmax: f32,
) {
    let y0 = (cy - ry).floor() as i32;
    let y1 = (cy + ry).ceil() as i32;
    let x0 = (cx - rx).floor() as i32;
    let x1 = (cx + rx).ceil() as i32;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let u = (x as f32 + 0.5 - cx) / rx;
            let v = (y as f32 + 0.5 - cy) / ry;
            if u * u + v * v <= 1.0 && v <= vmax {
                let col = if shade { lit(base, shade_k(u, v)) } else { base };
                l.put(x, y, col, alpha);
            }
        }
    }
}

/// Shaded, opaque ellipse.
pub fn blob(l: &mut Layer, cx: f32, cy: f32, rx: f32, ry: f32, base: Rgb) {
    ellipse(l, cx, cy, rx, ry, base, 1.0, true, 1.0);
}

/// Shaded, opaque circle.
pub fn ball(l: &mut Layer, cx: f32, cy: f32, r: f32, base: Rgb) {
    ellipse(l, cx, cy, r, r, base, 1.0, true, 1.0);
}

/// Flat (unshaded) circle with alpha.
pub fn disc(l: &mut Layer, cx: f32, cy: f32, r: f32, col: Rgb, alpha: f32) {
    ellipse(l, cx, cy, r, r, col, alpha, false, 1.0);
}

pub fn tri(l: &mut Layer, p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), col: Rgb, alpha: f32) {
    let minx = p0.0.min(p1.0).min(p2.0).floor() as i32;
    let maxx = p0.0.max(p1.0).max(p2.0).ceil() as i32;
    let miny = p0.1.min(p1.1).min(p2.1).floor() as i32;
    let maxy = p0.1.max(p1.1).max(p2.1).ceil() as i32;
    let sg = |a: (f32, f32), b: (f32, f32), c: (f32, f32)| -> f32 {
        (a.0 - c.0) * (b.1 - c.1) - (b.0 - c.0) * (a.1 - c.1)
    };
    for y in miny..=maxy {
        for x in minx..=maxx {
            let p = (x as f32 + 0.5, y as f32 + 0.5);
            let d1 = sg(p, p0, p1);
            let d2 = sg(p, p1, p2);
            let d3 = sg(p, p2, p0);
            let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
            let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
            if !(neg && pos) {
                l.put(x, y, col, alpha);
            }
        }
    }
}

pub fn vgrad(sc: &mut Layer, y0: i32, y1: i32, c0: Rgb, c1: Rgb) {
    for y in y0..y1 {
        let f = (y - y0) as f32 / ((y1 - y0 - 1).max(1)) as f32;
        let c = mix(c0, c1, f);
        for x in 0..PWI {
            sc.put(x, y, c, 1.0);
        }
    }
}

/// Soft radial light.
pub fn glow(sc: &mut Layer, cx: f32, cy: f32, r: f32, col: Rgb, strength: f32) {
    let y0 = (cy - r) as i32 - 1;
    let y1 = (cy + r) as i32 + 1;
    let x0 = (cx - r) as i32 - 1;
    let x1 = (cx + r) as i32 + 1;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let dx = x as f32 + 0.5 - cx;
            let dy = (y as f32 + 0.5 - cy) * 1.1;
            let d = (dx * dx + dy * dy).sqrt() / r;
            if d < 1.0 {
                sc.put(x, y, col, strength * (1.0 - d) * (1.0 - d));
            }
        }
    }
}

// ── tiny pixel patterns (eyes, mouths, particles) ──

pub fn pal(ch: char) -> Option<Rgb> {
    match ch {
        'K' => Some((22.0, 16.0, 30.0)),
        'S' | 'W' => Some((255.0, 255.0, 255.0)),
        'Y' => Some((255.0, 226.0, 90.0)),
        'B' => Some((110.0, 190.0, 255.0)),
        'R' => Some((255.0, 90.0, 125.0)),
        'r' => Some((190.0, 40.0, 70.0)),
        'G' => Some((120.0, 230.0, 140.0)),
        'Z' => Some((200.0, 210.0, 255.0)),
        'P' => Some((255.0, 150.0, 170.0)),
        'O' => Some((255.0, 170.0, 60.0)),
        _ => None,
    }
}

pub fn pattern(l: &mut Layer, x: i32, y: i32, rows: &[&str], mirror: bool, alpha: f32) {
    for (j, row) in rows.iter().enumerate() {
        let chars: Vec<char> = if mirror {
            row.chars().rev().collect()
        } else {
            row.chars().collect()
        };
        for (i, ch) in chars.iter().enumerate() {
            if let Some(c) = pal(*ch) {
                l.put(x + i as i32, y + j as i32, c, alpha);
            }
        }
    }
}

pub const HEART: [&str; 4] = [".R.R.", "RRRRR", ".RRR.", "..R.."];
pub const ZEE_A: [&str; 4] = ["ZZZ", ".Z.", "Z..", "ZZZ"];
pub const ZEE_B: [&str; 3] = ["ZZ.", ".Z.", ".ZZ"];
pub const SPARK: [&str; 3] = [".Y.", "YYY", ".Y."];
pub const CROSS: [&str; 3] = [".R.", "RRR", ".R."];
pub const ANGER: [&str; 4] = [".R.R", "RRRR", ".R.R", "RRRR"];
pub const EXCLAIM: [&str; 5] = ["Y", "Y", "Y", ".", "Y"];
pub const BULB: [&str; 6] = [".YYY.", "YYYYY", "YYYYY", ".YYY.", "..O..", ".OOO."];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Eye {
    Open,
    Blink,
    Happy,
    Star,
    Sad,
    Angry,
    Sick,
    Droop,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mouth {
    Smile,
    Flat,
    Open,
    Frown,
    Grin,
    Wavy,
    Dot,
}

pub fn eye_rows(k: Eye) -> [&'static str; 5] {
    match k {
        Eye::Open => ["...", "...", "SKK", "KKK", ".K."],
        Eye::Blink => ["...", "...", "...", "KKK", "..."],
        Eye::Happy => ["...", "...", "...", ".K.", "K.K"],
        Eye::Star => ["...", ".Y.", "YYY", ".Y.", "..."],
        Eye::Sad => [".KK", "K..", "SKK", "KKK", "B.."],
        Eye::Angry => ["KK.", "..K", "SKK", "KKK", "..."],
        Eye::Sick => ["...", "K.K", ".K.", "K.K", "..."],
        Eye::Droop => ["...", "...", "KKK", "SKK", "..."],
    }
}

pub fn mouth_rows(k: Mouth) -> &'static [&'static str] {
    match k {
        Mouth::Smile => &["K...K", ".KKK."],
        Mouth::Flat => &["KKK"],
        Mouth::Open => &[".KKK.", "KRRRK", ".KKK."],
        Mouth::Frown => &[".KKK.", "K...K"],
        Mouth::Grin => &["KKKKK", "KWWWK", ".KKK."],
        Mouth::Wavy => &["K.K.K", ".K.K."],
        Mouth::Dot => &[".K.", "K.K", ".K."],
    }
}

pub fn draw_eye(l: &mut Layer, cx: i32, cy: i32, kind: Eye, mirror: bool) {
    pattern(l, cx - 1, cy - 3, &eye_rows(kind), mirror, 1.0);
}

pub fn draw_mouth(l: &mut Layer, cx: i32, y: i32, kind: Mouth) {
    let rows = mouth_rows(kind);
    let w = rows[0].chars().count() as i32;
    pattern(l, cx - w / 2, y, rows, false, 1.0);
}

/// Mood + reaction → facial expression.
pub fn expression(mood: Mood, reaction: Reaction, t: f32) -> (Eye, Mouth) {
    let blink = (t % 5.0) > 4.8;
    let eo = if blink { Eye::Blink } else { Eye::Open };
    if mood == Mood::Sleeping {
        let m = if reaction == Reaction::Loved { Mouth::Smile } else { Mouth::Dot };
        return (Eye::Blink, m);
    }
    match reaction {
        Reaction::Eating => {
            let m = if (t * 4.0) as i32 % 2 == 0 { Mouth::Open } else { Mouth::Smile };
            return (Eye::Happy, m);
        }
        Reaction::Playing => return (Eye::Star, Mouth::Grin),
        Reaction::Loved => return (Eye::Happy, Mouth::Smile),
        Reaction::Washing => return (Eye::Blink, Mouth::Open),
        Reaction::Medicine => return (Eye::Angry, Mouth::Wavy),
        Reaction::Listening => {
            let m = if (t * 1.5) as i32 % 2 == 0 { Mouth::Flat } else { Mouth::Dot };
            return (Eye::Open, m);
        }
        Reaction::Eureka => return (Eye::Star, Mouth::Grin),
        Reaction::None => {}
    }
    match mood {
        Mood::Sick => (Eye::Sick, Mouth::Wavy),
        Mood::Hungry => {
            let m = if (t * 3.0) as i32 % 2 == 0 { Mouth::Open } else { Mouth::Flat };
            (eo, m)
        }
        Mood::Tired => (Eye::Droop, Mouth::Flat),
        Mood::Grumpy => (Eye::Angry, Mouth::Frown),
        Mood::Sad => (Eye::Sad, Mouth::Frown),
        Mood::Lonely => (Eye::Sad, Mouth::Flat),
        Mood::Happy => (eo, Mouth::Smile),
        Mood::Ecstatic => (Eye::Star, Mouth::Grin),
        Mood::Neutral | Mood::Egg | Mood::Sleeping => (eo, Mouth::Flat),
    }
}

/// Dark 1-pixel outline around everything drawn on the layer.
pub fn outline(l: &mut Layer, col: Rgb) {
    let mut mark: Vec<(i32, i32)> = Vec::new();
    for y in 0..PHI {
        for x in 0..PWI {
            if l.alpha(x, y) < 0.5 {
                let n = l
                    .alpha(x + 1, y)
                    .max(l.alpha(x - 1, y))
                    .max(l.alpha(x, y + 1))
                    .max(l.alpha(x, y - 1));
                if n > 0.6 {
                    mark.push((x, y));
                }
            }
        }
    }
    for (x, y) in mark {
        l.put(x, y, col, 1.0);
    }
}

pub fn composite(sc: &mut Layer, l: &Layer) {
    for y in 0..PH {
        for x in 0..PW {
            let i = y * PW + x;
            let a = l.a[i];
            if a > 0.0 {
                sc.put(x as i32, y as i32, l.c[i], a);
            }
        }
    }
}

// ── creatures ──

pub fn draw_jelly(l: &mut Layer, cx: f32, cy: f32, s: f32, t: f32, ex: (Eye, Mouth), mood: Mood) {
    let pulse = (t * 2.4).sin();
    let rx = 11.0 * s - pulse * 0.5 * s;
    let ry = 9.5 * s + pulse * 0.6 * s;

    // trailing tentacles
    let n = 6;
    for i in 0..n {
        let x0 = cx - 8.0 * s + i as f32 * (16.0 * s / (n as f32 - 1.0));
        let len = (12.0 * s) as i32 + 2;
        for k in 0..len {
            let f = k as f32 / (12.0 * s + 1.0);
            let x = x0 + (t * 1.8 + k as f32 * 0.55 + i as f32 * 1.3).sin() * 1.6 * (f + 0.2);
            let y = cy + 3.5 * s + k as f32;
            let col = mix((170.0, 130.0, 255.0), (255.0, 130.0, 210.0), f);
            plot(l, x, y, col, 0.9 - 0.65 * f);
            if i % 2 == 0 {
                plot(l, x + 1.0, y, col, 0.5 - 0.4 * f);
            }
        }
    }
    // oral arms
    for i in 0..3 {
        let x0 = cx - 3.0 * s + i as f32 * 3.0 * s;
        let len = (9.0 * s) as i32 + 2;
        for k in 0..len {
            let f = k as f32 / (9.0 * s + 1.0);
            let x = x0 + (t * 1.4 + k as f32 * 0.7 + i as f32 * 2.1).sin() * 1.2 * (f + 0.3);
            let y = cy + 4.5 * s + k as f32;
            plot(l, x, y, (255.0, 170.0, 230.0), 0.85 - 0.5 * f);
            plot(l, x + 1.0, y, (255.0, 170.0, 230.0), 0.7 - 0.5 * f);
        }
    }
    // translucent bell
    let mut base: Rgb = (110.0, 175.0, 255.0);
    if mood == Mood::Sick {
        base = (140.0, 200.0, 170.0);
    }
    ellipse(l, cx, cy, rx, ry, base, 0.88, true, 0.42);
    // wavy rim
    let m = (rx * 2.0) as i32;
    for i in 0..=m {
        let fx = cx - rx + i as f32 * (2.0 * rx / m as f32);
        let fy = cy + ry * 0.42 + (t * 2.0 + i as f32 * 0.9).sin() * 0.7;
        disc(l, fx, fy, 0.9 * s.max(0.7), (200.0, 150.0, 255.0), 0.9);
    }
    // inner organs
    for (ox, oy) in [(-2.6_f32, 1.4_f32), (2.6, 1.4), (-1.2, 3.0), (1.2, 3.0)] {
        disc(l, cx + ox * s, cy + oy * s - 1.0 * s, 1.3 * s + 0.3, (255.0, 130.0, 210.0), 0.45);
    }
    // glowing spots
    for i in 0..5 {
        let a = PI * (0.15 + 0.7 * i as f32 / 4.0);
        let px = cx - a.cos() * rx * 0.82;
        let py = cy - a.sin() * ry * 0.82;
        plot(l, px, py, (200.0, 255.0, 255.0), 0.5 + 0.5 * (t * 3.0 + i as f32).sin());
    }
    // face
    let sp = (4.2 * s).round().max(3.0);
    let ey = ri(cy - 1.0 * s + 1.0);
    draw_eye(l, ri(cx - sp), ey, ex.0, false);
    draw_eye(l, ri(cx + sp), ey, ex.0, true);
    draw_mouth(l, ri(cx), ri(cy + 3.0 * s - 0.5 * s), ex.1);
}

pub fn draw_frog(l: &mut Layer, cx: f32, gy: f32, s: f32, t: f32, ex: (Eye, Mouth), mood: Mood) {
    let mut g: Rgb = (96.0, 200.0, 90.0);
    let d: Rgb = (60.0, 150.0, 70.0);
    if mood == Mood::Sick {
        g = (150.0, 190.0, 100.0);
    }
    let bob = 0.4 * (t * 2.0).sin();
    // hind legs and feet
    for sx in [-1.0_f32, 1.0] {
        blob(l, cx + sx * 9.5 * s, gy - 3.0 * s, 4.0 * s, 3.2 * s, d);
        blob(l, cx + sx * 11.5 * s, gy - 0.8 * s, 3.4 * s, 1.4 * s, g);
    }
    // body + belly
    blob(l, cx, gy - 7.0 * s + bob * 0.5, 10.0 * s, 6.5 * s, g);
    blob(l, cx, gy - 5.2 * s + bob * 0.5, 6.5 * s, 4.2 * s, (215.0, 242.0, 150.0));
    // croaking throat pouch
    let p = (t * 2.6).sin().max(0.0);
    ellipse(
        l,
        cx,
        gy - 5.0 * s,
        3.0 * s + p * 1.4 * s,
        1.8 * s + p * 1.2 * s,
        (255.0, 175.0, 150.0),
        0.95,
        true,
        1.0,
    );
    // head
    blob(l, cx, gy - 11.5 * s + bob, 9.5 * s, 5.6 * s, g);
    // eye bumps
    for sx in [-1.0_f32, 1.0] {
        ball(l, cx + sx * 6.2 * s, gy - 16.2 * s + bob, 3.9 * s + 0.2, g);
        disc(l, cx + sx * 6.2 * s, gy - 16.0 * s + bob, 3.0 * s + 0.2, (255.0, 255.0, 225.0), 1.0);
    }
    // back spots
    for (ox, oy) in [(-5.0_f32, -9.5_f32), (4.0, -9.0), (0.0, -8.0)] {
        disc(l, cx + ox * s, gy + oy * s, 0.9 * s + 0.2, d, 0.8);
    }
    // front arms
    for sx in [-1.0_f32, 1.0] {
        blob(l, cx + sx * 5.0 * s, gy - 1.6 * s, 1.9 * s, 2.0 * s, d);
    }
    // face
    let ey = ri(gy - 16.0 * s + bob);
    draw_eye(l, ri(cx - 6.2 * s), ey + 1, ex.0, false);
    draw_eye(l, ri(cx + 6.2 * s), ey + 1, ex.0, true);
    draw_mouth(l, ri(cx), ri(gy - 11.2 * s + bob), ex.1);
    if ex.1 == Mouth::Smile || ex.1 == Mouth::Grin {
        for sx in [-1.0_f32, 1.0] {
            plot(l, cx + sx * 7.5 * s, gy - 11.0 * s + bob, (255.0, 140.0, 150.0), 0.8);
            plot(l, cx + sx * 8.5 * s, gy - 11.0 * s + bob, (255.0, 140.0, 150.0), 0.6);
        }
    }
}

pub fn draw_fox(l: &mut Layer, cx: f32, gy: f32, s: f32, t: f32, ex: (Eye, Mouth), mood: Mood) {
    let mut o: Rgb = (240.0, 122.0, 40.0);
    let wh: Rgb = (252.0, 243.0, 228.0);
    let dk: Rgb = (70.0, 40.0, 34.0);
    if mood == Mood::Sick {
        o = (214.0, 150.0, 90.0);
    }
    // wagging tail (behind the body)
    let wag = (t * 3.0).sin() * 0.45;
    let mut px = cx + 5.5 * s;
    let mut py = gy - 3.2 * s;
    let steps = (15.0 * s) as i32 + 3;
    for k in 0..steps {
        let f = k as f32 / steps as f32;
        let ang = -0.12 + wag * f - f * 1.05;
        px += ang.cos() * 0.9;
        py += ang.sin() * 0.9;
        let r = (3.1 - 1.2 * f) * s + 0.6;
        let col = if f > 0.8 { wh } else { mix(o, (200.0, 85.0, 25.0), f) };
        ball(l, px, py, r, col);
    }
    // body, chest, paws
    blob(l, cx, gy - 5.2 * s, 7.4 * s, 5.0 * s, o);
    blob(l, cx, gy - 5.0 * s, 3.6 * s, 4.0 * s, wh);
    for sx in [-1.0_f32, 1.0] {
        blob(l, cx + sx * 3.6 * s, gy - 1.2 * s, 2.1 * s, 1.5 * s, dk);
    }
    // ears
    for sx in [-1.0_f32, 1.0] {
        tri(
            l,
            (cx + sx * 8.6 * s, gy - 16.0 * s),
            (cx + sx * 7.8 * s, gy - 26.0 * s),
            (cx + sx * 2.4 * s, gy - 21.0 * s),
            o,
            1.0,
        );
        tri(
            l,
            (cx + sx * 7.0 * s, gy - 17.5 * s),
            (cx + sx * 6.8 * s, gy - 23.5 * s),
            (cx + sx * 3.6 * s, gy - 20.5 * s),
            dk,
            1.0,
        );
    }
    // head, cheeks, muzzle, nose
    blob(l, cx, gy - 14.5 * s, 8.8 * s, 6.4 * s, o);
    for sx in [-1.0_f32, 1.0] {
        tri(
            l,
            (cx + sx * 8.8 * s, gy - 13.5 * s),
            (cx + sx * 3.0 * s, gy - 8.8 * s),
            (cx + sx * 1.0 * s, gy - 14.2 * s),
            wh,
            1.0,
        );
    }
    ellipse(l, cx, gy - 11.6 * s, 4.2 * s, 2.8 * s, wh, 1.0, false, 1.0);
    disc(l, cx, gy - 13.2 * s, 1.15 * s + 0.2, (30.0, 20.0, 30.0), 1.0);
    // face
    let ey = ri(gy - 15.2 * s);
    let es = (4.3 * s).round().max(3.0);
    draw_eye(l, ri(cx - es), ey, ex.0, false);
    draw_eye(l, ri(cx + es), ey, ex.0, true);
    draw_mouth(l, ri(cx), ri(gy - 11.2 * s), ex.1);
}

#[allow(clippy::too_many_arguments)]
pub fn draw_glimmi(l: &mut Layer, cx: f32, cy: f32, s: f32, t: f32, ex: (Eye, Mouth), mood: Mood, hue: f32) {
    let mut base: Rgb = if hue >= 0.0 { hsv(hue, 0.45, 1.0) } else { (190.0, 135.0, 255.0) };
    if mood == Mood::Sick {
        base = (170.0, 190.0, 150.0);
    }
    // glowing crest
    for (i, &(ox, hh, ww)) in [(0.0_f32, 10.0_f32, 2.6_f32), (-4.5, 7.0, 2.2), (4.5, 7.0, 2.2)]
        .iter()
        .enumerate()
    {
        let sw = (t * 2.0 + i as f32).sin() * 0.8;
        tri(
            l,
            (cx + (ox - ww) * s, cy - 6.5 * s),
            (cx + ox * s + sw, cy - (6.5 + hh) * s),
            (cx + (ox + ww) * s, cy - 6.5 * s),
            (255.0, 236.0, 130.0),
            1.0,
        );
    }
    // feet and arms
    for sx in [-1.0_f32, 1.0] {
        blob(l, cx + sx * 4.2 * s, cy + 8.2 * s, 2.7 * s, 1.9 * s, mix(base, (80.0, 40.0, 140.0), 0.45));
        blob(
            l,
            cx + sx * 10.2 * s,
            cy + 2.2 * s + (t * 3.0 + sx).sin() * 0.6,
            2.1 * s,
            2.5 * s,
            mix(base, (80.0, 40.0, 140.0), 0.2),
        );
    }
    blob(l, cx, cy, 9.8 * s, 8.8 * s, base);
    // belly gem
    let pulse = 0.5 + 0.5 * (t * 3.0).sin();
    let gx = cx;
    let gy2 = cy + 5.0 * s;
    let r = 2.0 * s + 0.6;
    tri(
        l,
        (gx, gy2 - r),
        (gx - r, gy2),
        (gx + r, gy2),
        mix((255.0, 240.0, 150.0), (255.0, 255.0, 255.0), pulse),
        1.0,
    );
    tri(
        l,
        (gx, gy2 + r * 1.3),
        (gx - r, gy2),
        (gx + r, gy2),
        mix((255.0, 190.0, 90.0), (255.0, 240.0, 160.0), pulse),
        1.0,
    );
    // face
    let sp = (4.3 * s).round().max(3.0);
    let ey = ri(cy - 0.5 * s);
    draw_eye(l, ri(cx - sp), ey, ex.0, false);
    draw_eye(l, ri(cx + sp), ey, ex.0, true);
    draw_mouth(l, ri(cx), ri(cy + 2.6 * s), ex.1);
    if ex.1 == Mouth::Smile || ex.1 == Mouth::Grin {
        for sx in [-1.0_f32, 1.0] {
            plot(l, cx + sx * 7.0 * s, cy + 2.0 * s, (255.0, 120.0, 170.0), 0.8);
            plot(l, cx + sx * 8.0 * s, cy + 2.0 * s, (255.0, 120.0, 170.0), 0.6);
        }
    }
}

/// Shaded egg / torpedo shape. `k > 0` makes it wider at the bottom than at the top.
pub fn egg_blob(l: &mut Layer, cx: f32, cy: f32, rx: f32, ry: f32, k: f32, base: Rgb) {
    let y0 = (cy - ry).floor() as i32 - 1;
    let y1 = (cy + ry).ceil() as i32 + 1;
    let x0 = (cx - rx * 1.4).floor() as i32 - 1;
    let x1 = (cx + rx * 1.4).ceil() as i32 + 1;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let v = (y as f32 + 0.5 - cy) / ry;
            let u = (x as f32 + 0.5 - cx) / rx / (1.0 + k * v);
            if u * u + v * v <= 1.0 {
                l.put(x, y, lit(base, shade_k(u, v)), 1.0);
            }
        }
    }
}

/// Penguin with realistic proportions: tall torpedo-shaped body (about twice as tall as wide),
/// a clearly smaller head, long slim flippers hanging close to the body, a short stiff tail,
/// a long pointed beak and short legs with orange webbed feet.
/// `young` draws a fluffy chick instead: round body, big head, stubby flippers.
#[allow(clippy::too_many_arguments)]
pub fn draw_penguin(l: &mut Layer, cx: f32, gy: f32, s: f32, t: f32, ex: (Eye, Mouth), mood: Mood, young: bool) {
    let bob = 0.25 * (t * 2.1).sin();
    let walk = (t * 2.4).sin() * 0.3 * s;
    let beak: Rgb = (255.0, 172.0, 44.0);
    let feet: Rgb = (255.0, 145.0, 35.0);
    let belly: Rgb = (250.0, 252.0, 255.0);
    let smiling = ex.1 == Mouth::Smile || ex.1 == Mouth::Grin;

    if !young {
        let wave = (t * 3.0).sin();
        let mut body: Rgb = (28.0, 32.0, 48.0);
        if mood == Mood::Sick {
            body = mix(body, (120.0, 170.0, 110.0), 0.45);
        }

        // short legs and orange webbed feet with three toes
        for sx in [-1.0_f32, 1.0] {
            let lx = cx + sx * 2.6 * s + sx * walk * 0.3;
            blob(l, lx, gy - 2.2 * s, 1.5 * s, 2.0 * s, body);
            let fx = cx + sx * 3.0 * s + sx * walk * 0.4;
            blob(l, fx, gy - 0.4 * s, 3.0 * s, 1.3 * s, feet);
            for ti in 0..3 {
                ball(l, fx + (ti as f32 - 1.0) * 1.1 * s, gy + 0.45 * s, 0.8 * s, feet);
            }
        }
        // short stiff tail
        tri(
            l,
            (cx - 2.0 * s, gy - 3.0 * s),
            (cx + 2.0 * s, gy - 3.0 * s),
            (cx, gy + 0.2 * s),
            body,
            1.0,
        );
        // long slim flippers (the right one waves a little)
        for sx in [-1.0_f32, 1.0] {
            let x0 = cx + sx * 5.0 * s;
            let y0 = gy - 16.0 * s + bob;
            let flap = if sx > 0.0 { wave * 1.5 * s } else { -wave * 0.5 * s };
            let x1 = cx + sx * 8.6 * s + sx * flap * 0.25;
            let y1 = gy - 6.2 * s + flap * 0.6;
            let n = (13.0 * s) as i32 + 4;
            for k in 0..=n {
                let f = k as f32 / n as f32;
                ball(
                    l,
                    x0 + (x1 - x0) * f,
                    y0 + (y1 - y0) * f,
                    (1.8 - 0.8 * f) * s + 0.35,
                    mix(body, (10.0, 12.0, 30.0), 0.2),
                );
            }
        }
        // torpedo body with white belly
        egg_blob(l, cx, gy - 9.4 * s + bob, 5.9 * s, 8.8 * s, 0.24, body);
        egg_blob(l, cx, gy - 8.4 * s + bob, 4.0 * s, 7.2 * s, 0.28, belly);
        // small head with white cheeks
        let hy = gy - 19.6 * s + bob;
        blob(l, cx, hy, 4.5 * s, 3.9 * s, body);
        blob(l, cx, hy + 0.9 * s, 3.5 * s, 2.7 * s, belly);
        // long pointed beak (opens for Open / Grin)
        let by = hy + 0.8 * s;
        let blen = 4.2 * s;
        tri(l, (cx - 1.5 * s, by), (cx + 1.5 * s, by), (cx, by + blen), beak, 1.0);
        if ex.1 == Mouth::Open || ex.1 == Mouth::Grin {
            tri(
                l,
                (cx - 0.9 * s, by + 1.2 * s),
                (cx + 0.9 * s, by + 1.2 * s),
                (cx, by + blen - 0.4 * s),
                (70.0, 24.0, 34.0),
                1.0,
            );
        }
        plot(l, cx - 0.6 * s, by + 0.5, (255.0, 214.0, 110.0), 0.8);
        // eyes (both look forward, no mirroring) and blush
        let ecx = ri(cx);
        let ey = ri(hy - 0.3 * s);
        let es = ((3.0 * s).round() as i32).max(3);
        draw_eye(l, ecx - es, ey, ex.0, false);
        draw_eye(l, ecx + es, ey, ex.0, false);
        if smiling {
            for sx in [-1_i32, 1] {
                plot(l, (ecx + sx * 4) as f32, hy + 1.6 * s, (255.0, 140.0, 160.0), 0.75);
                plot(l, (ecx + sx * 5) as f32, hy + 1.6 * s, (255.0, 140.0, 160.0), 0.55);
            }
        }
    } else {
        // chick: fluffy grey down, big head with a white face, stubby flippers
        let mut down: Rgb = (120.0, 128.0, 155.0);
        if mood == Mood::Sick {
            down = mix(down, (120.0, 170.0, 110.0), 0.45);
        }
        let flap = (t * 4.0).sin();
        for sx in [-1.0_f32, 1.0] {
            let fx = cx + sx * 2.5 + sx * walk * 0.4;
            blob(l, fx, gy - 0.5, 2.4, 1.1, feet);
            for ti in 0..3 {
                ball(l, fx + (ti as f32 - 1.0) * 0.9, gy + 0.35, 0.65, feet);
            }
        }
        for sx in [-1.0_f32, 1.0] {
            let f = if sx > 0.0 { flap * 0.9 } else { 0.0 };
            blob(
                l,
                cx + sx * 5.9 + sx * f * 0.4,
                gy - 5.4 + f,
                1.5,
                2.7,
                mix(down, (60.0, 66.0, 96.0), 0.35),
            );
        }
        egg_blob(l, cx, gy - 5.4 + bob, 5.4, 5.4, 0.10, down);
        egg_blob(l, cx, gy - 4.8 + bob, 3.6, 4.0, 0.12, belly);
        let hy = gy - 11.8 + bob;
        blob(l, cx, hy, 4.6, 4.2, mix(down, (30.0, 34.0, 56.0), 0.55));
        blob(l, cx, hy + 0.7, 3.7, 2.9, belly);
        let by = hy + 0.9;
        tri(l, (cx - 1.1, by), (cx + 1.1, by), (cx, by + 2.5), beak, 1.0);
        let ecx = ri(cx);
        let ey = ri(hy - 0.2);
        draw_eye(l, ecx - 3, ey, ex.0, false);
        draw_eye(l, ecx + 3, ey, ex.0, false);
        if smiling {
            for sx in [-1_i32, 1] {
                plot(l, (ecx + sx * 4) as f32, hy + 1.4, (255.0, 140.0, 160.0), 0.75);
            }
        }
    }
}

/// Draws the egg (with nest). Returns the egg's center.
pub fn draw_egg(l: &mut Layer, cx: f32, gy: f32, t: f32, progress: f32, species: Species) -> (f32, f32) {
    let base = egg_base(species);
    let spot = species_rgb(species);
    let mut wob = 0.0_f32;
    if progress > 0.5 {
        let amp = if progress < 0.85 { 0.6 } else { 1.1 };
        wob = (t * 11.0).sin() * amp;
    }
    let ecx = cx + wob;
    let ecy = gy - 7.2;
    let rx = 6.2_f32;
    let ry = 8.2_f32;
    // straw nest
    blob(l, cx, gy - 0.8, 10.0, 2.6, (150.0, 105.0, 60.0));
    for i in 0..12 {
        let x = cx - 9.0 + i as f32 * 1.7;
        plot(l, x, gy - 2.6 + (i % 2) as f32, (200.0, 160.0, 90.0), 1.0);
    }
    // egg shell, a bit narrower at the top
    for y in ((ecy - ry) as i32 - 1)..=((ecy + ry) as i32 + 1) {
        for x in ((ecx - rx) as i32 - 1)..=((ecx + rx) as i32 + 1) {
            let v = (y as f32 + 0.5 - ecy) / ry;
            let u = (x as f32 + 0.5 - ecx) / rx / (1.0 + 0.17 * v);
            if u * u + v * v <= 1.0 {
                let mut col = lit(base, shade_k(u, v));
                for (sx, sy, sr) in [(-0.3_f32, -0.2_f32, 0.28_f32), (0.35, 0.25, 0.24), (-0.1, 0.55, 0.2), (0.2, -0.5, 0.17)] {
                    if (u - sx) * (u - sx) + (v - sy) * (v - sy) < sr * sr {
                        col = lit(spot, shade_k(u, v) * 0.8);
                    }
                }
                l.put(x, y, col, 1.0);
            }
        }
    }
    // cracks (glowing when about to hatch)
    if progress > 0.5 {
        let n = ((progress - 0.5) * 2.0 * 9.0) as i32 + 2;
        let mut x = ecx - 4.0;
        let mut y = ecy - 1.0;
        let glowc: Rgb = if progress > 0.85 { (255.0, 240.0, 170.0) } else { (60.0, 40.0, 40.0) };
        for k in 0..n {
            let nx = x + 1.0 + (k % 2) as f32;
            let dy = if k % 3 != 0 { 1.0 } else { -1.0 };
            let ny = y + dy;
            plot(l, nx, ny, glowc, 1.0);
            plot(l, nx, ny + 1.0, glowc, 0.8);
            x = nx;
            y = ny;
        }
    }
    (ecx, ecy)
}

// ── backgrounds ──

pub fn bg_ocean(sc: &mut Layer, t: f32) {
    vgrad(sc, 0, PHI, (22.0, 92.0, 156.0), (3.0, 10.0, 38.0));
    // light rays
    for i in 0..5 {
        let x0 = 4.0 + i as f32 * 10.0 + (t * 0.3 + i as f32).sin() * 2.0;
        for y in 0..22 {
            let x = x0 + y as f32 * 0.5;
            for dx in 0..3 {
                sc.put(x as i32 + dx, y, (200.0, 240.0, 255.0), 0.07 * (1.0 - y as f32 / 22.0));
            }
        }
    }
    // rising bubbles
    for i in 0..8 {
        let ph = (t * 0.35 + i as f32 * 0.137 * 7.0) % 1.0;
        let y = 27.0 - ph * 30.0;
        let x = 3.0 + ((i * 37) % 44) as f32 + (t * 0.9 + i as f32).sin() * 1.3;
        let r = 1.0 + (i % 3) as f32 * 0.4;
        disc(sc, x, y, r, (190.0, 230.0, 255.0), 0.35);
        plot(sc, x - 0.3, y - 0.3, (255.0, 255.0, 255.0), 0.7);
    }
    // sandy sea floor
    for x in 0..PWI {
        let top = 24.5 + (x as f32 * 0.45).sin() * 0.8;
        for y in (top as i32)..PHI {
            sc.put(x, y, mix((70.0, 80.0, 110.0), (22.0, 26.0, 48.0), (y as f32 - top) / 4.0), 1.0);
        }
    }
    // swaying seaweed and corals
    for (bx, col) in [(5.0_f32, (255.0_f32, 110.0_f32, 150.0_f32)), (44.0, (255.0, 160.0, 90.0))] {
        for k in 0..6 {
            let x = bx + (t * 1.3 + k as f32 * 0.6 + bx).sin() * (0.4 + k as f32 * 0.15);
            plot(sc, x, 24.0 - k as f32, (40.0, 170.0, 110.0), 1.0);
            plot(sc, x + 1.0, 24.0 - k as f32, (30.0, 140.0, 95.0), 1.0);
        }
        disc(sc, bx, 20.5, 1.4, col, 0.9);
    }
    // drifting plankton
    for i in 0..14u32 {
        let x = (hashf(i * 7) * PWF + t * (1.0 + hashf(i)) * 0.6) % PWF;
        let y = hashf(i * 13 + 5) * 22.0;
        plot(sc, x, y, (180.0, 230.0, 230.0), 0.25 + 0.2 * (t * 2.0 + i as f32).sin());
    }
}

pub fn bg_pond(sc: &mut Layer, t: f32) {
    vgrad(sc, 0, 15, (34.0, 22.0, 84.0), (240.0, 150.0, 120.0));
    vgrad(sc, 15, PHI, (36.0, 66.0, 120.0), (8.0, 24.0, 54.0));
    disc(sc, 41.0, 6.0, 3.2, (252.0, 246.0, 222.0), 1.0);
    glow(sc, 41.0, 6.0, 11.0, (255.0, 240.0, 200.0), 0.28);
    for i in 0..28u32 {
        plot(
            sc,
            hashf(i * 3 + 1) * PWF,
            hashf(i * 5 + 2) * 9.0,
            (255.0, 255.0, 255.0),
            0.35 + 0.35 * (t * 1.5 + i as f32).sin(),
        );
    }
    // water ripples and moon reflection
    for y in (16..PHI).step_by(2) {
        for x in 0..PWI {
            if (x as f32 * 0.7 + y as f32 * 1.3 + t * 1.2).sin() > 0.82 {
                sc.put(x, y, (190.0, 210.0, 255.0), 0.22);
            }
        }
    }
    for k in 0..10 {
        plot(
            sc,
            41.0 + (t * 2.0 + k as f32).sin() * (1.0 + k as f32 * 0.15),
            17.0 + k as f32,
            (255.0, 240.0, 200.0),
            0.22,
        );
    }
    // reeds with cattails
    for (bx, lean) in [(3.0_f32, 1.0_f32), (7.0, -1.0), (45.0, -1.0), (47.0, 1.0)] {
        for k in 0..14 {
            let x = bx + lean * (t * 0.9 + bx).sin() * k as f32 * 0.08;
            plot(sc, x, 24.0 - k as f32, (24.0, 70.0, 56.0), 1.0);
        }
        for k in 0..3 {
            plot(sc, bx + lean * 0.2, 11.0 + k as f32, (110.0, 60.0, 40.0), 1.0);
        }
    }
    // lily pad + flower
    ellipse(sc, 25.0, 24.6, 13.5, 3.2, (60.0, 150.0, 80.0), 1.0, false, 1.0);
    ellipse(sc, 25.0, 24.2, 12.0, 2.5, (84.0, 176.0, 98.0), 1.0, false, 1.0);
    for k in 0..4 {
        sc.put(38 + k, 24 + k / 2, (36.0, 66.0, 120.0), 1.0);
    }
    ball(sc, 39.5, 22.5, 1.7, (255.0, 150.0, 190.0));
    // fireflies
    for i in 0..5 {
        let fi = i as f32;
        let fx = 25.0 + (t * 0.6 + fi * 1.7).sin() * (12.0 + fi * 1.5);
        let fy = 12.0 + (t * 0.8 + fi * 2.3).cos() * 7.0;
        let b = 0.5 + 0.5 * (t * 2.5 + fi * 1.9).sin();
        glow(sc, fx, fy, 3.2, (255.0, 240.0, 100.0), 0.5 * b);
        plot(sc, fx, fy, (255.0, 255.0, 180.0), 0.5 + 0.5 * b);
    }
}

pub fn bg_forest(sc: &mut Layer, t: f32) {
    vgrad(sc, 0, 23, (8.0, 10.0, 44.0), (52.0, 36.0, 92.0));
    for i in 0..34u32 {
        plot(
            sc,
            hashf(i * 3 + 7) * PWF,
            hashf(i * 5 + 3) * 14.0,
            (255.0, 255.0, 255.0),
            0.3 + 0.5 * (0.5 + 0.5 * (t * 1.7 + i as f32 * 2.1).sin()),
        );
    }
    ball(sc, 8.0, 6.0, 3.6, (250.0, 245.0, 220.0));
    glow(sc, 8.0, 6.0, 12.0, (240.0, 240.0, 255.0), 0.25);
    // pine silhouettes
    for (px, h_, col) in [
        (3.0_f32, 16, (18.0_f32, 38.0_f32, 62.0_f32)),
        (13.0, 12, (18.0, 38.0, 62.0)),
        (36.0, 13, (18.0, 38.0, 62.0)),
        (46.0, 17, (14.0, 30.0, 52.0)),
        (24.0, 9, (26.0, 50.0, 76.0)),
    ] {
        for k in 0..h_ {
            let half = (k as f32 / h_ as f32) * 5.0 + 0.5;
            let x0 = (px - half) as i32;
            let x1 = (px + half) as i32;
            for x in x0..=x1 {
                sc.put(x, 22 - h_ + k + 1, col, 1.0);
            }
        }
    }
    // grass
    for y in 22..PHI {
        let f = (y - 22) as f32 / 5.0;
        for x in 0..PWI {
            sc.put(x, y, mix((40.0, 96.0, 60.0), (16.0, 44.0, 34.0), f), 1.0);
        }
    }
    for i in 0..16u32 {
        let gx = (hashf(i * 11) * PWF) as i32;
        sc.put(gx, 22, (60.0, 130.0, 80.0), 1.0);
        sc.put(gx + 1, 21, (60.0, 130.0, 80.0), 1.0);
    }
    // fireflies
    for i in 0..5 {
        let fi = i as f32;
        let fx = 25.0 + (t * 0.5 + fi * 1.7).sin() * (14.0 + fi);
        let fy = 14.0 + (t * 0.7 + fi * 2.3).cos() * 6.0;
        let b = 0.5 + 0.5 * (t * 2.2 + fi * 1.3).sin();
        glow(sc, fx, fy, 2.8, (255.0, 230.0, 90.0), 0.45 * b);
        plot(sc, fx, fy, (255.0, 255.0, 190.0), 0.5 + 0.5 * b);
    }
    // falling leaves
    for i in 0..5 {
        let fi = i as f32;
        let ph = (t * 0.25 + fi * 0.21) % 1.0;
        plot(sc, 6.0 + fi * 9.0 + (t + fi).sin() * 2.0, ph * 24.0, (230.0, 120.0, 40.0), 0.9);
    }
}

pub fn bg_cosmic(sc: &mut Layer, t: f32) {
    vgrad(sc, 0, PHI, (8.0, 4.0, 28.0), (44.0, 10.0, 62.0));
    // nebula clouds
    for y in 0..PHI {
        for x in 0..PWI {
            let xf = x as f32;
            let yf = y as f32;
            let n = (xf * 0.30 + t * 0.25).sin() * (yf * 0.38 - t * 0.2).sin() + ((xf + yf) * 0.17 + t * 0.15).sin();
            if n > 0.2 {
                sc.put(x, y, (140.0, 50.0, 190.0), ((n - 0.2) * 0.25).min(0.30));
            }
            if n < -0.7 {
                sc.put(x, y, (40.0, 170.0, 190.0), ((-n - 0.7) * 0.25).min(0.22));
            }
        }
    }
    // twinkling stars
    for i in 0..45u32 {
        plot(
            sc,
            hashf(i * 3 + 1) * PWF,
            hashf(i * 5 + 2) * 28.0,
            (255.0, 255.0, 255.0),
            0.25 + 0.6 * (0.5 + 0.5 * (t * 2.0 + i as f32 * 3.1).sin()),
        );
    }
    // shooting star
    let ph = (t * 0.17) % 1.0;
    if ph < 0.35 {
        let q = ph / 0.35;
        for k in 0..8 {
            plot(
                sc,
                44.0 - q * 40.0 - k as f32 * 0.9,
                2.0 + q * 14.0 + k as f32 * 0.55,
                (255.0, 255.0, 255.0),
                0.8 - k as f32 * 0.1,
            );
        }
    }
    // crystal platform
    ellipse(sc, 25.0, 24.8, 12.5, 2.6, (150.0, 110.0, 230.0), 0.85, true, 1.0);
    for i in 0..6 {
        let fi = i as f32;
        plot(
            sc,
            15.0 + fi * 4.0 + (t + fi).sin() * 0.8,
            24.0 + (i % 2) as f32,
            (230.0, 220.0, 255.0),
            0.5 + 0.4 * (t * 3.0 + fi).sin(),
        );
    }
}

pub fn bg_arctic(sc: &mut Layer, t: f32) {
    vgrad(sc, 0, 23, (6.0, 12.0, 42.0), (34.0, 70.0, 120.0));
    for i in 0..30u32 {
        plot(
            sc,
            hashf(i * 3 + 5) * PWF,
            hashf(i * 5 + 1) * 12.0,
            (255.0, 255.0, 255.0),
            0.3 + 0.5 * (0.5 + 0.5 * (t * 1.6 + i as f32 * 2.3).sin()),
        );
    }
    // aurora ribbons
    for x in 0..PWI {
        let xf = x as f32;
        let yc = 7.0 + (xf * 0.16 + t * 0.4).sin() * 3.0 + (xf * 0.07 - t * 0.25).sin() * 2.0;
        let col = mix((70.0, 255.0, 170.0), (170.0, 100.0, 255.0), xf / PWF);
        for k in 0..10 {
            sc.put(
                x,
                yc as i32 + k - 3,
                col,
                0.30 * (1.0 - k as f32 / 10.0) * (0.6 + 0.4 * (xf * 0.3 + t).sin()),
            );
        }
    }
    // far icebergs
    for (bx, h_) in [(6.0_f32, 7), (20.0, 4), (38.0, 8), (46.0, 5)] {
        for k in 0..h_ {
            let half = (k as f32 / h_ as f32) * 5.0 + 0.5;
            let x0 = (bx - half) as i32;
            let x1 = (bx + half) as i32;
            let c = mix((150.0, 190.0, 230.0), (90.0, 130.0, 190.0), k as f32 / h_ as f32);
            for x in x0..=x1 {
                sc.put(x, 22 - h_ + k + 1, c, 1.0);
            }
        }
    }
    // snowy ground
    for y in 22..PHI {
        let f = (y - 22) as f32 / 5.0;
        for x in 0..PWI {
            let bump = (x as f32 * 0.3).sin() * 0.6;
            if y as f32 >= 22.0 + bump {
                sc.put(x, y, mix((228.0, 240.0, 255.0), (150.0, 180.0, 218.0), f), 1.0);
            }
        }
    }
    // falling snow
    for i in 0..22u32 {
        let ph = (t * (0.12 + hashf(i) * 0.1) + hashf(i * 7)) % 1.0;
        let x = (hashf(i * 11) * PWF + (t * 0.8 + i as f32).sin() * 2.0).rem_euclid(PWF);
        plot(sc, x, ph * 26.0, (255.0, 255.0, 255.0), 0.85);
    }
}

// ── particles / effects ──

pub fn effects(sc: &mut Layer, mood: Mood, reaction: Reaction, cx: f32, top_in: f32, ground: f32, t: f32) {
    let top = top_in.max(7.0);

    if reaction == Reaction::Loved || mood == Mood::Ecstatic {
        for i in 0..4 {
            let fi = i as f32;
            let ph = (t * 0.5 + fi * 0.25) % 1.0;
            let x = cx - 9.0 + fi * 6.0 + (t * 2.0 + fi).sin() * 1.5;
            pattern(sc, x as i32, (top + 6.0 - ph * 8.0) as i32, &HEART, false, 1.0 - ph * 0.7);
        }
    }
    if mood == Mood::Sleeping {
        for i in 0..3 {
            let fi = i as f32;
            let ph = (t * 0.3 + fi * 0.33) % 1.0;
            let rows: &[&str] = if i % 2 == 0 { &ZEE_A } else { &ZEE_B };
            pattern(
                sc,
                (cx + 8.0 + ph * 5.0 + fi * 1.5) as i32,
                (top + 6.0 - ph * 8.0) as i32,
                rows,
                false,
                1.0 - ph * 0.7,
            );
        }
    }
    if mood == Mood::Sad || mood == Mood::Lonely {
        let cyp = top - 2.0;
        for (ox, oy, r) in [(-3.0_f32, 0.0_f32, 2.6_f32), (0.0, -1.0, 3.2), (3.2, 0.0, 2.5), (0.5, 1.0, 3.0)] {
            ellipse(sc, cx + ox, cyp + oy, r, r, (130.0, 140.0, 170.0), 0.95, true, 1.0);
        }
        for i in 0..5 {
            let fi = i as f32;
            let ph = (t * 1.3 + fi * 0.2) % 1.0;
            plot(sc, cx - 4.0 + fi * 2.0, cyp + 3.0 + ph * 8.0, (140.0, 190.0, 255.0), 1.0 - ph * 0.6);
        }
    }
    if mood == Mood::Grumpy {
        for i in 0..3 {
            let fi = i as f32;
            let ph = (t * 0.5 + fi * 0.33) % 1.0;
            disc(
                sc,
                cx - 3.0 + fi * 3.0 + (t * 3.0 + fi).sin(),
                top - 1.0 - ph * 7.0,
                1.0 + ph * 1.4,
                (210.0, 210.0, 220.0),
                0.55 * (1.0 - ph),
            );
        }
        pattern(sc, (cx + 7.0) as i32, (top + 1.0) as i32, &ANGER, false, 1.0);
    }
    if mood == Mood::Hungry && (t * 2.0) as i32 % 2 == 0 {
        pattern(sc, (cx + 9.0) as i32, (top + 1.0) as i32, &EXCLAIM, false, 1.0);
    }
    if mood == Mood::Sick {
        for i in 0..4 {
            let fi = i as f32;
            let ph = (t * 0.4 + fi * 0.25) % 1.0;
            disc(
                sc,
                cx - 6.0 + fi * 4.0 + (t + fi).sin() * 1.5,
                top + 6.0 - ph * 12.0,
                1.1,
                (150.0, 230.0, 120.0),
                0.6 * (1.0 - ph),
            );
        }
    }
    if reaction == Reaction::Playing {
        let bx = cx + (t * 3.0).sin() * 11.0;
        let by = ground - 2.5 - (t * 6.0).sin().abs() * 10.0;
        ball(sc, bx, by, 2.4, (240.0, 70.0, 80.0));
        plot(sc, bx - 1.0, by - 1.0, (255.0, 255.0, 255.0), 0.9);
    }
    if reaction == Reaction::Washing {
        for i in 0..8 {
            let fi = i as f32;
            let ph = (t * 0.8 + fi * 0.125) % 1.0;
            let x = cx - 10.0 + fi * 3.0 + (t * 2.0 + fi).sin() * 1.5;
            let y = ground - 3.0 - ph * 18.0;
            disc(sc, x, y, 1.2 + (i % 3) as f32 * 0.5, (210.0, 240.0, 255.0), 0.45);
            plot(sc, x - 0.4, y - 0.4, (255.0, 255.0, 255.0), 0.9);
        }
    }
    if reaction == Reaction::Listening {
        // thought bubble with animated dots ("...")
        let bx = cx + 12.0;
        let by = top + 2.0;
        ball(sc, bx - 5.0, by + 6.0, 0.9, (235.0, 240.0, 255.0));
        ball(sc, bx - 3.0, by + 4.0, 1.4, (235.0, 240.0, 255.0));
        ellipse(sc, bx, by, 5.0, 3.4, (240.0, 244.0, 255.0), 1.0, true, 1.0);
        let n = 1 + (t * 2.0) as i32 % 3;
        for i in 0..n {
            plot(sc, bx - 2.5 + i as f32 * 2.5, by, (70.0, 80.0, 110.0), 1.0);
        }
    }
    if reaction == Reaction::Eureka {
        // light bulb, rays and confetti
        let bx = cx;
        let by = top.max(8.0) - 3.0 + (t * 6.0).sin() * 0.8;
        glow(sc, bx, by + 3.0, 9.0, (255.0, 240.0, 120.0), 0.35 + 0.15 * (t * 8.0).sin());
        pattern(sc, (bx - 2.0) as i32, (by - 2.0) as i32, &BULB, false, 1.0);
        for i in 0..8 {
            let a = i as f32 * PI / 4.0 + t;
            plot(sc, bx + a.cos() * 6.0, by + 1.0 + a.sin() * 5.0, (255.0, 240.0, 150.0), 0.9);
        }
        for i in 0..10 {
            let fi = i as f32;
            let ph = (t * 0.9 + fi * 0.1) % 1.0;
            let col = hsv(fi * 0.1 + t * 0.2, 0.7, 1.0);
            plot(sc, cx - 14.0 + fi * 3.0 + (fi * 3.0 + t * 2.0).sin() * 2.0, 2.0 + ph * 20.0, col, 1.0 - ph * 0.5);
        }
    }
    if reaction == Reaction::Medicine {
        pattern(sc, (cx - 1.0) as i32, (top - 4.0 - (t * 5.0).sin()) as i32, &CROSS, false, 1.0);
    }
    if reaction == Reaction::Eating {
        for i in 0..6 {
            let fi = i as f32;
            let ph = (t * 3.0 + fi * 0.17) % 1.0;
            plot(
                sc,
                cx - 3.0 + fi * 1.3 + (fi * 5.0).sin() * 2.0,
                ground - 10.0 - ph * 4.0 + ph * ph * 8.0,
                (255.0, 200.0, 90.0),
                1.0 - ph,
            );
        }
    }
    if (mood == Mood::Happy || mood == Mood::Ecstatic) && reaction == Reaction::None {
        let count = if mood == Mood::Happy { 3 } else { 6 };
        for i in 0..count {
            let fi = i as f32;
            let ph = (t * 0.7 + fi * 0.31) % 1.0;
            let x = cx + (fi * 2.4 + t).cos() * 12.0;
            let y = top + 10.0 + (fi * 1.7 + t * 1.3).sin() * 7.0;
            pattern(sc, x as i32, y as i32, &SPARK, false, (1.0 - (ph - 0.5).abs() * 2.0) * 0.9);
        }
    }
}

/// Mood color grading over the whole scene.
pub fn grade(sc: &mut Layer, mood: Mood) {
    for i in 0..PW * PH {
        let (mut r, mut g, mut b) = sc.c[i];
        match mood {
            Mood::Sleeping => {
                r *= 0.62;
                g *= 0.66;
                b = b * 0.85 + 10.0;
            }
            Mood::Sick => {
                let m = (r + g + b) / 3.0;
                r = r * 0.5 + m * 0.2;
                g = g * 0.5 + m * 0.4 + 12.0;
                b = b * 0.5 + m * 0.2;
            }
            Mood::Sad | Mood::Lonely => {
                let m = (r + g + b) / 3.0;
                r = r * 0.6 + m * 0.4;
                g = g * 0.6 + m * 0.4;
                b = b * 0.6 + m * 0.4 + 6.0;
            }
            Mood::Ecstatic => {
                r = r * 1.1 + 6.0;
                g = g * 1.1 + 6.0;
                b = b * 1.1 + 6.0;
            }
            _ => {}
        }
        sc.c[i] = (r.clamp(0.0, 255.0), g.clamp(0.0, 255.0), b.clamp(0.0, 255.0));
    }
}

/// Renders the complete scene (background + creature + effects) for the current state.
pub fn render_scene(species: Species, stage: Stage, mood: Mood, reaction: Reaction, t: f32, progress: f32) -> Layer {
    let mut sc = Layer::new();
    match species {
        Species::Jellyfish => bg_ocean(&mut sc, t),
        Species::Frog => bg_pond(&mut sc, t),
        Species::Fox => bg_forest(&mut sc, t),
        Species::Glimmi => bg_cosmic(&mut sc, t),
        Species::Penguin => bg_arctic(&mut sc, t),
    }
    let mut body = Layer::new();
    let cx = 25.0_f32;
    let mut gy = 24.0_f32;
    let mut s = if stage == Stage::Adult { 1.0_f32 } else { 0.62_f32 };
    if species == Species::Fox && stage == Stage::Adult {
        s = 0.84;
    }
    if species == Species::Penguin && stage == Stage::Adult {
        s = 0.92;
    }
    if species == Species::Fox || species == Species::Penguin {
        gy = 25.0;
    }
    let ex = expression(mood, reaction, t);
    let outc: Rgb = (14.0, 10.0, 30.0);
    let top: f32;
    let mut ground = gy;
    let mut hue = -1.0_f32;

    if stage == Stage::Egg {
        let (ecx, ecy) = draw_egg(&mut body, cx, gy, t, progress, species);
        outline(&mut body, outc);
        if progress > 0.5 {
            glow(&mut sc, ecx, ecy, 12.0, (255.0, 240.0, 170.0), 0.12 + 0.25 * (progress - 0.5));
        }
        composite(&mut sc, &body);
        return sc;
    }

    let bounce = if reaction == Reaction::Playing { (t * 6.0).sin().abs() * 5.0 } else { 0.0 };
    match species {
        Species::Jellyfish => {
            let mut cy = 12.5 + (t * 1.2).sin() * 1.5;
            if stage == Stage::Baby {
                cy = 14.0 + (t * 1.2).sin() * 1.2;
            }
            glow(&mut sc, cx, cy, 15.0 * s, (170.0, 150.0, 255.0), 0.28);
            draw_jelly(&mut body, cx, cy, s, t, ex, mood);
            let oc: Rgb = if mood == Mood::Sleeping { (90.0, 130.0, 200.0) } else { (150.0, 215.0, 255.0) };
            outline(&mut body, oc);
            top = cy - 10.0 * s;
            ground = 26.0;
        }
        Species::Frog => {
            draw_frog(&mut body, cx, gy - bounce, s, t, ex, mood);
            outline(&mut body, outc);
            top = gy - 21.0 * s - bounce;
        }
        Species::Fox => {
            draw_fox(&mut body, cx, gy - bounce, s, t, ex, mood);
            outline(&mut body, outc);
            top = gy - 28.0 * s - bounce;
        }
        Species::Penguin => {
            draw_penguin(&mut body, cx, gy - bounce, s, t, ex, mood, stage == Stage::Baby);
            outline(&mut body, outc);
            let head_top = if stage == Stage::Adult { 23.5 * s } else { 16.0 };
            top = gy - head_top - bounce;
        }
        Species::Glimmi => {
            let mut cy = 13.0 + (t * 1.5).sin() * 1.5;
            if stage == Stage::Baby {
                cy = 16.0 + (t * 1.5).sin() * 1.2;
            }
            if mood == Mood::Ecstatic {
                hue = (t * 0.25) % 1.0;
            }
            let gc: Rgb = if hue >= 0.0 { hsv(hue, 0.5, 1.0) } else { (200.0, 140.0, 255.0) };
            glow(&mut sc, cx, cy, 17.0 * s, gc, 0.35);
            draw_glimmi(&mut body, cx, cy, s, t, ex, mood, hue);
            outline(&mut body, outc);
            top = cy - 17.0 * s;
            ground = 25.0;
            for i in 0..4 {
                let a = t * 1.3 + i as f32 * PI / 2.0;
                let ox = cx + a.cos() * 15.0 * s;
                let oy = cy + a.sin() * 9.0 * s;
                plot(&mut sc, ox, oy, (255.0, 255.0, 200.0), 0.95);
                plot(&mut sc, ox + 1.0, oy, (255.0, 255.0, 200.0), 0.5);
                plot(&mut sc, ox - 1.0, oy, (255.0, 255.0, 200.0), 0.5);
            }
        }
    }

    // soft ground shadow for the land animals
    if species == Species::Frog || species == Species::Fox || species == Species::Penguin {
        let x0 = (cx - 10.0 * s) as i32;
        let x1 = (cx + 10.0 * s) as i32;
        for y in (gy as i32)..(gy as i32 + 2) {
            for x in x0..x1 {
                let u = (x as f32 + 0.5 - cx) / (10.0 * s);
                if u.abs() < 1.0 {
                    sc.put(x, y, (0.0, 0.0, 0.0), 0.25 * (1.0 - u.abs()));
                }
            }
        }
    }

    composite(&mut sc, &body);
    effects(&mut sc, mood, reaction, cx, top, ground, t);
    grade(&mut sc, mood);
    sc
}

// ── turning pixels into terminal text ──

pub fn rgb_u8(c: Rgb) -> (u8, u8, u8) {
    (
        c.0.clamp(0.0, 255.0) as u8,
        c.1.clamp(0.0, 255.0) as u8,
        c.2.clamp(0.0, 255.0) as u8,
    )
}

/// Nearest color in the xterm 256-color palette (6x6x6 cube or grayscale ramp).
pub fn rgb256(c: (u8, u8, u8)) -> u8 {
    const LV: [i32; 6] = [0, 95, 135, 175, 215, 255];
    let r = c.0 as i32;
    let g = c.1 as i32;
    let b = c.2 as i32;
    let idx = |v: i32| -> usize {
        if v < 48 {
            0
        } else if v < 115 {
            1
        } else {
            (((v - 35) / 40) as usize).min(5)
        }
    };
    let (ri_, gi_, bi_) = (idx(r), idx(g), idx(b));
    let cube = 16 + 36 * ri_ + 6 * gi_ + bi_;
    let (cr, cg, cb) = (LV[ri_], LV[gi_], LV[bi_]);
    let d_cube = (cr - r).pow(2) + (cg - g).pow(2) + (cb - b).pow(2);
    let avg = (r + g + b) / 3;
    let gl = ((avg - 8).max(0) / 10).min(23);
    let gv = 8 + 10 * gl;
    let d_gray = (gv - r).pow(2) + (gv - g).pow(2) + (gv - b).pow(2);
    if d_gray < d_cube {
        (232 + gl) as u8
    } else {
        cube as u8
    }
}

pub fn fg_u8(c: (u8, u8, u8)) -> String {
    if TRUECOLOR.load(Ordering::Relaxed) {
        format!("\x1b[38;2;{};{};{}m", c.0, c.1, c.2)
    } else {
        format!("\x1b[38;5;{}m", rgb256(c))
    }
}

pub fn bg_u8(c: (u8, u8, u8)) -> String {
    if TRUECOLOR.load(Ordering::Relaxed) {
        format!("\x1b[48;2;{};{};{}m", c.0, c.1, c.2)
    } else {
        format!("\x1b[48;5;{}m", rgb256(c))
    }
}

type Rgb8 = (u8, u8, u8);

/// Half-block rendering: PH pixel rows become PH/2 text rows.
pub fn canvas_lines(cv: &Layer) -> Vec<String> {
    let mut lines: Vec<String> = Vec::with_capacity(PH / 2);
    for ty in 0..PH / 2 {
        let mut line = String::new();
        let mut last: Option<(Rgb8, Rgb8)> = None;
        for x in 0..PW {
            let top = rgb_u8(cv.c[(ty * 2) * PW + x]);
            let bot = rgb_u8(cv.c[(ty * 2 + 1) * PW + x]);
            if last != Some((top, bot)) {
                line.push_str(&fg_u8(top));
                line.push_str(&bg_u8(bot));
                last = Some((top, bot));
            }
            line.push('▀');
        }
        line.push_str(RESET);
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bounding box (x0, y0, x1, y1) of everything that is clearly opaque.
    fn bbox(l: &Layer) -> (i32, i32, i32, i32) {
        let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for y in 0..PHI {
            for x in 0..PWI {
                if l.alpha(x, y) > 0.5 {
                    x0 = x0.min(x);
                    y0 = y0.min(y);
                    x1 = x1.max(x);
                    y1 = y1.max(y);
                }
            }
        }
        (x0, y0, x1, y1)
    }

    /// Draws a penguin exactly like `render_scene` does (same position, scale and outline).
    fn penguin_layer(young: bool, mood: Mood, reaction: Reaction, t: f32) -> Layer {
        let s = if young { 0.62 } else { 0.92 };
        let mut l = Layer::new();
        draw_penguin(&mut l, 25.0, 25.0, s, t, expression(mood, reaction, t), mood, young);
        outline(&mut l, (14.0, 10.0, 30.0));
        l
    }

    #[test]
    fn canvas_has_half_as_many_text_rows_as_pixel_rows() {
        let lines = canvas_lines(&Layer::new());
        assert_eq!(lines.len(), PH / 2);
        assert!(lines.iter().all(|l| l.matches('▀').count() == PW));
    }

    #[test]
    fn layout_constants_match_the_canvas() {
        // header + scene rows + message + two rows of key hints
        assert_eq!(ANSI_ROWS, 1 + PH / 2 + 3);
        // scene + one blank column + the 28-column side panel
        assert_eq!(SCREEN_W, PW + 1 + 28);
    }

    #[test]
    fn rgb256_maps_basic_colours() {
        assert_eq!(rgb256((0, 0, 0)), 16);
        assert_eq!(rgb256((255, 255, 255)), 231);
        assert_eq!(rgb256((255, 0, 0)), 196);
        assert!((232..=255).contains(&rgb256((128, 128, 128))), "mid grey uses the grey ramp");
    }

    #[test]
    fn hsv_produces_the_primary_colours() {
        let (r, g, b) = hsv(0.0, 1.0, 1.0);
        assert!(r > 254.0 && g < 1.0 && b < 1.0);
        let (r, g, b) = hsv(1.0 / 3.0, 1.0, 1.0);
        assert!(g > 254.0 && r < 1.0 && b < 1.0);
        let (r, g, b) = hsv(0.5, 0.0, 0.5);
        assert!((r - g).abs() < 1e-3 && (g - b).abs() < 1e-3, "no saturation = grey");
    }

    #[test]
    fn hashf_is_deterministic_and_stays_in_range() {
        for n in 0..1000 {
            let v = hashf(n);
            assert!((0.0..=1.0).contains(&v));
            assert!(v == hashf(n));
        }
    }

    #[test]
    fn mix_interpolates_and_clamps() {
        let a = (0.0, 0.0, 0.0);
        let b = (100.0, 200.0, 50.0);
        assert!(mix(a, b, 0.5) == (50.0, 100.0, 25.0));
        assert!(mix(a, b, 5.0) == b);
        assert!(mix(a, b, -5.0) == a);
    }

    #[test]
    fn layers_clip_at_the_edges_and_blend_alpha() {
        let mut l = Layer::new();
        l.put(-1, 0, (1.0, 1.0, 1.0), 1.0);
        l.put(0, PHI, (1.0, 1.0, 1.0), 1.0);
        l.put(PWI, 0, (1.0, 1.0, 1.0), 1.0);
        assert!(bbox(&l).0 == i32::MAX, "nothing outside the canvas is drawn");
        l.put(3, 3, (255.0, 0.0, 0.0), 0.5);
        assert!((l.alpha(3, 3) - 0.5).abs() < 1e-6);
        l.put(3, 3, (0.0, 0.0, 255.0), 1.0);
        assert!(l.alpha(3, 3) == 1.0);
    }

    #[test]
    fn adult_penguin_is_taller_than_wide_and_fits_the_canvas() {
        for mood in [Mood::Happy, Mood::Sleeping, Mood::Ecstatic] {
            for reaction in [Reaction::None, Reaction::Eating, Reaction::Loved] {
                for t in [0.0, 0.4, 1.3, 2.9] {
                    let l = penguin_layer(false, mood, reaction, t);
                    let (x0, y0, x1, y1) = bbox(&l);
                    let (w, h) = (x1 - x0 + 1, y1 - y0 + 1);
                    assert!(y0 >= 1, "head clipped at the top (y0 = {y0})");
                    assert!(y1 <= PHI - 2, "feet touch the bottom edge of the canvas (y1 = {y1})");
                    assert!(x0 >= 1 && x1 <= PWI - 2, "sideways clipping ({x0}..{x1})");
                    assert!(h * 10 >= w * 11, "a realistic penguin is taller than wide ({w}x{h})");
                }
            }
        }
    }

    #[test]
    fn penguin_chick_is_smaller_than_the_adult() {
        let (_, ay0, _, ay1) = bbox(&penguin_layer(false, Mood::Happy, Reaction::None, 1.0));
        let (cx0, cy0, cx1, cy1) = bbox(&penguin_layer(true, Mood::Happy, Reaction::None, 1.0));
        assert!(cy1 - cy0 < ay1 - ay0);
        assert!(cy0 >= 1 && cx0 >= 1 && cx1 <= PWI - 2);
    }
}
