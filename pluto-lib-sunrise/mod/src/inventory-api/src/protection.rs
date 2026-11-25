//! Host-independent protection policy inputs and provider registration.
//! Native adapters own hit authority, arithmetic, gates, phase and scope.
use crate::equipment::EquipmentSlots;
use std::collections::BTreeMap;
use std::sync::RwLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bypass {
    Armor,
    Effects,
    Resistance,
    Enchantments,
}

/// Borrowed from the hit's actual resident catalog. Unknown native IDs produce
/// None, not false. Known sources in missing tags produce Some(false).
/// No persistent numeric catalog identity, native enum or cached table escapes.
pub trait Source {
    fn name(&self) -> Option<&str>;
    fn is_type(&self, name: &str) -> Option<bool>;
    fn in_tag(&self, tag: &str) -> Option<bool>;
    fn bypasses(&self, kind: Bypass) -> Option<bool>;
}

/// Pre-evaluation facts, not a final outcome. Providers cannot mutate the
/// player/equipment, cancel a hit, resolve another actor or commit inventory.
pub struct Request<'a> {
    pub victim_entity: i32,
    /// Captured Game dimension if supplied; bare motion paths have None.
    pub captured_dimension: Option<&'a str>,
    pub client_side: bool,
    pub replaying: bool,
    pub spectator: bool,
    pub invulnerable: bool,
    pub source: &'a dyn Source,
    /// Raw method argument before difficulty, blocking, cooldown and reduction.
    pub requested_amount: f32,
    pub living_non_player_cause: bool,
    /// Native zero/unknown sentinel becomes None. No direct entity is inferred.
    pub causing_entity: Option<i32>,
    /// Captured six-slot facts, not live Game reads. None means the native route
    /// supplied no equipment context. Active use is deliberately not exposed.
    pub equipment: Option<&'a dyn EquipmentSlots>,
}

/// One registration's policy contribution, bound privately by the native adapter.
/// Invalid/nonfinite values must return false without changing prior values.
/// These setters do not authorize changing another registration's contribution.
/// Arithmetic and native fallback remain in the adapter, not this facade.
pub trait Contribution {
    /// Finite values <= 0, default 0. Changes only this provider's lower bound.
    fn set_lower(&mut self, lower: f32) -> bool;
    /// Finite values in 20..=25, default 20.
    fn set_upper(&mut self, upper: f32) -> bool;
    /// Signed finite raw protection points, added before the protection clamp.
    /// Zero removes only this provider's additive term.
    fn set_additive(&mut self, points: f32) -> bool;
    /// Remove only this provider's bounds and additive term for this evaluation.
    fn clear(&mut self);
}
pub type Provider = for<'a> fn(&Request<'a>, &mut dyn Contribution);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistrationError {
    InvalidId,
    DuplicateId,
    Poisoned,
}

