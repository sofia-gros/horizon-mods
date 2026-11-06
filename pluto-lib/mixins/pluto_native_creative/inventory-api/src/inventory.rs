//! Public entry points for inventory plans and detached storage proposals.
//!
//! These are re-exports, not a second implementation, registry or set of types.
//! Existing root imports and `storage::stage_updates` remain supported.
//! Consumers still choose admission, merge identity, limits and native commit
//! phases. Neither a plan nor a staged copy grants live inventory authority.

pub use crate::{plan_between, plan_move, MoveError, MovePlan, StackSnapshot};
pub use crate::storage::stage_updates;

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, collections::BTreeMap};

    #[test]
    fn root_and_public_entry_types_are_interchangeable() {
        let item = "example:quartz";
        let source: crate::StackSnapshot<'_, &str> = StackSnapshot { item: &item, count: 12 };
        let target: StackSnapshot<'_, &str> = crate::StackSnapshot { item: &item, count: 60 };
        let old_result: Result<Option<crate::MovePlan>, crate::MoveError> =
            plan_between(Some(source), Some(target), 12, |_, _| 64, |_, _| true, |a, b| a == b);
        let plan: MovePlan = old_result.unwrap().unwrap();
        let root_literal = crate::MovePlan {
            from: 0, to: 1, count: 4, source_count_before: 12,
            destination_count_before: Some(60),
        };
        assert_eq!(plan, root_literal);
        let entry_literal: MovePlan = root_literal;
        let _: crate::MovePlan = entry_literal;
        let old_error: crate::MoveError = MoveError::SlotOutOfBounds;
        let _: MoveError = old_error;
    }

    #[test]
    fn plan_move_reexport_keeps_results_errors_and_callback_order() {
        let item = 7u8;
        let slots = [Some(StackSnapshot { item: &item, count: 12 }),
            Some(StackSnapshot { item: &item, count: 60 })];
        let root_trace = RefCell::new(Vec::new());
        let entry_trace = RefCell::new(Vec::new());
        let old = crate::plan_move(&slots, 0, 1, 12,
            |_, _| { root_trace.borrow_mut().push("limit"); 64 },
            |_, _| { root_trace.borrow_mut().push("accepts"); true },
            |a, b| { root_trace.borrow_mut().push("same"); a == b });
        let public = plan_move(&slots, 0, 1, 12,
            |_, _| { entry_trace.borrow_mut().push("limit"); 64 },
            |_, _| { entry_trace.borrow_mut().push("accepts"); true },
            |a, b| { entry_trace.borrow_mut().push("same"); a == b });
        assert_eq!(public, old);
        assert_eq!(public.unwrap().unwrap().count, 4);
        assert_eq!(*entry_trace.borrow(), *root_trace.borrow());
        assert_eq!(*entry_trace.borrow(), ["accepts", "same", "limit"]);
        assert_eq!(plan_move(&slots, 9, 1, 12, |_, _| 64, |_, _| true, |a, b| a == b),
            Err(MoveError::SlotOutOfBounds));
        assert_eq!(slots[0].as_ref().unwrap().count, 12);
        assert_eq!(slots[1].as_ref().unwrap().count, 60);
    }

    #[test]
    fn pair_reexport_keeps_logical_indexes_and_zero_request_behavior() {
        let item = 5u8;
        let root_snapshot = crate::StackSnapshot { item: &item, count: 3 };
        let public = plan_between(Some(root_snapshot), None, 2,
            |_, _| 64, |_, _| true, |a, b| a == b).unwrap().unwrap();
        let entry_snapshot = StackSnapshot { item: &item, count: 3 };
        let old = crate::plan_between(Some(entry_snapshot), None, 2,
            |_, _| 64, |_, _| true, |a, b| a == b).unwrap().unwrap();
        assert_eq!(public, old);
        assert_eq!((public.from, public.to, public.count), (0, 1, 2));
        assert_eq!(public.destination_count_before, None);
        assert_eq!(plan_between(Some(StackSnapshot { item: &item, count: 3 }), None, 0,
            |_, _| panic!("zero request must not query a limit"),
            |_, _| panic!("zero request must not query admission"),
            |_, _| panic!("zero request must not query identity")), Ok(None));
    }

    #[test]
    fn staging_reexport_keeps_independent_copies_order_and_rejection() {
        let live = BTreeMap::from([(1u8, vec![10]), (3u8, vec![30])]);
        let before = live.clone();
        let changes = [(3u8, 31), (1u8, 11), (3u8, 32)];
        let old = crate::storage::stage_updates(changes, |change| change.0,
            |key| live.get(key).cloned(), |copy, (_, value)| { copy.push(value); true });
        let loads = RefCell::new(Vec::new());
        let applied = RefCell::new(Vec::new());
        let public = stage_updates(changes, |change| change.0,
            |key| { loads.borrow_mut().push(*key); live.get(key).cloned() },
            |copy, (key, value)| { applied.borrow_mut().push((key, value)); copy.push(value); true });
        assert_eq!(public, old);
        assert_eq!(*loads.borrow(), [3, 1]);
        assert_eq!(*applied.borrow(), changes);
        assert_eq!(public.unwrap().into_keys().collect::<Vec<_>>(), [1, 3]);
        assert!(stage_updates([(1u8, 12), (9u8, 99)], |change| change.0,
            |key| live.get(key).cloned(), |copy, (_, value)| { copy.push(value); true }).is_none());
        assert!(stage_updates([(1u8, 12), (3u8, 33)], |change| change.0,
            |key| live.get(key).cloned(), |copy, (key, value)| { copy.push(value); key != 3 }).is_none());
        assert_eq!(live, before);
    }
}
