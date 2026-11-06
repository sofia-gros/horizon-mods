// Isolated synchronous entry gate candidate. Not enabled in PlutoLib.
use std::cell::Cell;

thread_local! {
    static NEXT_NATIVE_ENTRY: Cell<bool> = const { Cell::new(false) };
}

/// Must be the first operation in the exclusive native HEAD adapter.
pub(super) fn take_native_entry() -> bool {
    NEXT_NATIVE_ENTRY.with(|pending| pending.replace(false))
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct AlreadyArmed;

/// Arm exactly the immediately following synchronous native entry. The callee
/// consumes the flag before any native body/callback. No stack/actor is stored.
/// Repeated arming fails without invoking the closure or altering the first arm.
pub(super) fn call_native<R>(call: impl FnOnce() -> R) -> Result<R, AlreadyArmed> {
    NEXT_NATIVE_ENTRY.with(|pending| {
        if pending.replace(true) {
            return Err(AlreadyArmed);
        }
        struct Reset<'a>(&'a Cell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) { self.0.set(false); }
        }
        let _reset = Reset(pending);
        Ok(call())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    #[test]
    fn unarmed_entry_has_no_bypass() {
        assert!(!take_native_entry());
    }

    #[test]
    fn immediate_entry_consumes_once_before_body_callbacks() {
        let calls = Cell::new(0);
        let result = call_native(|| {
            assert_eq!(calls.get(), 0);
            assert!(take_native_entry());
            assert!(!take_native_entry());
            calls.set(calls.get() + 1);
            37
        });
        assert_eq!(result, Ok(37));
        assert_eq!(calls.get(), 1);
        assert!(!take_native_entry());
    }

    #[test]
    fn nested_entries_are_independent_after_parent_consumption() {
        let mut order = Vec::new();
        call_native(|| {
            assert!(take_native_entry());
            order.push("parent-body");
            assert!(!take_native_entry()); // Child's ordinary HEAD.
            call_native(|| {
                assert!(take_native_entry());
                order.push("child-body");
                assert!(!take_native_entry());
            }).unwrap();
            order.push("parent-result");
        }).unwrap();
        assert_eq!(order, ["parent-body", "child-body", "parent-result"]);
        assert!(!take_native_entry());
    }

    #[test]
    fn duplicate_arm_never_invokes_second_callback() {
        call_native(|| {
            let result = call_native(|| panic!("second callback must not run"));
            assert_eq!(result, Err(AlreadyArmed));
            assert!(take_native_entry());
        }).unwrap();
        assert!(!take_native_entry());
    }

    #[test]
    fn return_without_entry_clears_pending_state() {
        assert_eq!(call_native(|| 5), Ok(5));
        assert!(!take_native_entry());
    }

    #[test]
    fn panic_before_consumption_clears_pending_state() {
        assert!(catch_unwind(|| {
            let _ = call_native(|| panic!("before entry"));
        }).is_err());
        assert!(!take_native_entry());
    }

    #[test]
    fn nested_panic_after_consumption_clears_both_scopes() {
        assert!(catch_unwind(AssertUnwindSafe(|| {
            call_native(|| {
                assert!(take_native_entry());
                call_native(|| {
                    assert!(take_native_entry());
                    panic!("nested native body");
                }).unwrap();
            }).unwrap();
        })).is_err());
        assert!(!take_native_entry());
        call_native(|| assert!(take_native_entry())).unwrap();
        assert!(!take_native_entry());
    }

    #[test]
    fn concurrent_threads_do_not_share_or_leave_pending_state() {
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
        let threads: Vec<_> = (0..4).map(|_| {
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                assert!(!take_native_entry());
                call_native(|| {
                    barrier.wait();
                    assert!(take_native_entry());
                    assert!(!take_native_entry());
                }).unwrap();
                assert!(!take_native_entry());
            })
        }).collect();
        for thread in threads { thread.join().unwrap(); }
        assert!(!take_native_entry());
    }
}