#[derive(Default)]
struct Registry(RwLock<BTreeMap<String, Provider>>);
impl Registry {
    const fn new() -> Self {
        Self(RwLock::new(BTreeMap::new()))
    }
    fn register(&self, id: &str, callback: Provider) -> Result<(), RegistrationError> {
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
    fn visit(&self, mut visitor: impl FnMut(&str, Provider)) {
        let callbacks = match self.0.read() {
            Ok(entries) => entries
                .iter()
                .map(|(id, callback)| (id.clone(), *callback))
                .collect::<Vec<_>>(),
            Err(_) => return,
        };
        // The lock is gone before consumer code. IDs determine lexical order;
        // changes to registration affect only subsequent dispatch snapshots.
        for (id, callback) in callbacks {
            visitor(&id, callback);
        }
    }
}
static CALLBACKS: Registry = Registry::new();

/// Register during consumer setup, before enabling associated item behavior.
/// Duplicates never replace another callback. Only function pointers are kept.
pub fn register(id: &str, callback: Provider) -> Result<(), RegistrationError> {
    CALLBACKS.register(id, callback)
}
pub fn remove(id: &str) -> Result<bool, RegistrationError> {
    CALLBACKS.remove(id)
}
pub fn enabled() -> bool {
    CALLBACKS.enabled()
}
/// Adapter-only enumeration. The callback list is copied before visitor code;
/// registration changes affect subsequent enumerations. The adapter must bind
/// each provider to its own native contribution capability and invoke it at the
/// documented native phase. This does not create a native hit or policy scope.
pub fn visit(mut visitor: impl FnMut(&str, Provider)) {
    CALLBACKS.visit(&mut visitor);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::equipment::{EquipmentSlot, Slot};
    use std::sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    };
    static SERIAL: Mutex<()> = Mutex::new(());
    static ORDER: AtomicUsize = AtomicUsize::new(0);
    struct Unknown;
    impl Source for Unknown {
        fn name(&self) -> Option<&str> {
            None
        }
        fn is_type(&self, _: &str) -> Option<bool> {
            None
        }
        fn in_tag(&self, _: &str) -> Option<bool> {
            None
        }
        fn bypasses(&self, _: Bypass) -> Option<bool> {
            None
        }
    }
    // No Equipment implementation and no active-use query is needed.
    struct Slots;
    impl EquipmentSlots for Slots {
        fn slot(&self, slot: EquipmentSlot) -> Slot<'_> {
            if slot == EquipmentSlot::MainHand {
                Slot::Unavailable
            } else {
                Slot::Empty
            }
        }
    }
    #[derive(Default)]
    struct Output(f32);
    impl Contribution for Output {
        fn set_lower(&mut self, _: f32) -> bool {
            true
        }
        fn set_upper(&mut self, _: f32) -> bool {
            true
        }
        fn set_additive(&mut self, points: f32) -> bool {
            self.0 = points;
            true
        }
        fn clear(&mut self) {
            self.0 = 0.0;
        }
    }
    fn request<'a>(
        source: &'a dyn Source,
        equipment: Option<&'a dyn EquipmentSlots>,
    ) -> Request<'a> {
        Request {
            victim_entity: -4,
            captured_dimension: None,
            client_side: false,
            replaying: true,
            spectator: true,
            invulnerable: false,
            source,
            requested_amount: f32::from_bits(0x7fc0_0042),
            living_non_player_cause: false,
            causing_entity: None,
            equipment,
        }
    }
    fn first(request: &Request<'_>, output: &mut dyn Contribution) {
        assert_eq!(ORDER.fetch_add(1, Ordering::SeqCst), 0);
        assert_eq!(request.requested_amount.to_bits(), 0x7fc0_0042);
        assert_eq!(request.victim_entity, -4);
        assert_eq!(request.captured_dimension, None);
        assert!(request.replaying);
        assert_eq!(request.source.name(), None);
        assert_eq!(request.source.in_tag("test:tag"), None);
        assert_eq!(request.source.bypasses(Bypass::Armor), None);
        let equipment = request.equipment.unwrap();
        assert!(matches!(
            equipment.slot(EquipmentSlot::MainHand),
            Slot::Unavailable
        ));
        assert!(matches!(
            equipment.slot(EquipmentSlot::OffHand),
            Slot::Empty
        ));
        output.set_additive(-2.0);
    }
    fn second(_: &Request<'_>, output: &mut dyn Contribution) {
        assert_eq!(ORDER.fetch_add(1, Ordering::SeqCst), 1);
        output.set_additive(4.0);
    }
    fn noop(_: &Request<'_>, _: &mut dyn Contribution) {}
    #[test]
    fn providers_visit_in_lexical_id_order_with_separate_outputs() {
        let _serial = SERIAL.lock().unwrap();
        ORDER.store(0, Ordering::SeqCst);
        let registry = Registry::new();
        registry.register("test:z", second).unwrap();
        registry.register("test:a", first).unwrap();
        let request = request(&Unknown, Some(&Slots));
        let mut values = Vec::new();
        registry.visit(|id, provider| {
            let mut output = Output::default();
            provider(&request, &mut output);
            values.push((id.to_owned(), output.0));
        });
        assert_eq!(
            values,
            vec![("test:a".into(), -2.0), ("test:z".into(), 4.0)]
        );
    }
    #[test]
    fn invalid_and_duplicate_registration_never_replaces() {
        let registry = Registry::new();
        for id in [
            "",
            "test",
            ":x",
            "test:",
            "Upper:x",
            "test:../x",
            "test:x//y",
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
        assert_eq!(registry.remove("test:a"), Ok(true));
        assert!(!registry.enabled());
        assert_eq!(registry.remove("test:a"), Ok(false));
    }
    #[test]
    fn removal_during_visit_changes_only_future_snapshots() {
        let registry = Registry::new();
        registry.register("test:a", noop).unwrap();
        registry.register("test:z", noop).unwrap();
        let mut seen = Vec::new();
        registry.visit(|id, _| {
            seen.push(id.to_owned());
            registry.remove("test:z").unwrap();
        });
        assert_eq!(seen, ["test:a", "test:z"]);
        seen.clear();
        registry.visit(|id, _| seen.push(id.to_owned()));
        assert_eq!(seen, ["test:a"]);
    }
    #[test]
    fn absent_equipment_and_unknown_source_remain_explicit() {
        let request = request(&Unknown, None);
        assert!(request.equipment.is_none());
        assert_eq!(request.source.is_type("test:source"), None);
        let registry = Registry::new();
        assert!(!registry.enabled());
        registry.visit(|_, _| panic!("no providers"));
    }
}
