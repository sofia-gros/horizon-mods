//! Host-independent, synchronous item-predicate extension boundary.
//!
//! Register once during startup, before inventory events or predicate evaluation.
//! Callbacks must be read-only. No content keys or eligibility rules live here.
use std::collections::BTreeMap;
use std::sync::RwLock;

/// Lossless UTF-16 text. Invalid surrogate sequences are preserved, not replaced.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentText(pub Vec<u16>);
impl From<&str> for ComponentText {
    fn from(value: &str) -> Self {
        Self(value.encode_utf16().collect())
    }
}
impl ComponentText {
    pub fn to_string(&self) -> Result<String, std::string::FromUtf16Error> {
        String::from_utf16(&self.0)
    }
}

/// Lossless component tree: integer widths, arrays, float bits and text survive.
/// This is a data snapshot, not a parser or a component schema interpreter.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ComponentValue {
    End,
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(u32),
    Double(u64),
    ByteArray(Vec<u8>),
    String(ComponentText),
    List(Vec<ComponentValue>),
    Compound(BTreeMap<ComponentText, ComponentValue>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}
impl ComponentValue {
    pub fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Compound(entries) => entries.get(&ComponentText::from(key)),
            _ => None,
        }
    }
}

/// Borrowed facts valid only for this synchronous evaluation. Missing components
/// means unavailable, not an empty stack; consumers requiring them fail closed.
#[derive(Clone, Copy, Debug)]
pub struct ItemSnapshot<'a> {
    pub id: &'a str,
    pub count: u32,
    pub damage: u32,
    pub enchantments: &'a [(String, u32)],
    pub potion: Option<&'a str>,
    pub custom_name: bool,
    pub components: Option<&'a ComponentValue>,
}

/// Read-only membership query. The adapter owns world identity and resolution;
/// callers supply the registry/tag/item strings and the library interprets none.
pub trait TagLookup {
    fn in_tag(&self, registry: &str, tag: &str, id: &str) -> bool;
}

pub struct PredicateRequest<'a> {
    pub predicate: &'a serde_json::Value,
    pub item: ItemSnapshot<'a>,
    pub tags: &'a dyn TagLookup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PredicateResult {
    /// This callback does not own the supplied predicate form.
    Unsupported,
    /// This callback owns the form, including malformed/nonmatching cases.
    Handled(bool),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DispatchResult {
    Unsupported,
    Handled(bool),
    /// Two or more callbacks claimed ownership. The adapter must fail closed.
    /// These are the first two owner IDs in lexical order, independent of load order.
    Conflict {
        first: &'static str,
        second: &'static str,
    },
}
pub type PredicateCallback = for<'a> fn(&PredicateRequest<'a>) -> PredicateResult;

