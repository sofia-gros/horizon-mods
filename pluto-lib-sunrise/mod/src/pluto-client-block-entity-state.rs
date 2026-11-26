// Instance-owned, unsaved client block-entity state. This module is tested
// directly by the pure suite. The state lives in the module-owned identity
// store below (client_state), not in a woven field; native compilation of
// the consumers remains pending.
// Owner candidate: ModBlockEntity's private-field constructor. Its custom Clone
// calls new, and live block rendering borrows the world's entity, not a clone.
// State/constructor contracts exist in source but await real plan review.
// Client tick/lifetime dispatch and draw forwarding are source-wired;
// neither the native lifecycle nor actual state attachment is validated yet.
use std::sync::{Arc, Mutex, OnceLock};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_CACHE_TOKEN: AtomicU64 = AtomicU64::new(1);
fn fresh_cache_token() -> u64 {
    let mut current = NEXT_CACHE_TOKEN.load(Ordering::Relaxed);
    loop {
        let next = current.checked_add(1).expect("client cache token exhausted");
        match NEXT_CACHE_TOKEN.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return current,
            Err(seen) => current = seen,
        }
    }
}
static CACHE_GENERATION: OnceLock<fn() -> Option<u64>> = OnceLock::new();
pub fn set_cache_generation_provider(provider: fn() -> Option<u64>) -> Result<(), &'static str> {
    CACHE_GENERATION.set(provider).map_err(|_| "cache generation provider already installed")
}
pub fn cache_generation() -> Option<u64> { CACHE_GENERATION.get().and_then(|provider| provider()) }

static CLIENT_STATES: Mutex<std::collections::BTreeMap<u64, Arc<dyn std::any::Any + Send + Sync>>> =
    Mutex::new(std::collections::BTreeMap::new());

/// The client state owned by the block entity with the given native
/// identity (`ModBlockEntity::revision().identity()`: one per entity
/// made, never reused in-process; a clone is another entity with a
/// fresh identity, matching this state's "clone = fresh default"
/// semantics). The Sunrise host carries no woven field for this state,
/// so the module owns the store, keyed by that identity; the host
/// itself keeps an identity-keyed static store for the same entities
/// (UPDATE_FIELDS in sunrise-mod-runtime). Level unload resets the
/// states of the entities it visits instead of evicting them: parked
/// worlds keep their entities, whose states resume on restore. A
/// caller asking for a different T than the stored one receives a
/// fresh detached state instead of another instance's data — the
/// conservative direction, like a busy or poisoned state refusing
/// reuse.
pub fn client_state<T: Default + Send + Sync + 'static>(identity: u64) -> Arc<ClientState<T>> {
    let mut store = CLIENT_STATES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(existing) = store.get(&identity) {
        if let Ok(state) = Arc::clone(existing).downcast::<ClientState<T>>() {
            return state;
        }
        return Arc::new(ClientState::default());
    }
    let state = Arc::new(ClientState::default());
    store.insert(identity, Arc::clone(&state) as Arc<dyn std::any::Any + Send + Sync>);
    state
}

/// The native animation pass also has server callers. Window tick events alone
/// are insufficient because they run paused and on title screens as well.
pub fn client_tick_enabled(client: bool, has_level: bool, paused: bool, title: bool, running: bool) -> bool {
    client && has_level && !paused && !title && running
}

/// Match the host's ClientLevel identity, not just the dimension type or a
/// respawn flag. Missing prior identity and forced replacements start anew.
pub fn client_level_replaced(keep_world: bool, heard_level: Option<&str>, incoming: &str) -> bool {
    !keep_world || heard_level != Some(incoming)
}

/// Actual native instance candidates, not names inferred from a head suffix or
/// the mesher's template-only block-entity list. Borrowed only during selection.
pub struct ClientCandidate<'a> {
    pub pos: [i32; 3],
    pub block: &'a str,
    pub is_mod_entity: bool,
}

