//! Plan a transfer between two logical slots without changing the inventory.
//!
//! The acceptance, identity and effective stack limit are supplied by the
//! consumer. Adapted from the Tinkers chest's `insert_slot` at repository
//! commit 45d91474730938ce9d93af65a5d71abafbb4aea9.

/// A borrowed stack identity and its count; `item` may include component data.
#[derive(Debug)]
pub struct StackSnapshot<'a, T> {
    pub item: &'a T,
    pub count: u32,
}

/// An immutable move proposal. Recheck both original stacks and the applicable
/// rules before applying it to a live inventory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MovePlan {
    pub from: usize,
    pub to: usize,
    pub count: u32,
    pub source_count_before: u32,
    pub destination_count_before: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveError {
    SlotOutOfBounds,
}
impl std::fmt::Display for MoveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SlotOutOfBounds => f.write_str("inventory slot is out of bounds"),
        }
    }
}
impl std::error::Error for MoveError {}

/// Plan a move without mutating the snapshot. `limit` is the effective limit
/// for this item in the destination (including any slot-specific limit).
/// `accepts` decides whether the destination accepts the source identity;
/// `same` must compare the complete stack identity, excluding count.
pub fn plan_move<T>(
    slots: &[Option<StackSnapshot<'_, T>>],
    from: usize,
    to: usize,
    requested: u32,
    limit: impl FnOnce(usize, &T) -> u32,
    accepts: impl FnOnce(usize, &T) -> bool,
    same: impl FnOnce(&T, &T) -> bool,
) -> Result<Option<MovePlan>, MoveError> {
    let source = slots.get(from).ok_or(MoveError::SlotOutOfBounds)?;
    let destination = slots.get(to).ok_or(MoveError::SlotOutOfBounds)?;
    if from == to || requested == 0 {
        return Ok(None);
    }
    let Some(source) = source else {
        return Ok(None);
    };
    if source.count == 0 || !accepts(to, source.item) {
        return Ok(None);
    }
    if destination
        .as_ref()
        .is_some_and(|target| !same(source.item, target.item))
    {
        return Ok(None);
    }
    let target_count = destination.as_ref().map_or(0, |target| target.count);
    let count = source
        .count
        .min(requested)
        .min(limit(to, source.item).saturating_sub(target_count));
    if count == 0 {
        return Ok(None);
    }
    Ok(Some(MovePlan {
        from,
        to,
        count,
        source_count_before: source.count,
        destination_count_before: destination.as_ref().map(|target| target.count),
    }))
}

/// Plan a move between snapshots owned by different containers. The caller
/// revalidates and commits both live containers as one transaction.
pub fn plan_between<T>(
    source: Option<StackSnapshot<'_, T>>,
    destination: Option<StackSnapshot<'_, T>>,
    requested: u32,
    limit: impl FnOnce(usize, &T) -> u32,
    accepts: impl FnOnce(usize, &T) -> bool,
    same: impl FnOnce(&T, &T) -> bool,
) -> Result<Option<MovePlan>, MoveError> {
    plan_move(&[source, destination], 0, 1, requested, limit, accepts, same)
}

/// Select the first approved candidate without committing any inventory changes.
///
/// `check` runs once per candidate, in input order. `Some(true)` selects it,
/// `Some(false)` skips a canceled candidate, and `None` aborts the entire
/// attempt (for example, when a callback invalidates a snapshot). The caller
/// owns cancellation rules, snapshot validation, and the eventual commit.
/// Adapted from TConstruct's casting hopper transaction at `08afa992`.
pub fn select_candidate<T>(
    candidates: impl IntoIterator<Item = (usize, T)>,
    mut check: impl FnMut(usize, &T) -> Option<bool>,
) -> Option<(usize, T)> {
    for (index, item) in candidates {
        match check(index, &item) {
            Some(true) => return Some((index, item)),
            Some(false) => continue,
            None => return None,
        }
    }
    None
}

/// Recheck an exact snapshot, rejecting a missing live value.
///
/// Equality must cover every field relevant to the transaction. Pass the raw
/// persisted state as well as any decoded inventory when decoding may discard
/// fields. This is only a comparison: it does not lock, write, or roll back
/// storage. Recheck after callbacks and commit in one authoritative operation.
pub fn snapshot_matches<T: PartialEq + ?Sized>(expected: &T, current: Option<&T>) -> bool {
    current == Some(expected)
}

/// Encode a value only when the supplied decoder preserves its full identity.
///
/// The input is borrowed and never replaced. `same` must compare all relevant
/// state, including count, damage, and components for item stacks. Reject any
/// opaque state not covered by that comparison before calling this function.
/// This does not prove a network round trip or preserve fields already lost
/// when reading raw storage; compare the original raw encoding separately.
pub fn encode_lossless<T, E>(
    value: &T,
    encode: impl FnOnce(&T) -> E,
    decode: impl FnOnce(&E) -> Option<T>,
    same: impl FnOnce(&T, &T) -> bool,
) -> Option<E> {
    let encoded = encode(value);
    let decoded = decode(&encoded)?;
    same(value, &decoded).then_some(encoded)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(PartialEq, Eq)]
    struct Item(&'static str, u8);
    fn stack(item: &Item, count: u32) -> Option<StackSnapshot<'_, Item>> {
        Some(StackSnapshot { item, count })
    }
    #[test]
    fn separate_container_plan_respects_identity_and_limits() {
        let source = Item("example:part", 1);
        let same = Item("example:part", 1);
        let different = Item("example:part", 2);
        let plan = plan_between(stack(&source, 4), stack(&same, 63), 4,
            |_, _| 64, |_, _| true, |a, b| a == b).unwrap().unwrap();
        assert_eq!((plan.count, plan.source_count_before, plan.destination_count_before), (1, 4, Some(63)));
        assert!(plan_between(stack(&source, 4), stack(&different, 1), 1,
            |_, _| 64, |_, _| true, |a, b| a == b).unwrap().is_none());
        assert!(plan_between(stack(&source, 4), None, 1,
            |_, _| 1, |_, _| false, |a, b| a == b).unwrap().is_none());
    }
    fn plan(
        slots: &[Option<StackSnapshot<'_, Item>>],
        from: usize,
        to: usize,
        requested: u32,
        max: u32,
        allowed: bool,
    ) -> Result<Option<MovePlan>, MoveError> {
        plan_move(
            slots,
            from,
            to,
            requested,
            |_, _| max,
            |_, _| allowed,
            |a, b| a == b,
        )
    }
    #[test]
    fn moves_to_empty_and_respects_limit_and_requested_count() {
        let item = Item("example:gem", 1);
        let slots = [stack(&item, 20), None];
        let plan = plan(&slots, 0, 1, 7, 4, true).unwrap().unwrap();
        assert_eq!(
            (
                plan.count,
                plan.source_count_before,
                plan.destination_count_before
            ),
            (4, 20, None)
        );
        assert_eq!(slots[0].as_ref().unwrap().count, 20);
        assert!(slots[1].is_none());
    }
    #[test]
    fn merges_only_matching_complete_identity() {
        let item = Item("example:gem", 1);
        let equivalent = Item("example:gem", 1);
        let different = Item("example:gem", 2);
        let slots = [
            stack(&item, 12),
            stack(&equivalent, 62),
            stack(&different, 1),
        ];
        assert_eq!(plan(&slots, 0, 1, 12, 64, true).unwrap().unwrap().count, 2);
        assert_eq!(plan(&slots, 0, 2, 12, 64, true), Ok(None));
    }
    #[test]
    fn preserves_partial_move_and_rejection_from_chest_fixture() {
        // Same quantities as the existing Part Chest case; limits and rules
        // remain consumer inputs rather than fixed library policy.
        let item = Item("example:handle", 1);
        let slots = [stack(&item, 11), None, None];
        let moved = plan(&slots, 0, 1, 11, 8, true).unwrap().unwrap();
        assert_eq!(moved.count, 8);
        assert_eq!(moved.source_count_before - moved.count, 3);
        assert_eq!(plan(&slots, 0, 2, 3, 8, false), Ok(None));
    }
    #[test]
    fn rejects_disallowed_full_empty_same_and_out_of_range() {
        let item = Item("example:gem", 1);
        let slots = [stack(&item, 3), stack(&item, 4), None];
        assert_eq!(plan(&slots, 0, 1, 3, 4, true), Ok(None));
        assert_eq!(plan(&slots, 0, 2, 3, 64, false), Ok(None));
        assert_eq!(plan(&slots, 0, 0, 3, 64, true), Ok(None));
        assert_eq!(plan(&slots, 2, 0, 3, 64, true), Ok(None));
        assert_eq!(plan(&slots, 0, 2, 0, 64, true), Ok(None));
        assert_eq!(plan(&slots, 0, 2, 3, 0, true), Ok(None));
        assert_eq!(
            plan(&slots, 0, 3, 3, 64, true),
            Err(MoveError::SlotOutOfBounds)
        );
    }
}

#[cfg(test)]
mod transaction_tests {
    use super::*;

    #[test]
    fn canceled_candidate_tries_next_but_stale_snapshot_aborts() {
        let mut visited = Vec::new();
        let choice = select_candidate([(0, "first"), (1, "second"), (2, "third")], |index, _| {
            visited.push(index);
            Some(index == 1)
        });
        assert_eq!(choice, Some((1, "second")));
        assert_eq!(visited, vec![0, 1]);
        visited.clear();
        let choice = select_candidate([(0, "first"), (1, "second"), (2, "third")], |index, _| {
            visited.push(index);
            if index == 0 { Some(false) } else { None }
        });
        assert_eq!(choice, None);
        assert_eq!(visited, vec![0, 1]);
    }

    #[test]
    fn empty_or_all_canceled_candidates_never_select() {
        assert_eq!(select_candidate(Vec::<(usize, ())>::new(), |_, _| panic!("empty")), None);
        let source = [(1, "first"), (3, "second")];
        assert_eq!(select_candidate(source, |_, _| Some(false)), None);
        assert_eq!(source, [(1, "first"), (3, "second")]);
    }

    #[test]
    fn snapshots_require_present_exact_live_state() {
        let before = vec![Some(("test:item", 2, 1, "custom")), None];
        assert!(snapshot_matches(before.as_slice(), Some(before.as_slice())));
        assert!(!snapshot_matches(before.as_slice(), None));
        for changed in [
            vec![Some(("test:item", 3, 1, "custom")), None],
            vec![Some(("test:item", 2, 2, "custom")), None],
            vec![Some(("test:item", 2, 1, "changed")), None],
            vec![Some(("test:item", 2, 1, "custom"))],
        ] {
            assert!(!snapshot_matches(before.as_slice(), Some(changed.as_slice())));
        }
        // A decoded value may still match when the original raw storage changed.
        assert!(!snapshot_matches("{item:1,extension:old}", Some("{item:1,extension:new}")));
        assert!(!snapshot_matches::<[u8]>(&[], None));
        assert!(snapshot_matches::<[u8]>(&[], Some(&[])));
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Value { count: u32, damage: u32, components: Vec<u8> }

    #[test]
    fn encoding_requires_full_roundtrip_and_keeps_the_input() {
        let source = Value { count: 2, damage: 7, components: vec![4, 9] };
        let before = source.clone();
        assert_eq!(encode_lossless(&source, Clone::clone, |v| Some(v.clone()), |a, b| a == b), Some(before.clone()));
        assert_eq!(encode_lossless(&source, Clone::clone, |_| None, |a, b| a == b), None);
        for field in 0..3 {
            assert_eq!(encode_lossless(&source, Clone::clone, |v| {
                let mut decoded = v.clone();
                match field {
                    0 => decoded.count += 1,
                    1 => decoded.damage += 1,
                    _ => decoded.components.pop().map(|_| ()).unwrap(),
                }
                Some(decoded)
            }, |a, b| a == b), None);
        }
        assert_eq!(source, before);
    }

    #[test]
    fn failed_decode_never_calls_identity_comparison() {
        assert_eq!(encode_lossless(&5, |n| n.to_string(), |_| None,
            |_, _| panic!("decode failed")), None);
    }
}
/// Shared data scopes and component-aware inventory event keys.
pub mod advancements;

/// Non-mutating proposal staging for caller-owned partitioned storage.
pub mod storage;

/// Host-independent read-only raised-item block events.
pub mod blocking;

/// Shared read-only actor and equipment facts.
pub mod equipment;

/// Read-only observations after native authoritative armor-wear consumption.
pub mod armor_wear;

/// Read-only inventory views with caller-supplied slot mappings.
pub mod projection;

/// Scoped recipe-book presentation cache invalidation.
pub mod recipe_book;

/// Host-independent protection providers and scoped policy contributions.
pub mod protection;
/// Bounded portable read-only guide documents; no content or screen ownership.
pub mod guide;

/// Opt-in read-only clientbound play custom-payload observations.
pub mod client_payload;

/// Deny-only native crafting admission without output or inventory authority.
pub mod crafting;

/// Ordered lazy three-valued predicate folds, without schema or leaf policy.
pub mod predicate_fold;

/// Opt-in Java linked-bin ordering compatibility, without resource policy.
pub mod java_hashmap;
