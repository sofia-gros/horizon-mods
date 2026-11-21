//! Read-only, synchronous block feedback without native host types.
//!
//! An adapter supplies facts at its final raised-item feedback boundary. This
//! API cannot mutate equipment, cancel a hit, award damage or perform wear.
use std::collections::BTreeMap;
use std::sync::RwLock;

/// Exact outcome facts. `hurt == false` does not imply `blocked == 0`.
/// Nonfinite/zero amounts and canceled outcomes are not normalized or filtered.
#[derive(Clone, Copy, Debug, Default)]
pub struct Outcome {
    pub hurt: bool,
    pub blocked: f32,
    pub fully_blocked: bool,
    pub sound: bool,
    pub knockback: bool,
    pub canceled: bool,
}

/// Entity numbers are opaque, scoped to this event's current world/actor.
/// They are not persistent identities or permission to resolve another entity.
/// The adapter preserves its raw zero/unknown source sentinel.
#[derive(Clone, Copy, Debug)]
pub struct Source<'a> {
    pub kind: &'a str,
    pub causing: i32,
    pub direct: i32,
}
#[derive(Clone, Copy, Debug)]
pub struct Event<'a> {
    pub outcome: Outcome,
    pub source: Source<'a>,
    pub attacker: Option<u32>,
    pub projectile: bool,
}
pub use crate::equipment::{ActiveUse, Actor, Equipment, EquipmentSlot, Slot};

pub struct Request<'a> {
    pub event: Event<'a>,
    pub actor: Actor<'a>,
    pub equipment: &'a dyn Equipment,
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
    use crate::advancements::predicates::ItemSnapshot;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    };
    static SERIAL: Mutex<()> = Mutex::new(());
    static ORDER: AtomicUsize = AtomicUsize::new(0);
    struct Empty;
    impl Equipment for Empty {
        fn slot(&self, slot: EquipmentSlot) -> Slot<'_> {
            if slot == EquipmentSlot::MainHand {
                Slot::Unavailable
            } else {
                Slot::Empty
            }
        }
        fn active_use(&self) -> Option<ActiveUse<'_>> {
            None
        }
    }
    fn request(equipment: &dyn Equipment) -> Request<'_> {
        Request {
            event: Event {
                outcome: Outcome {
                    blocked: f32::from_bits(0x7fc0_0042),
                    canceled: true,
                    ..Default::default()
                },
                source: Source {
                    kind: "test:damage",
                    causing: -3,
                    direct: 0,
                },
                attacker: Some(42),
                projectile: true,
            },
            actor: Actor {
                entity: -3,
                dimension: "test:world",
                creative: false,
                spectator: false,
            },
            equipment,
        }
    }
    fn first(request: &Request<'_>) {
        assert_eq!(ORDER.fetch_add(1, Ordering::SeqCst), 0);
        assert_eq!(request.event.outcome.blocked.to_bits(), 0x7fc0_0042);
        assert!(request.event.outcome.canceled);
        assert_eq!(request.event.source.causing, -3);
        assert_eq!(request.actor.dimension, "test:world");
        assert!(matches!(
            request.equipment.slot(EquipmentSlot::MainHand),
            Slot::Unavailable
        ));
        assert!(matches!(
            request.equipment.slot(EquipmentSlot::OffHand),
            Slot::Empty
        ));
    }
    fn second(_: &Request<'_>) {
        assert_eq!(ORDER.fetch_add(1, Ordering::SeqCst), 1);
    }
    fn noop(_: &Request<'_>) {}
    #[test]
    fn callbacks_receive_raw_facts_in_stable_order_without_host_types() {
        let _serial = SERIAL.lock().unwrap();
        ORDER.store(0, Ordering::SeqCst);
        let registry = Registry::new();
        registry.register("test:z", second).unwrap();
        registry.register("test:a", first).unwrap();
        registry.dispatch(&request(&Empty));
        assert_eq!(ORDER.load(Ordering::SeqCst), 2);
    }
    #[test]
    fn invalid_and_duplicate_ids_do_not_replace_callbacks() {
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
        registry.dispatch(&request(&Empty));
    }
    #[test]
    fn item_and_active_use_views_preserve_empty_unavailable_and_other_slots() {
        struct EquipmentFacts;
        impl Equipment for EquipmentFacts {
            fn slot(&self, _: EquipmentSlot) -> Slot<'_> {
                Slot::Empty
            }
            fn active_use(&self) -> Option<ActiveUse<'_>> {
                Some(ActiveUse {
                    slot: None,
                    item: Slot::Item(ItemSnapshot {
                        id: "test:item",
                        count: 0,
                        damage: u32::MAX,
                        enchantments: &[],
                        potion: None,
                        custom_name: false,
                        components: None,
                    }),
                })
            }
        }
        let facts: &dyn Equipment = &EquipmentFacts;
        let active = facts.active_use().unwrap();
        assert_eq!(active.slot, None);
        let Slot::Item(item) = active.item else {
            panic!("known item facts")
        };
        assert_eq!((item.count, item.damage), (0, u32::MAX));
        assert!(item.components.is_none());
    }
    static CHANGING: Registry = Registry::new();
    fn remove_second(request: &Request<'_>) {
        first(request);
        CHANGING.remove("test:z").unwrap();
    }
    #[test]
    fn callbacks_run_unlocked_and_removal_only_changes_future_snapshots() {
        let _serial = SERIAL.lock().unwrap();
        ORDER.store(0, Ordering::SeqCst);
        CHANGING.register("test:a", remove_second).unwrap();
        CHANGING.register("test:z", second).unwrap();
        CHANGING.dispatch(&request(&Empty));
        assert_eq!(ORDER.load(Ordering::SeqCst), 2);
        ORDER.store(0, Ordering::SeqCst);
        CHANGING.dispatch(&request(&Empty));
        assert_eq!(ORDER.load(Ordering::SeqCst), 1);
        CHANGING.remove("test:a").unwrap();
    }
}