/// Deterministic client targets. Section Y is relative to the dimension's
/// bottom, matching Renderer::has_section. Do not gate on fade/frustum/dirty
/// meshes: those are rendering concerns, not client block-entity lifetimes.
/// Conflicting duplicates are refused instead of ticking an arbitrary occupant.
pub fn client_targets<'a>(
    candidates: impl IntoIterator<Item = ClientCandidate<'a>>, client: bool, bounds: [i32; 2],
    mut registered: impl FnMut(&str) -> bool, mut loaded: impl FnMut([i32; 3]) -> bool,
    mut resident: impl FnMut([i32; 3]) -> bool,
) -> Vec<[i32; 3]> {
    if !client || bounds[0] >= bounds[1] { return Vec::new(); }
    let mut unique = std::collections::BTreeMap::new();
    for candidate in candidates {
        let occupant = (candidate.block, candidate.is_mod_entity);
        unique.entry(candidate.pos).and_modify(|previous| {
            if *previous != Some(occupant) { *previous = None; }
        }).or_insert(Some(occupant));
    }
    unique.into_iter().filter_map(|(pos, occupant)| {
        let (block, is_mod) = occupant?;
        if !is_mod || pos[1] < bounds[0] || pos[1] >= bounds[1] { return None; }
        let y = pos[1].checked_sub(bounds[0])?;
        let section = [pos[0].div_euclid(16), y.div_euclid(16), pos[2].div_euclid(16)];
        (loaded(pos) && resident(section) && registered(block)).then_some(pos)
    }).collect()
}

struct Slot<T> {
    value: T,
    updating: bool,
    cache_memo: Option<(u64, u64)>,
}

/// T is host-owned plain data (a compound), not a mod-supplied Clone callback.
/// No serialization API or position/world registry is provided. An ordinary
/// clone is a new client instance with fresh default state, never a shared clock.
pub struct ClientState<T> {
    root: Mutex<ClientRoot<T>>,
}
struct ClientRoot<T> {
    inner: Arc<Mutex<Slot<T>>>,
    client_active: bool,
}
impl<T: Default> Default for ClientState<T> {
    fn default() -> Self {
        Self { root: Mutex::new(ClientRoot { inner: Self::fresh(), client_active: false }) }
    }
}
impl<T: Default> ClientState<T> {
    fn fresh() -> Arc<Mutex<Slot<T>>> {
        Arc::new(Mutex::new(Slot { value: T::default(), updating: false, cache_memo: None }))
    }

    /// Interior reset avoids BlockEntities::get_mut/iter_mut, which wake server
    /// tickers. Old leases remain detached. Call only for real client lifetimes.
    pub fn reset(&self) -> Result<(), &'static str> {
        let fresh = Self::fresh();
        let previous = {
            let mut root = self.root.lock().map_err(|_| "client block entity root poisoned")?;
            root.client_active = false;
            std::mem::replace(&mut root.inner, fresh)
        };
        drop(previous);
        Ok(())
    }

    /// Tick-driver membership, not frustum visibility. Enter/leave resets once;
    /// staying active preserves the clock. This never takes native mutable data.
    pub fn set_client_active(&self, active: bool) -> Result<bool, &'static str> {
        let previous = {
            let mut root = self.root.lock().map_err(|_| "client block entity root poisoned")?;
            if root.client_active == active { return Ok(active); }
            root.client_active = active;
            std::mem::replace(&mut root.inner, Self::fresh())
        };
        drop(previous);
        Ok(active)
    }
}
impl<T> ClientState<T> {
    /// Native changes is an opaque counter, not data. Memoize a unique token
    /// for its current value plus this committed client lifetime. Never hash
    /// counters together or mutate the native saved-data revision to invalidate.
    /// Busy/poisoned state returns a fresh token, conservatively refusing reuse.
    pub fn cache_token(&self, saved_changes: u64) -> u64 {
        let Ok(inner) = self.current() else { return fresh_cache_token() };
        let Ok(mut slot) = inner.lock() else { return fresh_cache_token() };
        if slot.updating { return fresh_cache_token(); }
        if let Some((previous, token)) = slot.cache_memo {
            if previous == saved_changes { return token; }
        }
        let token = fresh_cache_token();
        slot.cache_memo = Some((saved_changes, token));
        token
    }

