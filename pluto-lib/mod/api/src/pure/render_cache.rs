//! Process-local registration generations, not saved block-entity revisions.
use std::sync::atomic::{AtomicU64, Ordering};

struct Generation(AtomicU64);
impl Generation {
    const fn new() -> Self { Self(AtomicU64::new(0)) }
    fn advance(&self) {
        let mut current = self.0.load(Ordering::Acquire);
        loop {
            let next = current.checked_add(1).expect("render registration generation exhausted");
            match self.0.compare_exchange_weak(current, next, Ordering::AcqRel, Ordering::Acquire) {
                Ok(_) => return,
                Err(seen) => current = seen,
            }
        }
    }
    fn get(&self) -> u64 { self.0.load(Ordering::Acquire) }
}
static GENERATION: Generation = Generation::new();
pub(crate) fn changed() { GENERATION.advance(); }
pub(crate) fn generation() -> Option<u64> {
    (super::block_entity_api_v1::registry_available()
        && super::block_entity_renderer_api_v1::registry_available()).then(|| GENERATION.get())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registry_updates_invalidate_global_generation() {
        // The registries are process-global. Run this exact test in its own
        // test process so unrelated parallel registrations cannot hide a missed
        // generation bump. No source copies or test-only production fields.
        if std::env::var_os("PLUTO_RENDER_EPOCH_TEST_CHILD").is_none() || !std::env::args().any(|arg| arg == "--exact") {
            let name = concat!(module_path!(), "::registry_updates_invalidate_global_generation");
            let filter = name.split_once("::").unwrap().1;
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", filter, "--test-threads=1"])
                .env("PLUTO_RENDER_EPOCH_TEST_CHILD", "1").output().unwrap();
            assert!(output.status.success() && String::from_utf8_lossy(&output.stdout).contains("1 passed"),
                "{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
            return;
        }
        use crate::pure::base::Identifier;
        use crate::pure::block_entity_api_v1::{BlockEntityType, BlockEntityTypeRegistry};
        use crate::pure::block_entity_renderer_api_v1::BlockEntityRendererRegistry;
        let block = Identifier::parse("test:registration_epoch").unwrap();
        let initial = generation().unwrap();
        BlockEntityTypeRegistry::register(BlockEntityType::new(block.clone()));
        assert_eq!(generation(), Some(initial + 1));
        BlockEntityTypeRegistry::register(BlockEntityType::new(block.clone()).client_ticker(|_| {}));
        assert_eq!(generation(), Some(initial + 2));
        BlockEntityRendererRegistry::register(block.clone(), |_| {});
        assert_eq!(generation(), Some(initial + 3));
        BlockEntityRendererRegistry::register(block, |_| {});
        assert_eq!(generation(), Some(initial + 4));
    }

    #[test]
    fn generations_are_stable_until_change_and_never_wrap_into_an_old_cache_key() {
        let generation = Generation::new();
        assert_eq!(generation.get(), 0);
        generation.advance();
        assert_eq!(generation.get(), 1);
        assert_eq!(generation.get(), 1);
        generation.advance();
        assert_eq!(generation.get(), 2);
        let exhausted = Generation(AtomicU64::new(u64::MAX));
        assert!(std::panic::catch_unwind(|| exhausted.advance()).is_err());
        assert_eq!(exhausted.get(), u64::MAX);
    }
}
