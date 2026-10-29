use horizon_core::random::{LegacyRandom, RandomSource};
use std::sync::{Mutex, OnceLock};

// TConstruct.RANDOM is one Java Random per process. Sharing this in the
// horizon-items crate lets the in-frame host and LAN server use the same state.
static RANDOM: OnceLock<Mutex<LegacyRandom>> = OnceLock::new();

impl super::Items {
    pub fn __horizon_mixins_tconstruct_recycling_draw(&self, seed: impl FnOnce() -> i64) -> bool {
        let rng = RANDOM.get_or_init(|| Mutex::new(LegacyRandom::new(seed())));
        let mut rng = rng.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        rng.next_int_bound(1).ok() == Some(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_random_bounded_sequence_matches_java_oracle() {
        let mut rng = LegacyRandom::new(123456789);
        let bounds = [1, 2, 3, 7, 16, 1000, 1073741825];
        let expected = [0, 1, 2, 3, 6, 554, 913732807];
        for (bound, expected) in bounds.into_iter().zip(expected) {
            assert_eq!(rng.next_int_bound(bound), Ok(expected));
        }
    }
}
