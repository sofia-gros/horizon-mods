// Excluded candidate: installed only after constructor/composition proof.
use crate::pluto_inventory::crafting::matching::RegistrationError;
use std::sync::OnceLock;

static REGISTERED: OnceLock<Result<(), RegistrationError>> = OnceLock::new();

// The production initializer is one static registration, not user code. It must
// not invoke interest/callbacks or construct Recipes recursively.
fn require_registration(
    state: &OnceLock<Result<(), RegistrationError>>,
    register: impl FnOnce() -> Result<(), RegistrationError>,
) {
    if let Err(error) = state.get_or_init(register) {
        // Both entry points use this explicit initialization failure. In
        // particular, Recipes::empty returns Self and cannot return a load Err.
        // Store Err before panic, so catch_unwind cannot turn a collision into
        // a successful retry or silently construct an unprotected recipe table.
        panic!("TConstruct crafting policy registration failed: {error:?}");
    }
}

pub(super) fn ensure() {
    require_registration(&REGISTERED, super::tconstruct_crafting_deny_policies::register);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

    #[test]
    fn successful_initializer_is_shared_across_repeated_entries() {
        let state = OnceLock::new();
        let calls = AtomicUsize::new(0);
        for _ in 0..4 {
            require_registration(&state, || { calls.fetch_add(1, Ordering::SeqCst); Ok(()) });
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(state.get(), Some(&Ok(())));
    }
    #[test]
    fn cached_conflict_panics_before_each_modeled_constructor_body() {
        let state = OnceLock::new();
        let calls = AtomicUsize::new(0);
        let bodies = AtomicUsize::new(0);
        for _ in 0..2 {
            let failure = std::panic::catch_unwind(|| {
                require_registration(&state, || {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Err(RegistrationError::DuplicateId)
                });
                bodies.fetch_add(1, Ordering::SeqCst);
            });
            assert!(failure.is_err());
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(bodies.load(Ordering::SeqCst), 0);
        assert_eq!(state.get(), Some(&Err(RegistrationError::DuplicateId)));
    }
    #[test]
    fn parallel_entries_wait_for_one_completed_registration() {
        let state = Arc::new(OnceLock::new());
        let calls = Arc::new(AtomicUsize::new(0));
        let completed = Arc::new(AtomicUsize::new(0));
        std::thread::scope(|scope| {
            for _ in 0..8 {
                let (state, calls, completed) = (state.clone(), calls.clone(), completed.clone());
                scope.spawn(move || {
                    require_registration(&state, || {
                        calls.fetch_add(1, Ordering::SeqCst);
                        completed.store(1, Ordering::SeqCst);
                        Ok(())
                    });
                    assert_eq!(completed.load(Ordering::SeqCst), 1);
                });
            }
        });
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
    #[test]
    fn parallel_conflict_is_sticky_and_never_returns_success() {
        let state = Arc::new(OnceLock::new());
        let calls = Arc::new(AtomicUsize::new(0));
        std::thread::scope(|scope| {
            for _ in 0..8 {
                let (state, calls) = (state.clone(), calls.clone());
                scope.spawn(move || {
                    assert!(std::panic::catch_unwind(|| require_registration(&state, || {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Err(RegistrationError::EmptyId)
                    })).is_err());
                });
            }
        });
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
    #[test]
    fn unexpected_initializer_panic_is_not_misreported_as_cached_success() {
        let state = OnceLock::new();
        assert!(std::panic::catch_unwind(|| require_registration(&state, || panic!("test initializer"))).is_err());
        assert!(state.get().is_none());
        // std OnceLock is not poisoned by panic. This is not a rollback claim
        // for arbitrary registration side effects or allocation failure.
        require_registration(&state, || Ok(()));
        assert_eq!(state.get(), Some(&Ok(())));
    }
}
