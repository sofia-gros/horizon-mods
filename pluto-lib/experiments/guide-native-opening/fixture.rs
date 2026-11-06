// EXCLUDED std-only mechanics. The Game adapter is deliberately not included.
// Future execution must select native_open_tests only, preserving old fixtures.
#[allow(dead_code)]
mod retained_book {
    include!("../guide-response-phase/native-patch/retained_book.rs");
    include!("../guide-owned-opening/owned.rs");
    include!("native-open-storage.rs");

    #[cfg(test)]
    mod native_open_tests {
        use super::*;
        use std::{cell::RefCell, rc::Rc};
        struct Value { name: &'static str, events: Rc<RefCell<Vec<&'static str>>>, panic: bool }
        impl Drop for Value {
            fn drop(&mut self) {
                self.events.borrow_mut().push(self.name);
                if self.panic { self.panic=false; panic!("model old-value destructor"); }
            }
        }
        fn value(name: &'static str, events: &Rc<RefCell<Vec<&'static str>>>) -> Value {
            Value { name, events: events.clone(), panic: false }
        }
        #[test]
        fn identity_failure_returns_candidate_without_drop_or_slot_change() {
            let events=Rc::new(RefCell::new(vec![])); let mut slot=RetainedBook::default();
            slot.set(Some(value("old",&events))); let before=slot.observed_stamp();
            let exhausted=AtomicU64::new(u64::MAX);
            let candidate=install_native_owned_value(&mut slot,value("candidate",&events),&exhausted).err().unwrap();
            assert_eq!(candidate.name,"candidate"); assert_eq!(slot.observed_stamp(),before);
            assert_eq!(slot.as_ref().unwrap().name,"old"); assert!(events.borrow().is_empty());
            drop(candidate); assert_eq!(&*events.borrow(), &["candidate"]);
        }
        #[test]
        fn concrete_assignment_drop_precedes_the_modeled_native_tail() {
            let events=Rc::new(RefCell::new(vec![])); let mut slot=RetainedBook::default();
            slot.set(Some(value("old",&events)));
            let owner=install_native_owned_value(&mut slot,value("new",&events),&NEXT).ok().unwrap();
            assert_eq!(&*events.borrow(), &["old"]); assert!(native_owner_is_current(&slot,&owner));
            events.borrow_mut().push("tail"); assert_eq!(&*events.borrow(), &["old","tail"]);
        }
        #[test]
        fn old_destructor_panic_never_leaves_an_observable_opening() {
            let events=Rc::new(RefCell::new(vec![])); let mut slot=RetainedBook::default();
            let mut old=value("old",&events); old.panic=true; slot.set(Some(old));
            let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _=install_native_owned_value(&mut slot,value("new",&events),&NEXT);
            }));
            assert!(result.is_err()); assert!(slot.observed_stamp().is_none());
            // No assertion of candidate return or rollback after assignment began.
        }
        #[test]
        fn page_revision_is_not_opening_replacement_but_reset_is() {
            let mut slot=RetainedBook::default();
            let owner=install_native_owned_value(&mut slot,0,&NEXT).ok().unwrap();
            *slot.as_mut().unwrap()=1; assert!(native_owner_is_current(&slot,&owner));
            slot.invalidate_attachment(); assert!(!native_owner_is_current(&slot,&owner));
            assert_eq!(slot.as_ref(),Some(&1));
        }
        #[test]
        fn identical_reopen_or_close_in_a_modeled_tail_revokes_owner() {
            let mut slot=RetainedBook::default();
            let owner=install_native_owned_value(&mut slot,"same",&NEXT).ok().unwrap();
            slot.set(Some("same")); assert!(!native_owner_is_current(&slot,&owner));
            assert_eq!(slot.as_ref(),Some(&"same"));
            let current=install_native_owned_value(&mut slot,"same",&NEXT).ok().unwrap();
            slot.take(); assert!(!native_owner_is_current(&slot,&current));
        }
        #[test]
        fn owner_cannot_cross_slots_or_survive_revision_exhaustion() {
            let mut a=RetainedBook::default(); let mut b=RetainedBook::default();
            let owner=install_native_owned_value(&mut a,1,&NEXT).ok().unwrap();
            install_native_owned_value(&mut b,1,&NEXT).ok().unwrap();
            assert!(!native_owner_is_current(&b,&owner));
            a.revision=u64::MAX; let _=a.as_mut(); assert!(!native_owner_is_current(&a,&owner));
        }
    }
}
