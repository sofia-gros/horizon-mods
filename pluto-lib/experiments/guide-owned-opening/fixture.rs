// Proposed std-only fixture, not executed by adding this file.
mod retained_book {
    include!("../guide-response-phase/native-patch/retained_book.rs");
    include!("owned.rs");

    #[cfg(test)]
    mod ownership_tests {
        use super::*;

        fn install<T>(slot: &mut RetainedBook<T>, value: T) -> Installed<T> {
            match slot.try_install_owned(value) { Ok(x) => x, Err(_) => panic!("fixture ID exhaustion") }
        }

        #[test]
        fn identical_owned_reopen_revokes_old_owner() {
            let mut slot = RetainedBook::default();
            let first = install(&mut slot, "same");
            let next = install(&mut slot, "same");
            assert_eq!(next.previous, Some("same"));
            let stamp = slot.observed_stamp().unwrap();
            assert!(matches!(slot.prepare_owned_replace(&first.owner, stamp), Err(PrepareError::WrongOwner)));
            assert_eq!(slot.prepare_owned_replace(&next.owner, stamp).unwrap().commit("new"), "same");
        }

        #[test]
        fn native_close_and_identical_restore_cannot_claim_old_owner() {
            let mut slot = RetainedBook::default();
            let owned = install(&mut slot, "same");
            let saved = slot.take();
            assert!(slot.observed_stamp().is_none());
            slot.set(saved);
            let stamp = slot.observed_stamp().unwrap();
            assert!(matches!(slot.prepare_owned_replace(&owned.owner, stamp), Err(PrepareError::WrongOwner)));
            assert_eq!(slot.as_ref(), Some(&"same"));
        }

        #[test]
        fn owner_from_other_slot_cannot_use_matching_contents() {
            let mut a = RetainedBook::default(); let mut b = RetainedBook::default();
            let owner = install(&mut a, "same").owner; b.set(Some("same"));
            let stamp = b.observed_stamp().unwrap();
            assert!(matches!(b.prepare_owned_replace(&owner, stamp), Err(PrepareError::WrongOwner)));
        }

        #[test]
        fn page_revision_rejects_old_observation_but_preserves_owner() {
            let mut slot = RetainedBook::default(); let owned = install(&mut slot, 1);
            let old = slot.observed_stamp().unwrap(); *slot.as_mut().unwrap() = 2;
            assert!(matches!(slot.prepare_owned_replace(&owned.owner, old), Err(PrepareError::StaleObservation)));
            let current = slot.observed_stamp().unwrap();
            assert_eq!(slot.prepare_owned_replace(&owned.owner, current).unwrap().commit(3), 2);
            assert_eq!(slot.observed_stamp().unwrap().revision, current.revision + 1);
        }

        #[test]
        fn attachment_reset_revokes_owner_without_removing_contents() {
            let mut slot = RetainedBook::default(); let owned = install(&mut slot, "retained");
            slot.invalidate_attachment(); let stamp = slot.observed_stamp().unwrap();
            assert!(matches!(slot.prepare_owned_replace(&owned.owner, stamp), Err(PrepareError::WrongOwner)));
            assert_eq!(slot.as_ref(), Some(&"retained"));
        }

        #[test]
        fn dropping_prepared_guard_does_not_mutate_storage() {
            let mut slot = RetainedBook::default(); let owned = install(&mut slot, "old");
            let stamp = slot.observed_stamp().unwrap();
            drop(slot.prepare_owned_replace(&owned.owner, stamp).unwrap());
            assert_eq!(slot.observed_stamp(), Some(stamp)); assert_eq!(slot.as_ref(), Some(&"old"));
        }

        #[test]
        fn exhausted_install_returns_exact_candidate_and_preserves_old_state() {
            let mut slot = RetainedBook::default(); let _owned = install(&mut slot, String::from("old"));
            let stamp = slot.observed_stamp(); let candidate = String::from("candidate"); let pointer = candidate.as_ptr();
            let exhausted = AtomicU64::new(u64::MAX);
            let returned = match slot.try_install_owned_using(candidate, &exhausted) {
                Err(value) => value, Ok(_) => panic!("exhausted allocator admitted installation"),
            };
            assert_eq!(returned.as_ptr(), pointer); assert_eq!(slot.observed_stamp(), stamp);
            assert_eq!(slot.as_ref().map(String::as_str), Some("old"));
        }

        #[test]
        fn revision_exhaustion_rejects_before_any_change() {
            let mut slot = RetainedBook::default(); let owned = install(&mut slot, 1); slot.revision = u64::MAX;
            let stamp = slot.observed_stamp().unwrap();
            assert!(matches!(slot.prepare_owned_replace(&owned.owner, stamp), Err(PrepareError::RevisionExhausted)));
            assert_eq!(slot.observed_stamp(), Some(stamp)); assert_eq!(slot.as_ref(), Some(&1));
        }

        #[test]
        fn old_value_drops_only_after_committed_state_is_observable() {
            use std::{cell::RefCell, rc::Rc};
            struct Value { label: &'static str, on_drop: Option<Box<dyn FnOnce()>> }
            impl Drop for Value { fn drop(&mut self) { if let Some(f) = self.on_drop.take() { f() } } }
            let shared = Rc::new(RefCell::new(RetainedBook::default())); let weak = Rc::downgrade(&shared);
            let owned = install(&mut shared.borrow_mut(), Value { label: "old", on_drop: Some(Box::new(move || {
                let shared = weak.upgrade().unwrap(); let mut slot = shared.borrow_mut();
                assert_eq!(slot.as_ref().unwrap().label, "new");
                slot.set(Some(Value { label: "later native change", on_drop: None }));
            })) });
            let old = {
                let mut slot = shared.borrow_mut(); let stamp = slot.observed_stamp().unwrap();
                slot.prepare_owned_replace(&owned.owner, stamp).unwrap().commit(Value { label: "new", on_drop: None })
            };
            assert_eq!(shared.borrow().as_ref().unwrap().label, "new");
            drop(old);
            assert_eq!(shared.borrow().as_ref().unwrap().label, "later native change");
            let stamp = shared.borrow().observed_stamp().unwrap();
            assert!(matches!(shared.borrow_mut().prepare_owned_replace(&owned.owner, stamp), Err(PrepareError::WrongOwner)));
        }

        #[test]
        fn destructor_panic_after_commit_does_not_restore_old_storage() {
            struct Value { label: &'static str, panic: bool }
            impl Drop for Value { fn drop(&mut self) { assert!(!self.panic, "old value destructor"); } }
            let mut slot = RetainedBook::default(); let owned = install(&mut slot, Value { label: "old", panic: true });
            let stamp = slot.observed_stamp().unwrap();
            let old = slot.prepare_owned_replace(&owned.owner, stamp).unwrap().commit(Value { label: "new", panic: false });
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(old)));
            assert!(outcome.is_err()); assert_eq!(slot.as_ref().unwrap().label, "new");
            assert_eq!(slot.observed_stamp().unwrap().revision, stamp.revision + 1);
        }
    }
}
