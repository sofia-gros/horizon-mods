use horizon_client::items::ItemStack;
use horizon_client::pluto_inventory::{plan_move, MovePlan, StackSnapshot};

fn plan(slots: &[Option<ItemStack>], from: usize, to: usize, accept: bool) -> Option<MovePlan> {
    let snapshot: Vec<_> = slots
        .iter()
        .map(|slot| {
            slot.as_ref().map(|stack| StackSnapshot {
                item: stack,
                count: stack.count,
            })
        })
        .collect();
    plan_move(
        &snapshot,
        from,
        to,
        u32::MAX,
        |_, _| 8,
        |_, _| accept,
        |left, right| left.same_item_same_components(right),
    )
    .ok()
    .flatten()
}

fn apply(slots: &mut [Option<ItemStack>], plan: MovePlan) -> bool {
    let Some(source) = slots.get(plan.from).and_then(Option::as_ref) else {
        return false;
    };
    let Some(target) = slots.get(plan.to) else {
        return false;
    };
    if source.count != plan.source_count_before
        || target.as_ref().map(|s| s.count) != plan.destination_count_before
        || target
            .as_ref()
            .is_some_and(|s| !s.same_item_same_components(source))
    {
        return false;
    }
    let mut moved = source.clone();
    moved.count = plan.count;
    let remaining = source.count - plan.count;
    if remaining == 0 {
        slots[plan.from] = None;
    } else {
        slots[plan.from].as_mut().unwrap().count = remaining;
    }
    if let Some(target) = slots[plan.to].as_mut() {
        target.count += plan.count;
    } else {
        slots[plan.to] = Some(moved);
    }
    true
}

/// Demo only: use a cloned transaction, then commit if every check succeeds.
pub fn run(inventory: &mut [Option<ItemStack>]) -> bool {
    if inventory.len() < 6 {
        return false;
    }
    let mut staged = inventory.to_vec();
    let (Some(source), Some(equal), Some(different)) = (&staged[1], &staged[2], &staged[4]) else {
        return false;
    };
    if source.count != 10
        || equal.count != 6
        || different.count != 1
        || staged[3].is_some()
        || staged[5].is_some()
        || !source.same_item_same_components(equal)
        || source.same_item_same_components(different)
    {
        return false;
    }
    let Some(merge) = plan(&staged, 1, 2, true) else {
        return false;
    };
    if merge.count != 2
        || plan(&staged, 1, 4, true).is_some()
        || plan(&staged, 1, 5, false).is_some()
        || plan(&staged, 1, 1, true).is_some()
        || !apply(&mut staged, merge)
    {
        return false;
    }
    let Some(empty) = plan(&staged, 1, 3, true) else {
        return false;
    };
    if empty.count != 8 || !apply(&mut staged, empty) {
        return false;
    }
    inventory.clone_from_slice(&staged);
    true
}
