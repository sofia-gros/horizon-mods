// Additive native value comparison including the private retained baselines.
// Existing observable snapshot callers and persistence policy stay unchanged.
use horizon_items::{item_data::pluto_wire_read, items::ItemStack};
use std::collections::HashSet;

/// Compare observable item state and every retained Wire decoded baseline.
///
/// This requires the actual Pluto native items read module. It adds private
/// baseline value checks to the existing observable comparator; it does not
/// replace that API or authorize a live write, transport, save or transaction.
/// Independently allocated equal values compare equal. Pointer pairs only
/// memoize comparisons after their values and all retained descendants passed.
///
/// Current native Wire baselines form immutable acyclic value graphs. Each
/// completed baseline-node pair is compared once. Host observable comparison
/// and traversal of each ordinary owned stack subtree retain their native cost;
/// this is not a constant-time comparison or a memory/size validation limit.
pub fn same_item_with_retained_baselines(before: &ItemStack, after: &ItemStack) -> bool {
    type Pair = (*const ItemStack, *const ItemStack);
    enum Work<'a> {
        Compare(&'a ItemStack, &'a ItemStack),
        Complete(Pair),
    }
    let mut completed = HashSet::new();
    let mut pending = vec![Work::Compare(before, after)];
    while let Some(work) = pending.pop() {
        let (before, after) = match work {
            Work::Complete(pair) => {
                completed.insert(pair);
                continue;
            }
            Work::Compare(before, after) => (before, after),
        };
        let pair = (before as *const ItemStack, after as *const ItemStack);
        if completed.contains(&pair) {
            continue;
        }
        // One owner for count/components, exposed wire facts and runtime bundle
        // selection. This also checks the ordinary tree shape before zipping.
        if !crate::pluto_stack_storage::same_item_snapshot(before, after) {
            return false;
        }
        // Postorder completion: every baseline discovered below must finish
        // before this pair may be skipped by a later repeated graph edge.
        pending.push(Work::Complete(pair));
        let mut ordinary = vec![(before, after)];
        while let Some((before, after)) = ordinary.pop() {
            let before_baseline = before
                .data()
                .and_then(|data| pluto_wire_read::retained(&data.wire));
            let after_baseline = after
                .data()
                .and_then(|data| pluto_wire_read::retained(&data.wire));
            match (before_baseline, after_baseline) {
                (None, None) => {}
                (Some((before, _)), Some((after, _))) => pending.push(Work::Compare(before, after)),
                _ => return false,
            }
            ordinary.extend(
                crate::pluto_stack_storage::nested_stacks(before)
                    .zip(crate::pluto_stack_storage::nested_stacks(after)),
            );
        }
    }
    true
}
