//! Tiny xorshift PRNG – standard library only.

use std::time::{SystemTime, UNIX_EPOCH};

pub struct Rng(u64);

impl Rng {
    pub fn new() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        let mut rng = Rng(seed | 1);
        for _ in 0..8 {
            rng.next();
        }
        rng
    }

    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    pub fn range(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    pub fn chance(&mut self, p: f64) -> bool {
        ((self.next() % 10_000) as f64 / 10_000.0) < p
    }

    pub fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.range(items.len())]
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_stays_in_bounds() {
        let mut r = Rng::new();
        for n in 1..50 {
            for _ in 0..200 {
                assert!(r.range(n) < n);
            }
        }
    }

    #[test]
    fn chance_handles_the_extremes() {
        let mut r = Rng::new();
        assert!((0..500).all(|_| !r.chance(0.0)));
        assert!((0..500).all(|_| r.chance(1.0)));
    }

    #[test]
    fn pick_returns_one_of_the_items() {
        let mut r = Rng::new();
        let items = ["a", "b", "c"];
        for _ in 0..100 {
            assert!(items.contains(&r.pick(&items)));
        }
    }
}
