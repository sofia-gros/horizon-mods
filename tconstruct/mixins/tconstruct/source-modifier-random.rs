// TC Modifier.RANDOM is a process-local java.util.Random, not the world's RNG.
// LegacyRandom supplies the same 48-bit generator and 24-bit float algorithm.
// This is a non-persistent TC stream; no claim of matching Java's startup seed
// or call interleaving with modifiers which have not migrated to this stream.
use horizon_core::random::{LegacyRandom, RandomSource};
use std::sync::{Mutex, OnceLock};

static RANDOM: OnceLock<Mutex<LegacyRandom>> = OnceLock::new();

pub(super) fn next_float() -> f32 {
    let random = RANDOM.get_or_init(|| {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as i64)
            .unwrap_or(0);
        Mutex::new(LegacyRandom::new(seed))
    });
    random.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).next_float()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_seed_uses_native_java_float_sequence() {
        let mut random = LegacyRandom::new(0);
        // java.util.Random(0): next(24) / 2^24.
        for numerator in [12_263_604_u32, 13_949_265, 4_035_531, 10_172_784] {
            assert_eq!(random.next_float(), numerator as f32 / 16_777_216.0);
        }
    }
}
