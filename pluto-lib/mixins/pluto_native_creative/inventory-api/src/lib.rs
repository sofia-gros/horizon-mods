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
