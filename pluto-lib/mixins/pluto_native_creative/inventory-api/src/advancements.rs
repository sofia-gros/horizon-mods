//! Generic synchronous data scopes and ordered inventory-change keys.
//!
//! Adapters provide complete snapshots. This crate imports no host or consumer
//! types, interprets no component keys, and owns exactly one context registry.
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    hash::{Hash, Hasher},
    rc::Rc,
};

// A native package may provide this API without any metadata consumer. The
// ordinary host event path must remain untouched until explicitly requested.
struct ComponentEventRequest(std::sync::atomic::AtomicBool);
impl ComponentEventRequest {
    const fn new() -> Self {
        Self(std::sync::atomic::AtomicBool::new(false))
    }
    fn request(&self) {
        self.0.store(true, std::sync::atomic::Ordering::Release);
    }
    fn requested(&self) -> bool {
        self.0.load(std::sync::atomic::Ordering::Acquire)
    }
}
static INVENTORY_COMPONENT_EVENTS: ComponentEventRequest = ComponentEventRequest::new();

/// Opt into full-component inventory events for this process. Idempotent.
/// Call during consumer initialization, before the first inventory event. Never
/// call from an item predicate: an unchanged native hash can prevent that
/// predicate from running. There is no runtime unregister/reset operation.
pub fn request_inventory_component_events() {
    INVENTORY_COMPONENT_EVENTS.request();
}

/// Whether any consumer has requested the shared inventory component adapter.
/// False by default: installing the library alone changes no event behavior.
pub fn inventory_component_events_requested() -> bool {
    INVENTORY_COMPONENT_EVENTS.requested()
}

struct Context {
    key_type: TypeId,
    address: usize,
    data: Rc<dyn Any>,
}
thread_local! { static CURRENT: RefCell<Vec<Context>> = const { RefCell::new(Vec::new()) }; }

/// Run synchronous work with owned data associated with this exact key object.
/// Nested scopes shadow earlier scopes, even for absent data or a different key.
/// Normal returns and panic unwinding restore the caller's scope. Do not return
/// deferred work expecting the data to remain available after this call.
pub fn with_data<K: 'static, D: 'static, R>(key: &K, data: D, run: impl FnOnce() -> R) -> R {
    struct Scope;
    impl Drop for Scope {
        fn drop(&mut self) {
            // Drop caller-owned data after releasing the registry borrow: a data
            // destructor may itself enter another synchronous scope.
            let previous = CURRENT.with(|current| current.borrow_mut().pop());
            drop(previous);
        }
    }
    CURRENT.with(|current| {
        current.borrow_mut().push(Context {
            key_type: TypeId::of::<K>(),
            address: key as *const K as usize,
            data: Rc::new(data),
        })
    });
    let _scope = Scope;
    run()
}

/// Clone data from the innermost scope when key identity and data type match.
/// Addresses are opaque identities and never dereferenced. A moved/cloned key,
/// another thread, the wrong data type, or an absent scope returns None.
/// Zero-sized keys cannot provide distinct object identities and always return None.
pub fn data<K: 'static, D: Clone + 'static>(key: &K) -> Option<D> {
    if std::mem::size_of::<K>() == 0 {
        return None;
    }
    let snapshot = CURRENT.with(|current| {
        let current = current.borrow();
        let context = current.last()?;
        (context.key_type == TypeId::of::<K>() && context.address == key as *const K as usize)
            .then(|| Rc::clone(&context.data))
    })?;
    // User-defined Clone implementations can reenter the API without borrowing
    // the registry twice. Rc does not escape this implementation.
    snapshot.downcast_ref::<D>().cloned()
}

