//! Advancement extension boundary: custom item predicates and custom trigger
//! events for the game's advancement engine, in host-independent form.
//!
//! The advancement tree itself — its data files, the vanilla triggers, player
//! progress and rewards — belongs to the game. What a mod adds is policy the
//! engine cannot know: whether an item satisfies a mod-defined item predicate
//! type named in advancement criteria (an inventory change, an equipment
//! check, a placed item), and the firing of mod-defined trigger ids with a
//! payload the criterion's conditions are matched against. Consumers register
//! item predicates and submit events here; a native adapter owns the tree,
//! the current actor, native player conditions, progress, rewards and
//! notifications, and evaluates submissions synchronously.
//!
//! Register once during startup, before inventory events or predicate
//! evaluation. Callbacks must be read-only. No content keys or eligibility
//! rules live here.
//!
//! Consumers also publish their resolved item-tag snapshots here: the
//! criteria a mod ships resolve `#tag` references against the world the
//! player is in, and the native engine's own tag table does not carry the
//! mod's data. A consumer publishes the snapshot its own data resolved to
//! under its owner id; the adapter applies it to the current world's store
//! and answers those tags from it, failing closed on a tag the owner marks
//! unavailable instead of falling back.

use std::collections::BTreeMap;
use std::sync::{Mutex, RwLock};

use serde_json::Value;

use super::base::Nbt;

/// Borrowed facts about one item stack, valid only for this synchronous
/// evaluation. Missing components means unavailable, not an empty stack;
/// consumers requiring them fail closed.
#[derive(Clone, Copy, Debug)]
pub struct ItemSnapshot<'a> {
    pub id: &'a str,
    pub count: u32,
    pub damage: u32,
    pub enchantments: &'a [(String, u32)],
    pub potion: Option<&'a str>,
    pub custom_name: bool,
    /// The stack's component tree in NBT form (for a tool, its custom data
    /// compound), when the adapter could supply it.
    pub components: Option<&'a Nbt>,
}

/// Read-only membership query. The adapter owns world identity and
/// resolution; callers supply the registry/tag/item strings and the library
/// interprets none.
pub trait TagLookup {
    fn in_tag(&self, registry: &str, tag: &str, id: &str) -> bool;
}

pub struct PredicateRequest<'a> {
    pub predicate: &'a Value,
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
    /// These are the first two owner IDs in lexical order, independent of
    /// load order.
    Conflict { first: &'static str, second: &'static str },
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
        self.0.read().unwrap_or_else(|error| error.into_inner()).is_empty()
    }
    fn dispatch(&self, request: &PredicateRequest<'_>) -> DispatchResult {
        // Never run consumer code while holding a lock. Registration is
        // startup-only by contract; an in-flight snapshot is stable even if
        // violated.
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
        handled.map_or(DispatchResult::Unsupported, |(_, value)| DispatchResult::Handled(value))
    }
}

static ITEM_PREDICATES: PredicateRegistry = PredicateRegistry::new();
static INVENTORY_COMPONENT_EVENTS: Mutex<bool> = Mutex::new(false);

/// Process-lifetime registration; duplicate IDs are rejected without
/// replacement. Call before the first inventory event, not lazily from a
/// predicate callback. `inventory_components` explicitly opts into the
/// shared component-aware inventory event adapter after successful
/// registration. No unregister/reset or per-consumer registry exists.
pub fn register_item_predicate(entry: PredicateRegistration) -> Result<(), RegistrationError> {
    ITEM_PREDICATES.register(entry)?;
    if entry.inventory_components {
        let mut requested = INVENTORY_COMPONENT_EVENTS.lock().unwrap_or_else(|error| error.into_inner());
        *requested = true;
    }
    Ok(())
}

pub fn has_item_predicates() -> bool {
    !ITEM_PREDICATES.is_empty()
}

/// Whether any successful registration asked for component-aware inventory
/// events. The native adapter reads this when deciding whether the shared
/// inventory evaluation may stay inert.
pub fn inventory_component_events_requested() -> bool {
    *INVENTORY_COMPONENT_EVENTS.lock().unwrap_or_else(|error| error.into_inner())
}

pub fn dispatch_item_predicate(request: &PredicateRequest<'_>) -> DispatchResult {
    ITEM_PREDICATES.dispatch(request)
}

/// Compares one submitted event payload with one criterion's conditions.
/// Both are the caller's own JSON; the library interprets neither.
pub type EventMatcher = fn(&Value, &Value) -> bool;

