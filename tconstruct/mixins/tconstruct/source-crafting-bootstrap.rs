// Source-pinned initialization before either native Recipes constructor body.
// Two authorities, one startup result; this is not cross-registry rollback.
use crate::pluto_inventory::crafting::matching::RegistrationError as StoredError;
use super::pluto_derivations::RegistrationError as PairError;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InitializationError { Stored(StoredError), Pair(PairError) }
static REGISTERED: OnceLock<Result<(), InitializationError>> = OnceLock::new();

fn initialize(
    stored: impl FnOnce() -> Result<(), StoredError>,
    pair: impl FnOnce() -> Result<(), PairError>,
) -> Result<(), InitializationError> {
    stored().map_err(InitializationError::Stored)?;
    pair().map_err(InitializationError::Pair)
}

fn require_registration(
    state: &OnceLock<Result<(), InitializationError>>,
    stored: impl FnOnce() -> Result<(), StoredError>,
    pair: impl FnOnce() -> Result<(), PairError>,
) {
    if let Err(error) = state.get_or_init(|| initialize(stored, pair)) {
        // Cache an ordinary Err before panic. A successful first registration
        // remains present if the second fails; guarded constructors never return
        // a new Recipes value after that failure. No rollback is advertised.
        panic!("TConstruct crafting initialization failed: {error:?}");
    }
}

pub(super) fn ensure() {
    require_registration(&REGISTERED,
        super::tconstruct_crafting_deny_policies::register,
        super::tconstruct_overslime_derivation::register);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
    #[test]
    fn both_registrations_complete_once_before_repeated_constructor_bodies() {
        let state = OnceLock::new();
        let first = AtomicUsize::new(0); let second = AtomicUsize::new(0);
        for _ in 0..4 {
            require_registration(&state,
                || { first.fetch_add(1, Ordering::SeqCst); Ok(()) },
                || { assert_eq!(first.load(Ordering::SeqCst), 1); second.fetch_add(1, Ordering::SeqCst); Ok(()) });
            assert_eq!(second.load(Ordering::SeqCst), 1);
        }
        assert_eq!(state.get(), Some(&Ok(())));
    }
    #[test]
    fn stored_failure_is_cached_and_pair_is_never_attempted() {
        let state = OnceLock::new(); let first = AtomicUsize::new(0); let second = AtomicUsize::new(0);
        for _ in 0..2 {
            assert!(std::panic::catch_unwind(|| require_registration(&state,
                || { first.fetch_add(1, Ordering::SeqCst); Err(StoredError::DuplicateId) },
                || { second.fetch_add(1, Ordering::SeqCst); Ok(()) })).is_err());
        }
        assert_eq!(first.load(Ordering::SeqCst), 1); assert_eq!(second.load(Ordering::SeqCst), 0);
        assert_eq!(state.get(), Some(&Err(InitializationError::Stored(StoredError::DuplicateId))));
    }
    #[test]
    fn pair_failure_keeps_first_effect_and_neither_constructor_returns() {
        let state = OnceLock::new(); let first = AtomicUsize::new(0); let second = AtomicUsize::new(0); let bodies = AtomicUsize::new(0);
        for _ in 0..2 {
            assert!(std::panic::catch_unwind(|| {
                require_registration(&state,
                    || { first.fetch_add(1, Ordering::SeqCst); Ok(()) },
                    || { second.fetch_add(1, Ordering::SeqCst); Err(PairError::DuplicateId) });
                bodies.fetch_add(1, Ordering::SeqCst);
            }).is_err());
        }
        assert_eq!(first.load(Ordering::SeqCst), 1); assert_eq!(second.load(Ordering::SeqCst), 1);
        assert_eq!(bodies.load(Ordering::SeqCst), 0);
        assert_eq!(state.get(), Some(&Err(InitializationError::Pair(PairError::DuplicateId))));
    }
    #[test]
    fn parallel_constructors_wait_for_both_static_registrations() {
        let state = Arc::new(OnceLock::new()); let first = Arc::new(AtomicUsize::new(0)); let second = Arc::new(AtomicUsize::new(0));
        std::thread::scope(|scope| {
            for _ in 0..8 {
                let (state, first, second) = (state.clone(), first.clone(), second.clone());
                scope.spawn(move || {
                    require_registration(&state,
                        || { first.fetch_add(1, Ordering::SeqCst); Ok(()) },
                        || { assert_eq!(first.load(Ordering::SeqCst), 1); second.fetch_add(1, Ordering::SeqCst); Ok(()) });
                    assert_eq!(second.load(Ordering::SeqCst), 1);
                });
            }
        });
        assert_eq!(first.load(Ordering::SeqCst), 1); assert_eq!(second.load(Ordering::SeqCst), 1);
    }
    #[test]
    fn unexpected_initializer_panic_does_not_promise_rollback_or_success() {
        let state = OnceLock::new(); let first = AtomicUsize::new(0);
        assert!(std::panic::catch_unwind(|| require_registration(&state,
            || { first.fetch_add(1, Ordering::SeqCst); Ok(()) }, || panic!("test pair initializer"))).is_err());
        assert!(state.get().is_none()); assert_eq!(first.load(Ordering::SeqCst), 1);
        // Model the now-occupied first ID. Retry fails instead of falsely
        // treating the prior partial initialization as completed startup.
        assert!(std::panic::catch_unwind(|| require_registration(&state,
            || Err(StoredError::DuplicateId), || Ok(()))).is_err());
        assert_eq!(state.get(), Some(&Err(InitializationError::Stored(StoredError::DuplicateId))));
    }
}
