//! Read-only observations of authoritative native armor-wear consumption.
//!
//! An observation is not a durability delta, commit receipt, original damage
//! outcome, or permission to change a stack. Adapters specify exact coverage.
use crate::equipment::{Actor, Equipment};
use std::collections::BTreeMap;
use std::sync::RwLock;

/// Borrowed only during synchronous dispatch. Both equipment views describe
/// consumption time: immediately before native wear and after its break tails.
/// Other native callbacks may also change equipment between these snapshots.
pub struct Request<'a> {
    pub actor: Actor<'a>,
    /// Catalog name at consumption, or None for an unknown native damage ID.
    /// HealthEvent does not retain an attacker or original outcome.
    pub source_kind: Option<&'a str>,
    /// The raw native request, not computed item wear. Nonfinite and nonpositive
    /// values are not normalized; native policy alone decides their effects.
    pub requested_damage: f32,
    pub before: &'a dyn Equipment,
    pub after: &'a dyn Equipment,
}
pub type Callback = for<'a> fn(&Request<'a>);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistrationError {
    InvalidId,
    DuplicateId,
    Poisoned,
}

#[derive(Default)]
struct Registry(RwLock<BTreeMap<String, Callback>>);
impl Registry {
    const fn new() -> Self {
        Self(RwLock::new(BTreeMap::new()))
    }
    fn register(&self, id: &str, callback: Callback) -> Result<(), RegistrationError> {
        let Some((namespace, path)) = id.split_once(':') else {
            return Err(RegistrationError::InvalidId);
        };
        if namespace.is_empty()
            || !namespace
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.-".contains(&c))
            || path.is_empty()
            || !path
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c))
            || path
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(RegistrationError::InvalidId);
        }
        let mut callbacks = self.0.write().map_err(|_| RegistrationError::Poisoned)?;
        if callbacks.contains_key(id) {
            return Err(RegistrationError::DuplicateId);
        }
        callbacks.insert(id.to_owned(), callback);
        Ok(())
    }
    fn remove(&self, id: &str) -> Result<bool, RegistrationError> {
        Ok(self
            .0
            .write()
            .map_err(|_| RegistrationError::Poisoned)?
            .remove(id)
            .is_some())
    }
    fn enabled(&self) -> bool {
        self.0.read().is_ok_and(|entries| !entries.is_empty())
    }
    fn dispatch(&self, request: &Request<'_>) {
        let callbacks = match self.0.read() {
            Ok(entries) => entries.values().copied().collect::<Vec<_>>(),
            Err(_) => return,
        };
        // The lock is gone before consumer code. IDs determine lexical order;
        // changes to registration affect only subsequent dispatch snapshots.
        for callback in callbacks {
            callback(request);
        }
    }
}
static CALLBACKS: Registry = Registry::new();

/// Register during consumer setup, before enabling associated item behavior.
/// Duplicates never replace another callback. Only function pointers are kept.
pub fn register(id: &str, callback: Callback) -> Result<(), RegistrationError> {
    CALLBACKS.register(id, callback)
}
pub fn remove(id: &str) -> Result<bool, RegistrationError> {
    CALLBACKS.remove(id)
}
pub fn enabled() -> bool {
    CALLBACKS.enabled()
}
/// Adapter entry. Establish authoritative actor/phase before calling. No native
/// actor selection, background work or event fabrication occurs in this API.
pub fn dispatch(request: &Request<'_>) {
    CALLBACKS.dispatch(request);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::equipment::{ActiveUse, EquipmentSlot, Slot};
    use std::sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    };
    static SERIAL: Mutex<()> = Mutex::new(());
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    struct View(bool);
    impl Equipment for View {
        fn slot(&self, _: EquipmentSlot) -> Slot<'_> {
            if self.0 {
                Slot::Unavailable
            } else {
                Slot::Empty
            }
        }
        fn active_use(&self) -> Option<ActiveUse<'_>> {
            None
        }
    }
    fn request<'a>(before: &'a View, after: &'a View) -> Request<'a> {
        Request {
            actor: Actor {
                entity: -3,
                dimension: "test:world",
                creative: true,
                spectator: false,
            },
            source_kind: None,
            requested_damage: f32::from_bits(0x7fc0_0042),
            before,
            after,
        }
    }
    fn first(request: &Request<'_>) {
        assert_eq!(CALLS.fetch_add(1, Ordering::SeqCst), 0);
        assert_eq!(request.requested_damage.to_bits(), 0x7fc0_0042);
        assert_eq!(request.source_kind, None);
        assert_eq!(request.actor.entity, -3);
        assert!(request.actor.creative);
        assert!(matches!(
            request.before.slot(EquipmentSlot::Chest),
            Slot::Unavailable
        ));
        assert!(matches!(
            request.after.slot(EquipmentSlot::Chest),
            Slot::Empty
        ));
    }
    fn second(_: &Request<'_>) {
        assert_eq!(CALLS.fetch_add(1, Ordering::SeqCst), 1);
    }
    fn noop(_: &Request<'_>) {}
    #[test]
    fn raw_unknown_source_and_distinct_views_reach_stably_ordered_observers() {
        let _lock = SERIAL.lock().unwrap();
        CALLS.store(0, Ordering::SeqCst);
        let registry = Registry::new();
        registry.register("test:z", second).unwrap();
        registry.register("test:a", first).unwrap();
        registry.dispatch(&request(&View(true), &View(false)));
        assert_eq!(CALLS.load(Ordering::SeqCst), 2);
    }
    #[test]
    fn invalid_and_duplicate_registrations_do_not_replace() {
        let registry = Registry::new();
        for id in [
            "",
            "test",
            ":x",
            "test:",
            "test:x:y",
            "Upper:x",
            "test:x//y",
            "test:../x",
        ] {
            assert_eq!(
                registry.register(id, noop),
                Err(RegistrationError::InvalidId)
            );
        }
        registry.register("test:a", noop).unwrap();
        assert_eq!(
            registry.register("test:a", first),
            Err(RegistrationError::DuplicateId)
        );
        assert!(registry.enabled());
        assert_eq!(registry.remove("test:a"), Ok(true));
        assert!(!registry.enabled());
        assert_eq!(registry.remove("test:a"), Ok(false));
    }
    static CHANGING: Registry = Registry::new();
    fn remove_second(request: &Request<'_>) {
        first(request);
        CHANGING.remove("test:z").unwrap();
    }
    #[test]
    fn observers_run_unlocked_and_removal_changes_only_later_dispatch() {
        let _lock = SERIAL.lock().unwrap();
        CALLS.store(0, Ordering::SeqCst);
        CHANGING.register("test:a", remove_second).unwrap();
        CHANGING.register("test:z", second).unwrap();
        CHANGING.dispatch(&request(&View(true), &View(false)));
        assert_eq!(CALLS.load(Ordering::SeqCst), 2);
        CALLS.store(0, Ordering::SeqCst);
        CHANGING.dispatch(&request(&View(true), &View(false)));
        assert_eq!(CALLS.load(Ordering::SeqCst), 1);
        CHANGING.remove("test:a").unwrap();
    }
    #[test]
    fn absent_observers_do_not_change_inputs() {
        let before = View(true);
        let after = View(false);
        let mut request = request(&before, &after);
        request.source_kind = Some("test:fire");
        request.requested_damage = -5.0;
        let registry = Registry::new();
        assert!(!registry.enabled());
        registry.dispatch(&request);
        assert_eq!(request.requested_damage, -5.0);
        assert_eq!(request.source_kind, Some("test:fire"));
    }
}
