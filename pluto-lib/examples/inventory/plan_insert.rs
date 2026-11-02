//! Adapt an existing container's incoming stack to the two-slot planner.
//! This only plans the insertion; the caller owns and commits the transaction.
use horizon_client::items::ItemStack;
use horizon_client::pluto_inventory::{plan_move, MoveError, MovePlan, StackSnapshot};

pub fn plan_insert(
    slots: &[Option<ItemStack>],
    incoming: &ItemStack,
    destination: usize,
    effective_limit: u32,
    accepts: impl FnOnce(usize, &ItemStack) -> bool,
) -> Result<Option<MovePlan>, MoveError> {
    if destination >= slots.len() {
        return Err(MoveError::SlotOutOfBounds);
    }
    // Add the caller-owned incoming stack as a temporary source slot. Its
    // index is not a slot in the container and must not be applied to it.
    let mut snapshot: Vec<_> = slots
        .iter()
        .map(|slot| {
            slot.as_ref().map(|stack| StackSnapshot {
                item: stack,
                count: stack.count,
            })
        })
        .collect();
    let source = snapshot.len();
    snapshot.push(Some(StackSnapshot {
        item: incoming,
        count: incoming.count,
    }));
    plan_move(
        &snapshot,
        source,
        destination,
        incoming.count,
        |_, _| effective_limit,
        accepts,
        |left, right| left.same_item_same_components(right),
    )
}