    fn current(&self) -> Result<Arc<Mutex<Slot<T>>>, &'static str> {
        Ok(Arc::clone(&self.root.lock().map_err(|_| "client block entity root poisoned")?.inner))
    }
}
impl<T: Default> Clone for ClientState<T> {
    fn clone(&self) -> Self { Self::default() }
}
impl<T: Clone> ClientState<T> {
    /// A renderer gets only a committed snapshot. Busy/poisoned is unavailable,
    /// not an empty/default compound or partially updated animation state.
    pub fn snapshot(&self) -> Result<T, &'static str> {
        let inner = self.current()?;
        let slot = inner.lock().map_err(|_| "client block entity state poisoned")?;
        if slot.updating { return Err("client block entity state busy"); }
        Ok(slot.value.clone())
    }

    /// The owned lease does not borrow the entity. A native adapter may release
    /// its world/entity borrow before executing a ticker that queries the world.
    /// Reentry refuses rather than deadlocking, and no ticker runs under a lock.
    pub fn begin(&self) -> Result<ClientUpdate<T>, &'static str> {
        let inner = self.current()?;
        let mut slot = inner.lock().map_err(|_| "client block entity state poisoned")?;
        if slot.updating { return Err("client block entity state busy"); }
        let value = slot.value.clone();
        slot.updating = true;
        drop(slot);
        Ok(ClientUpdate { inner, value: Some(value), active: true })
    }
}

/// Only a pending update shares the original instance's allocation. It cannot
/// accidentally publish into a replacement at the same position. Dropping it,
/// including during unwinding, leaves the last committed value untouched.
pub struct ClientUpdate<T> {
    inner: Arc<Mutex<Slot<T>>>,
    value: Option<T>,
    active: bool,
}
impl<T> ClientUpdate<T> {
    pub fn data_mut(&mut self) -> &mut T { self.value.as_mut().expect("unconsumed update") }

