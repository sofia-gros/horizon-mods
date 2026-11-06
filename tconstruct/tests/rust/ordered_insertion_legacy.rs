// Frozen unchanged consumer bodies; no native types are recreated.
use horizon_items::items::ItemStack;

pub(super) fn source_give_inventory(
    inventory: &mut [Option<ItemStack>], selected: usize, mut stack: ItemStack,
    max: u32, stackable: bool,
) -> Option<ItemStack> {
    if max == 0 { return Some(stack); }
    if stackable {
        let order = std::iter::once(selected).filter(|i| *i < inventory.len())
            .chain((0..inventory.len()).filter(|i| *i != selected)).collect::<Vec<_>>();
        for i in order {
            if let Some(existing) = &mut inventory[i] {
                if existing.same_item_same_components(&stack) && existing.count < max {
                    let moved = (max - existing.count).min(stack.count);
                    existing.count += moved;
                    stack.count -= moved;
                    if stack.count == 0 { return None; }
                }
            }
        }
    }
    for slot in inventory {
        if slot.is_none() {
            let moved = stack.count.min(max);
            *slot = Some(ItemStack { count: moved, ..stack.clone() });
            stack.count -= moved;
            if stack.count == 0 { return None; }
        }
    }
    Some(stack)
}

pub(super) fn source_quick_move_player(
    inventory: &mut [Option<ItemStack>], mut stack: ItemStack, max: u32, stackable: bool,
) -> Option<ItemStack> {
    if inventory.len() < 36 || max == 0 || stack.count == 0 { return Some(stack); }
    let order = (0..9).rev().chain((9..36).rev()).collect::<Vec<_>>();
    let kind = super::tconstruct_chest_rules::ChestKind::Plain(inventory.len());
    for occupied in [true, false] {
        if occupied && !stackable { continue; }
        for &i in &order {
            if inventory[i].is_some() != occupied { continue; }
            match super::tconstruct_chest_rules::insert_slot(kind, inventory, i, stack, max, true) {
                Some(remaining) => stack = remaining,
                None => return None,
            }
        }
    }
    Some(stack)
}