/// A custom trigger event waiting for the native adapter. Submission is
/// explicit and synchronous in intent: a consumer submits only after the
/// action's successful authoritative boundary, and the adapter binds the
/// event to its current actor, evaluates the matching criteria of the named
/// trigger (the matcher decides each criterion), and owns progress, rewards
/// and notifications. The queue is the hand-off: it is bounded, and a full
/// queue drops the new event rather than blocking gameplay.
#[derive(Clone, Debug)]
pub struct QueuedEvent {
    pub trigger: String,
    pub payload: Value,
    pub matches: EventMatcher,
}

pub const MAX_QUEUED_EVENTS: usize = 1024;

static EVENT_QUEUE: Mutex<Vec<QueuedEvent>> = Mutex::new(Vec::new());

/// Queue one custom trigger event for the native adapter. Returns false
/// when the queue is full and the event was dropped.
pub fn submit_event(trigger: &str, payload: Value, matches: EventMatcher) -> bool {
    let mut queue = EVENT_QUEUE.lock().unwrap_or_else(|error| error.into_inner());
    if queue.len() >= MAX_QUEUED_EVENTS {
        eprintln!("pluto_lib: advancement event queue full; dropping {trigger}");
        return false;
    }
    queue.push(QueuedEvent { trigger: trigger.to_owned(), payload, matches });
    true
}

/// Drain every queued event, oldest first. The native adapter calls this
/// from a game scope where it can bind events to the current actor; pure
/// consumers only submit.
pub fn take_queued_events() -> Vec<QueuedEvent> {
    let mut queue = EVENT_QUEUE.lock().unwrap_or_else(|error| error.into_inner());
    std::mem::take(&mut *queue)
}

pub fn queued_event_count() -> usize {
    EVENT_QUEUE.lock().unwrap_or_else(|error| error.into_inner()).len()
}

/// One owner's resolved item-tag snapshot for advancement matching, in
/// host-independent form: tag id -> member ids. `Some(empty)` is a known
/// empty tag; `None` is a tag the owner knows but cannot currently resolve,
/// which matching must fail closed on rather than fall back to the native
/// table. Tags absent from the snapshot are not the owner's business: the
/// adapter leaves them to the native table.
pub type TagSnapshot = BTreeMap<String, Option<Vec<String>>>;

static TAG_SNAPSHOTS: Mutex<BTreeMap<String, TagSnapshot>> = Mutex::new(BTreeMap::new());
static TAG_GENERATION: Mutex<u64> = Mutex::new(0);

/// Publish one owner's current resolved item-tag snapshot for advancement
/// matching, replacing the owner's previous publication wholesale.
/// Publishing an empty snapshot releases the ownership: the publication is
/// retained as an empty snapshot so the adapter releases the owner's
/// world-side ownership on its next tick. Consumers publish when their
/// data (re)loads and release when it unloads; the adapter applies
/// publications to the current world's store on its advancement tick and
/// leaves owners it never hears from unmanaged.
pub fn publish_tag_snapshot(owner: &str, snapshot: TagSnapshot) {
    TAG_SNAPSHOTS
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .insert(owner.to_owned(), snapshot);
    *TAG_GENERATION.lock().unwrap_or_else(|error| error.into_inner()) += 1;
}

/// The publication generation: 0 before the first publication, bumped by
/// every `publish_tag_snapshot` call. The native adapter compares it
/// against the generation it last applied to a world's store, so an
/// unchanged publication costs one comparison per tick.
pub fn tag_snapshot_generation() -> u64 {
    *TAG_GENERATION.lock().unwrap_or_else(|error| error.into_inner())
}