#[derive(Clone, Copy)]
pub struct PredicateRegistration {
    /// Stable unique registration identity, not a predicate discriminator.
    pub id: &'static str,
    pub callback: PredicateCallback,
    /// Explicit startup request for metadata-sensitive inventory events.
    pub inventory_components: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistrationError {
    EmptyId,
    DuplicateId,
}

#[derive(Default)]
struct PredicateRegistry(RwLock<BTreeMap<&'static str, PredicateCallback>>);
impl PredicateRegistry {
    const fn new() -> Self {
        Self(RwLock::new(BTreeMap::new()))
    }
    fn register(&self, entry: PredicateRegistration) -> Result<(), RegistrationError> {
        if entry.id.is_empty() {
            return Err(RegistrationError::EmptyId);
        }
        let mut callbacks = self.0.write().unwrap_or_else(|error| error.into_inner());
        if callbacks.contains_key(entry.id) {
            return Err(RegistrationError::DuplicateId);
        }
        callbacks.insert(entry.id, entry.callback);
        Ok(())
    }
    fn is_empty(&self) -> bool {
        self.0
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .is_empty()
    }
    fn dispatch(&self, request: &PredicateRequest<'_>) -> DispatchResult {
        // Never run consumer code while holding a lock. Registration is startup-
        // only by contract; an in-flight snapshot is stable even if violated.
        let callbacks: Vec<_> = self
            .0
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .iter()
            .map(|(&id, &callback)| (id, callback))
            .collect();
        let mut handled = None;
        for (id, callback) in callbacks {
            if let PredicateResult::Handled(value) = callback(request) {
                if let Some((first, _)) = handled {
                    return DispatchResult::Conflict { first, second: id };
                }
                handled = Some((id, value));
            }
        }
        handled.map_or(DispatchResult::Unsupported, |(_, value)| {
            DispatchResult::Handled(value)
        })
    }
}
static ITEM_PREDICATES: PredicateRegistry = PredicateRegistry::new();

/// Process-lifetime registration; duplicate IDs are rejected without replacement.
/// Call before the first inventory event, not lazily from a predicate callback.
/// `inventory_components` explicitly opts into the shared event adapter after
/// successful registration. No unregister/reset or per-consumer registry exists.
pub fn register_item_predicate(entry: PredicateRegistration) -> Result<(), RegistrationError> {
    ITEM_PREDICATES.register(entry)?;
    if entry.inventory_components {
        super::request_inventory_component_events();
    }
    Ok(())
}
pub fn has_item_predicates() -> bool {
    !ITEM_PREDICATES.is_empty()
}
pub fn dispatch_item_predicate(request: &PredicateRequest<'_>) -> DispatchResult {
    ITEM_PREDICATES.dispatch(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Tags;
    impl TagLookup for Tags {
        fn in_tag(&self, registry: &str, tag: &str, id: &str) -> bool {
            (registry, tag, id) == ("item", "example:tag", "example:item")
        }
    }
    fn request(v: &serde_json::Value) -> PredicateRequest<'_> {
        PredicateRequest {
            predicate: v,
            tags: &Tags,
            item: ItemSnapshot {
                id: "example:item",
                count: 1,
                damage: 2,
                enchantments: &[],
                potion: None,
                custom_name: false,
                components: None,
            },
        }
    }
    fn ignored(_: &PredicateRequest<'_>) -> PredicateResult {
        PredicateResult::Unsupported
    }
    fn yes(_: &PredicateRequest<'_>) -> PredicateResult {
        PredicateResult::Handled(true)
    }
    fn no(_: &PredicateRequest<'_>) -> PredicateResult {
        PredicateResult::Handled(false)
    }
    fn entry(id: &'static str, callback: PredicateCallback) -> PredicateRegistration {
        PredicateRegistration {
            id,
            callback,
            inventory_components: false,
        }
    }
    #[test]
    fn empty_and_unowned_predicates_preserve_native_fallback() {
        let registry = PredicateRegistry::new();
        let v = serde_json::json!({"anything": "uninterpreted"});
        assert!(registry.is_empty());
        assert_eq!(registry.dispatch(&request(&v)), DispatchResult::Unsupported);
        registry
            .register(entry("example:ignored", ignored))
            .unwrap();
        assert!(!registry.is_empty());
        assert_eq!(registry.dispatch(&request(&v)), DispatchResult::Unsupported);
    }
    #[test]
    fn handled_false_is_not_unsupported_and_duplicates_never_replace() {
        let registry = PredicateRegistry::new();
        registry.register(entry("example:owner", no)).unwrap();
        assert_eq!(
            registry.register(entry("example:owner", yes)),
            Err(RegistrationError::DuplicateId)
        );
        assert_eq!(
            registry.register(entry("", yes)),
            Err(RegistrationError::EmptyId)
        );
        assert_eq!(
            registry.dispatch(&request(&serde_json::Value::Null)),
            DispatchResult::Handled(false)
        );
    }
    #[test]
    fn conflicts_are_deterministic_independent_of_registration_order() {
        for entries in [
            [entry("z", yes), entry("a", no)],
            [entry("a", no), entry("z", yes)],
        ] {
            let registry = PredicateRegistry::new();
            for entry in entries {
                registry.register(entry).unwrap();
            }
            registry.register(entry("middle", ignored)).unwrap();
            assert_eq!(
                registry.dispatch(&request(&serde_json::Value::Null)),
                DispatchResult::Conflict {
                    first: "a",
                    second: "z"
                }
            );
        }
    }
    #[test]
    fn callback_receives_read_only_facts_and_unavailable_metadata() {
        fn inspect(r: &PredicateRequest<'_>) -> PredicateResult {
            PredicateResult::Handled(
                r.tags.in_tag("item", "example:tag", r.item.id)
                    && r.item.count == 1
                    && r.item.damage == 2
                    && r.item.components.is_none()
                    && r.predicate["key"] == 3,
            )
        }
        let registry = PredicateRegistry::new();
        registry.register(entry("inspect", inspect)).unwrap();
        assert_eq!(
            registry.dispatch(&request(&serde_json::json!({"key": 3}))),
            DispatchResult::Handled(true)
        );
    }
    #[test]
    fn startup_registration_activates_events_and_callbacks_run_outside_lock() {
        fn inspect(_: &PredicateRequest<'_>) -> PredicateResult {
            PredicateResult::Handled(ITEM_PREDICATES.0.try_write().is_ok())
        }
        // This is the only test using the public process-lifetime registry.
        assert!(!has_item_predicates());
        assert!(!super::super::inventory_component_events_requested());
        register_item_predicate(PredicateRegistration {
            id: "example:startup",
            callback: inspect,
            inventory_components: true,
        })
        .unwrap();
        assert!(has_item_predicates());
        assert!(super::super::inventory_component_events_requested());
        assert_eq!(
            dispatch_item_predicate(&request(&serde_json::Value::Null)),
            DispatchResult::Handled(true)
        );
        assert_eq!(
            register_item_predicate(entry("example:startup", no)),
            Err(RegistrationError::DuplicateId)
        );
    }
    #[test]
    fn component_values_preserve_numeric_width_bits_and_utf16() {
        let text = ComponentText(vec![0xd800]);
        assert!(text.to_string().is_err());
        assert_ne!(ComponentValue::Byte(1), ComponentValue::Int(1));
        let value = ComponentValue::Compound(
            [
                ("float".into(), ComponentValue::Float(0x7fc00042)),
                ("text".into(), ComponentValue::String(text.clone())),
                (text, ComponentValue::LongArray(vec![i64::MIN, i64::MAX])),
            ]
            .into_iter()
            .collect(),
        );
        assert_eq!(value.get("float"), Some(&ComponentValue::Float(0x7fc00042)));
        assert_eq!(
            value.get("text"),
            Some(&ComponentValue::String(ComponentText(vec![0xd800])))
        );
        assert_eq!(value.get("absent"), None);
    }
}