/// Fingerprint an ordered inventory snapshot, without choosing its data schema.
/// Include slot absence, item identity, count and all relevant component data.
/// This is a transient change gate, not a persistent identifier or security hash.
pub fn inventory_key<I: IntoIterator>(slots: I) -> u64
where
    I::Item: Hash,
{
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    for slot in slots {
        slot.hash(&mut hash);
    }
    hash.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inventory_events_start_disabled_and_requests_are_idempotent() {
        // A fresh private policy avoids process-global test-order assumptions.
        let request = ComponentEventRequest::new();
        assert!(!request.requested());
        request.request();
        assert!(request.requested());
        request.request();
        assert!(request.requested());
        std::thread::scope(|scope| {
            scope.spawn(|| assert!(request.requested()));
        });
    }
    #[test]
    fn exact_key_nested_and_thread_scopes_do_not_leak() {
        let a = String::from("same");
        let b = a.clone();
        assert_eq!(data::<_, i32>(&a), None);
        with_data(&a, 1, || {
            assert_eq!(data::<_, i32>(&a), Some(1));
            assert_eq!(data::<_, i32>(&b), None);
            assert_eq!(data::<_, String>(&a), None);
            with_data(&b, 2, || {
                assert_eq!(data::<_, i32>(&a), None);
                assert_eq!(data::<_, i32>(&b), Some(2));
            });
            assert_eq!(data::<_, i32>(&a), Some(1));
            std::thread::scope(|scope| {
                scope.spawn(|| assert_eq!(data::<_, i32>(&a), None));
            });
        });
        assert_eq!(data::<_, i32>(&a), None);
    }
    #[test]
    fn panic_and_absent_data_restore_outer_context() {
        let key = String::new();
        with_data(&key, Some(1), || {
            let result =
                std::panic::catch_unwind(|| with_data(&key, Some(2), || panic!("fixture")));
            assert!(result.is_err());
            assert_eq!(data::<_, Option<i32>>(&key), Some(Some(1)));
            with_data(&key, None::<i32>, || {
                assert_eq!(data::<_, Option<i32>>(&key), Some(None))
            });
            assert_eq!(data::<_, Option<i32>>(&key), Some(Some(1)));
        });
        assert_eq!(data::<_, Option<i32>>(&key), None);
    }
    #[test]
    fn caller_clone_and_drop_can_reenter_without_registry_borrow_panics() {
        struct ReentrantClone(i32);
        impl Clone for ReentrantClone {
            fn clone(&self) -> Self {
                with_data(&String::from("clone"), (), || {});
                Self(self.0)
            }
        }
        let key = Rc::new(String::from("outer"));
        with_data(&*key, ReentrantClone(7), || {
            assert_eq!(data::<_, ReentrantClone>(&*key).unwrap().0, 7);
        });
        struct ReentrantDrop(Rc<String>, Rc<std::cell::Cell<i32>>);
        impl Drop for ReentrantDrop {
            fn drop(&mut self) {
                self.1.set(data::<_, i32>(&*self.0).unwrap_or(-1));
                with_data(&String::from("drop"), (), || {});
            }
        }
        let observed = Rc::new(std::cell::Cell::new(0));
        with_data(&*key, 42, || {
            with_data(
                &String::from("inner"),
                ReentrantDrop(key.clone(), observed.clone()),
                || {},
            );
            assert_eq!(observed.get(), 42);
        });
    }
    #[test]
    fn zero_sized_keys_fail_closed() {
        with_data(&(), 1, || assert_eq!(data::<_, i32>(&()), None));
    }
    #[test]
    fn inventory_key_observes_metadata_count_identity_and_empty_positions() {
        fn key(id: &str, count: u32, metadata: &[u8]) -> u64 {
            inventory_key([None, Some((id, count, metadata))])
        }
        assert_eq!(key("example:item", 1, &[1]), key("example:item", 1, &[1]));
        assert_ne!(key("example:item", 1, &[1]), key("example:item", 1, &[2]));
        assert_ne!(key("example:item", 1, &[1]), key("example:item", 2, &[1]));
        assert_ne!(key("example:item", 1, &[1]), key("example:other", 1, &[1]));
        assert_ne!(key("example:item", 1, &[1]), key("example:item", 1, &[]));
        assert_ne!(
            key("example:item", 1, &[1]),
            inventory_key([Some(("example:item", 1u32, &[1u8][..])), None])
        );
    }
}

pub mod predicates;

pub mod events;

pub mod tags;

pub mod projection;