    pub fn commit(mut self) -> Result<(), &'static str> {
        let mut slot = self.inner.lock().map_err(|_| "client block entity state poisoned")?;
        let previous = std::mem::replace(&mut slot.value, self.value.take().expect("unconsumed update"));
        slot.updating = false;
        slot.cache_memo = None;
        self.active = false;
        drop(slot);
        // Plain data destructors run outside the lock as well.
        drop(previous);
        Ok(())
    }
}
impl<T> Drop for ClientUpdate<T> {
    fn drop(&mut self) {
        if self.active {
            if let Ok(mut slot) = self.inner.lock() { slot.updating = false; }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    #[test]
    fn same_level_respawn_preserves_animation_but_actual_replacement_detaches_old_updates() {
        let state = ClientState::<Vec<i32>>::default();
        state.set_client_active(true).unwrap();
        let mut update = state.begin().unwrap();
        update.data_mut().push(12);
        update.commit().unwrap();
        let token = state.cache_token(0);
        if client_level_replaced(true, Some("example:level_a"), "example:level_a") { state.reset().unwrap(); }
        assert_eq!(state.snapshot().unwrap(), [12]);
        assert_eq!(state.cache_token(0), token);
        for (keep, old, new) in [(true, Some("example:level_a"), "example:level_b"),
            (false, Some("example:level_b"), "example:level_b"), (true, None, "example:level_b")] {
            let mut stale = state.begin().unwrap();
            stale.data_mut().push(99);
            assert!(client_level_replaced(keep, old, new));
            state.reset().unwrap();
            stale.commit().unwrap();
            assert!(state.snapshot().unwrap().is_empty());
        }
    }

    #[test]
    fn identity_store_shares_one_state_per_identity_and_detaches_type_mismatches() {
        let first = client_state::<Vec<i32>>(1_001);
        let mut update = first.begin().unwrap();
        update.data_mut().push(5);
        update.commit().unwrap();
        assert!(Arc::ptr_eq(&first, &client_state::<Vec<i32>>(1_001)));
        assert_eq!(client_state::<Vec<i32>>(1_001).snapshot().unwrap(), [5]);
        let other = client_state::<Vec<i32>>(1_002);
        assert!(other.snapshot().unwrap().is_empty());
        // A different T for a live identity never sees the stored data.
        let mismatched = client_state::<BTreeMap<String, i32>>(1_001);
        assert!(mismatched.snapshot().unwrap().is_empty());
        assert_eq!(client_state::<Vec<i32>>(1_001).snapshot().unwrap(), [5]);
    }

    #[test]
    fn client_tick_gate_rejects_server_title_pause_frozen_and_missing_level() {
        for bits in 0..32 {
            let client = bits & 1 != 0;
            let level = bits & 2 != 0;
            let paused = bits & 4 != 0;
            let title = bits & 8 != 0;
            let running = bits & 16 != 0;
            assert_eq!(client_tick_enabled(client, level, paused, title, running), bits == 19);
        }
    }

    #[test]
    fn cache_generation_provider_preserves_unavailability_without_a_sentinel() {
        static AVAILABLE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        fn provider() -> Option<u64> { AVAILABLE.load(Ordering::Acquire).then_some(0) }
        assert_eq!(cache_generation(), None);
        set_cache_generation_provider(provider).unwrap();
        assert_eq!(cache_generation(), None);
        AVAILABLE.store(true, Ordering::Release);
        assert_eq!(cache_generation(), Some(0));
        AVAILABLE.store(false, Ordering::Release);
        assert_eq!(cache_generation(), None);
        assert!(set_cache_generation_provider(|| Some(99)).is_err());
        assert_eq!(cache_generation(), None);
    }

    #[test]
    fn client_targets_include_untemplated_instances_with_negative_relative_sections_and_no_server_queries() {
        let rows = [
            ([32, 0, 0], "example:clock", true), // nonresident section
            ([-1, -64, -17], "example:clock", true), // negative section coordinates
            ([0, 319, 0], "example:clock", true),
            ([-1, -64, -17], "example:clock", true), // duplicate, tick only once
            ([1, 64, 0], "example:clock_head", true), // no registered ticker, despite suffix
            ([2, 64, 0], "example:clock", false), // native entity, not Mod
            ([16, 0, 0], "example:clock", true), // unloaded origin
            ([0, 320, 0], "example:clock", true), // outside height
            ([48, 0, 0], "example:clock", true),
            ([48, 0, 0], "example:other", true), // conflicting snapshot, refuse
        ];
        let candidates = || rows.into_iter().map(|(pos, block, is_mod_entity)| ClientCandidate { pos, block, is_mod_entity });
        let mut sections = Vec::new();
        let selected = client_targets(candidates(), true, [-64, 320], |block| block == "example:clock",
            |pos| pos != [16, 0, 0], |section| { sections.push(section); section != [2, 4, 0] });
        assert_eq!(selected, [[-1, -64, -17], [0, 319, 0]]);
        assert!(sections.contains(&[-1, 0, -2]));
        assert!(sections.contains(&[0, 23, 0]));
        assert!(client_targets(candidates(), false, [-64, 320], |_| panic!("server registry query"),
            |_| panic!("server world query"), |_| panic!("server renderer query")).is_empty());
        assert!(client_targets([ClientCandidate { pos: [0, i32::MAX - 1, 0], block: "example:clock", is_mod_entity: true }],
            true, [i32::MIN, i32::MAX], |_| panic!("overflow registry query"),
            |_| panic!("overflow world query"), |_| panic!("overflow renderer query")).is_empty());
    }

    #[test]
    fn shared_residency_transitions_reset_once_without_mutable_native_entity_access() {
        let state = ClientState::<Vec<i32>>::default(); // deliberately immutable binding
        assert!(!state.set_client_active(false).unwrap());
        assert!(state.set_client_active(true).unwrap());
        let mut tick = state.begin().unwrap();
        tick.data_mut().push(7);
        tick.commit().unwrap();
        assert!(state.set_client_active(true).unwrap());
        assert_eq!(state.snapshot().unwrap(), [7]); // staying resident does not restart
        let mut previous = state.begin().unwrap();
        previous.data_mut().push(8);
        assert!(!state.set_client_active(false).unwrap());
        assert!(state.snapshot().unwrap().is_empty());
        assert!(state.set_client_active(true).unwrap());
        let mut current = state.begin().unwrap();
        current.data_mut().push(3);
        previous.commit().unwrap();
        assert!(state.snapshot().is_err()); // stale commit cannot release the new lease
        current.commit().unwrap();
        assert_eq!(state.snapshot().unwrap(), [3]);
        state.reset().unwrap(); // a real dimension/client-level transition
        assert!(state.snapshot().unwrap().is_empty());
    }

    #[test]
    fn cache_tokens_cover_saved_changes_commits_and_lifetimes_but_preserve_aborted_data() {
        let state = ClientState::<Vec<i32>>::default();
        let first = state.cache_token(0);
        assert_eq!(state.cache_token(0), first);
        let saved_change = state.cache_token(1);
        assert_ne!(saved_change, first);
        assert_eq!(state.cache_token(1), saved_change);
        let mut update = state.begin().unwrap();
        update.data_mut().push(7);
        assert_ne!(state.cache_token(1), state.cache_token(1)); // busy cannot be reused
        drop(update);
        assert_eq!(state.cache_token(1), saved_change); // abort did not change data
        let mut update = state.begin().unwrap();
        update.data_mut().push(8);
        update.commit().unwrap();
        let committed = state.cache_token(1);
        assert_ne!(committed, saved_change);
        assert_eq!(state.cache_token(1), committed);
        state.reset().unwrap();
        let reset = state.cache_token(1);
        assert_ne!(reset, committed);
        assert_eq!(state.cache_token(1), reset);
        assert_ne!(state.clone().cache_token(1), reset);
        assert_ne!(state.cache_token(0), first); // no stale key reuse on counter rewind
    }

    #[test]
    fn publication_is_atomic_reentry_refuses_and_aborted_or_panicked_ticks_rollback() {
        let state = ClientState::<BTreeMap<String, i32>>::default();
        let mut update = state.begin().unwrap();
        update.data_mut().insert("ticks".into(), 1);
        assert!(state.snapshot().is_err());
        assert!(state.begin().is_err());
        update.commit().unwrap();
        assert_eq!(state.snapshot().unwrap().get("ticks"), Some(&1));
        {
            let mut aborted = state.begin().unwrap();
            aborted.data_mut().insert("ticks".into(), 100);
        }
        assert_eq!(state.snapshot().unwrap().get("ticks"), Some(&1));
        let result = catch_unwind(AssertUnwindSafe(|| {
            let mut update = state.begin().unwrap();
            update.data_mut().insert("ticks".into(), 200);
            panic!("simulated ticker panic, outside state lock");
        }));
        assert!(result.is_err());
        assert_eq!(state.snapshot().unwrap().get("ticks"), Some(&1));
        let mut retry = state.begin().unwrap();
        retry.data_mut().insert("ticks".into(), 2);
        retry.commit().unwrap();
        assert_eq!(state.snapshot().unwrap().get("ticks"), Some(&2));
    }

    #[test]
    fn clones_and_replacements_are_fresh_and_detached_leases_do_not_keep_a_global_instance() {
        let state = ClientState::<Vec<i32>>::default();
        let mut first = state.begin().unwrap();
        first.data_mut().push(7);
        first.commit().unwrap();
        let same_entity = ClientState::<Vec<i32>>::default();
        let mut old_lifetime = same_entity.begin().unwrap();
        old_lifetime.data_mut().push(99);
        same_entity.reset().unwrap(); // client unload without deleting the server entity
        assert!(same_entity.snapshot().unwrap().is_empty());
        let mut fresh_lifetime = same_entity.begin().unwrap();
        fresh_lifetime.data_mut().push(3);
        old_lifetime.commit().unwrap(); // must not clear the new lease or publish 99
        assert!(same_entity.snapshot().is_err());
        fresh_lifetime.commit().unwrap();
        assert_eq!(same_entity.snapshot().unwrap(), [3]);
        same_entity.reset().unwrap();
        assert!(same_entity.snapshot().unwrap().is_empty());
        let copy = state.clone();
        assert!(copy.snapshot().unwrap().is_empty());
        assert_eq!(state.snapshot().unwrap(), [7]);
        let moved = state; // moving an instance, unlike cloning, preserves it
        assert_eq!(moved.snapshot().unwrap(), [7]);
        let weak = Arc::downgrade(&moved.current().unwrap());
        let mut pending = moved.begin().unwrap();
        pending.data_mut().push(8);
        drop(moved); // unload/replacement while the ticker still owns its lease
        assert!(weak.upgrade().is_some());
        let replacement = ClientState::<Vec<i32>>::default();
        pending.commit().unwrap(); // may finish only in the detached old instance
        assert!(weak.upgrade().is_none()); // no global registry/retained owner
        assert!(replacement.snapshot().unwrap().is_empty());
        assert!(copy.snapshot().unwrap().is_empty());
    }
}
