// Frozen complete insertion helper; native types and ChestKind policy are not recreated.
use crate::consumer_impl::ChestKind;

pub(super) fn insert_slot(
    kind: ChestKind, slots: &mut [Option<horizon_items::items::ItemStack>],
    slot: usize, mut incoming: horizon_items::items::ItemStack,
    item_limit: u32, tagged_part: bool,
) -> Option<horizon_items::items::ItemStack> {
    if slots.len() != kind.slots() || slot >= slots.len() || incoming.count == 0 { return Some(incoming); }
    use super::tconstruct_inventory_api::{plan_move, StackSnapshot};
    // The last entry is a borrowed, synthetic source, not a container slot.
    let mut snapshot: Vec<_> = slots.iter().map(|cell| cell.as_ref().map(|stack| StackSnapshot { item: stack, count: stack.count })).collect();
    let source = snapshot.len();
    snapshot.push(Some(StackSnapshot { item: &incoming, count: incoming.count }));
    let first_equal = slots.iter().position(|cell| cell.as_ref().is_some_and(|stack| stack.same_item_same_components(&incoming)));
    let limit = kind.slot_limit(item_limit);
    let Ok(Some(plan)) = plan_move(&snapshot, source, slot, incoming.count,
        |_, _| limit, |target, _| kind.accepts(target, first_equal, tagged_part),
        |left, right| left.same_item_same_components(right)) else { return Some(incoming); };
    // This function has exclusive access to both inputs through planning and
    // commit. Check the planner's original counts and use only its move count;
    // `plan.from` identifies the synthetic source, never index it in `slots`.
    if plan.from != source || plan.to != slot || plan.source_count_before != incoming.count
        || plan.destination_count_before != slots[slot].as_ref().map(|stack| stack.count) {
        return Some(incoming);
    }
    if let Some(existing) = slots[slot].as_mut() { existing.count += plan.count; }
    else { let mut inserted = incoming.clone(); inserted.count = plan.count; slots[slot] = Some(inserted); }
    incoming.count -= plan.count;
    (incoming.count > 0).then_some(incoming)
}