/// Every currently published owner snapshot, cloned for the native
/// adapter. Pure consumers only publish; the adapter reads.
pub fn tag_snapshots() -> Vec<(String, TagSnapshot)> {
    TAG_SNAPSHOTS
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .iter()
        .map(|(owner, snapshot)| (owner.clone(), snapshot.clone()))
        .collect()
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

    fn request(v: &Value) -> PredicateRequest<'_> {
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
        PredicateRegistration { id, callback, inventory_components: false }
    }

    #[test]
    fn empty_and_unowned_predicates_preserve_native_fallback() {
        let registry = PredicateRegistry::new();
        let v = serde_json::json!({"anything": "uninterpreted"});
        assert!(registry.is_empty());
        assert_eq!(registry.dispatch(&request(&v)), DispatchResult::Unsupported);
        registry.register(entry("example:ignored", ignored)).unwrap();
        assert!(!registry.is_empty());
        assert_eq!(registry.dispatch(&request(&v)), DispatchResult::Unsupported);
    }

    #[test]
    fn handled_false_is_not_unsupported_and_duplicates_never_replace() {
        let registry = PredicateRegistry::new();
        registry.register(entry("example:owner", no)).unwrap();
        assert_eq!(registry.register(entry("example:owner", yes)), Err(RegistrationError::DuplicateId));
        assert_eq!(registry.register(entry("", yes)), Err(RegistrationError::EmptyId));
        assert_eq!(registry.dispatch(&request(&Value::Null)), DispatchResult::Handled(false));
    }

    #[test]
    fn conflicts_are_deterministic_independent_of_registration_order() {
        for entries in [[entry("z", yes), entry("a", no)], [entry("a", no), entry("z", yes)]] {
            let registry = PredicateRegistry::new();
            for entry in entries {
                registry.register(entry).unwrap();
            }
            registry.register(entry("middle", ignored)).unwrap();
            assert_eq!(
                registry.dispatch(&request(&Value::Null)),
                DispatchResult::Conflict { first: "a", second: "z" }
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
        assert_eq!(registry.dispatch(&request(&serde_json::json!({"key": 3}))), DispatchResult::Handled(true));
    }

    #[test]
    fn startup_registration_activates_component_events_and_callbacks_run_outside_lock() {
        fn inspect(_: &PredicateRequest<'_>) -> PredicateResult {
            PredicateResult::Handled(ITEM_PREDICATES.0.try_write().is_ok())
        }
        // This is the only test using the public process-lifetime registry.
        assert!(!has_item_predicates());
        assert!(!inventory_component_events_requested());
        register_item_predicate(PredicateRegistration { id: "example:startup", callback: inspect, inventory_components: true }).unwrap();
        assert!(has_item_predicates());
        assert!(inventory_component_events_requested());
        assert_eq!(dispatch_item_predicate(&request(&Value::Null)), DispatchResult::Handled(true));
        assert_eq!(register_item_predicate(entry("example:startup", no)), Err(RegistrationError::DuplicateId));
    }

    #[test]
    fn events_queue_in_order_and_drain_empty() {
        fn same(payload: &Value, conditions: &Value) -> bool {
            payload["type"] == conditions["type"]
        }
        // The process queue is shared with the registration test above; drain
        // first so this test observes only its own submissions.
        let _ = take_queued_events();
        assert_eq!(queued_event_count(), 0);
        assert!(submit_event("example:opened", serde_json::json!({"type": "example:a"}), same));
        assert!(submit_event("example:opened", serde_json::json!({"type": "example:b"}), same));
        assert_eq!(queued_event_count(), 2);
        let drained = take_queued_events();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0].trigger, "example:opened");
        assert!((drained[0].matches)(&drained[0].payload, &serde_json::json!({"type": "example:a"})));
        assert!(!(drained[0].matches)(&drained[0].payload, &serde_json::json!({"type": "example:b"})));
        assert_eq!(queued_event_count(), 0);
        assert!(take_queued_events().is_empty());
    }

    fn snapshot(members: Option<Vec<&str>>) -> TagSnapshot {
        [("example:tag".to_string(), members.map(|v| v.into_iter().map(str::to_owned).collect()))]
            .into_iter()
            .collect()
    }

    #[test]
    fn tag_snapshots_publish_replace_and_release_per_owner() {
        // The publication registry is process-global; unique owner ids keep
        // this test observing only its own publications, and it releases
        // them again so no other test is affected.
        let before = tag_snapshot_generation();
        publish_tag_snapshot("example:owner-a", snapshot(Some(vec!["example:x"])));
        publish_tag_snapshot("example:owner-b", snapshot(None));
        assert_eq!(tag_snapshot_generation(), before + 2);
        let published = tag_snapshots();
        assert!(published.contains(&("example:owner-a".to_string(), snapshot(Some(vec!["example:x"])))));
        assert!(published.contains(&("example:owner-b".to_string(), snapshot(None))));
        // Replacement is wholesale, never a merge.
        publish_tag_snapshot("example:owner-a", snapshot(Some(vec!["example:z"])));
        let published = tag_snapshots();
        assert!(published.contains(&("example:owner-a".to_string(), snapshot(Some(vec!["example:z"])))));
        assert!(!published.contains(&("example:owner-a".to_string(), snapshot(Some(vec!["example:x"])))));
        // An empty publication is retained as an empty snapshot, so the
        // adapter can release the owner's world-side ownership.
        publish_tag_snapshot("example:owner-a", TagSnapshot::new());
        assert!(tag_snapshots().contains(&("example:owner-a".to_string(), TagSnapshot::new())));
        publish_tag_snapshot("example:owner-b", TagSnapshot::new());
        assert_eq!(tag_snapshot_generation(), before + 5);
    }
}
