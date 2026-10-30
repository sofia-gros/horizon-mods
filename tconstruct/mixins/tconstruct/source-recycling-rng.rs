use horizon_core::random::{XoroshiroRandom, RandomSource};
use std::sync::{Mutex, OnceLock};

// Share the Rust generator between the in-frame host and LAN server.
static RANDOM: OnceLock<Mutex<XoroshiroRandom>> = OnceLock::new();

impl super::Items {
    pub fn __horizon_mixins_tconstruct_recycling_draw(&self, seed: impl FnOnce() -> i64, bound: usize) -> Option<usize> {
        let rng = RANDOM.get_or_init(|| Mutex::new(XoroshiroRandom::new(seed())));
        let mut rng = rng.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        rng.next_int_bound(i32::try_from(bound).ok()?).ok().map(|value| value as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_draws_stay_within_the_requested_range() {
        let mut rng = XoroshiroRandom::new(123456789);
        for bound in [1, 2, 3, 7, 16, 1000, 1073741825] {
            for _ in 0..100 {
                let value = rng.next_int_bound(bound).unwrap();
                assert!((0..bound).contains(&value));
            }
        }
    }
}
