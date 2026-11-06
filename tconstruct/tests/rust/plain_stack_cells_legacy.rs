// Frozen byte-exact function bodies from source-tinker-chests.rs at
// 921bb12dc23caea0c970c5bcc39b72921e98981c. Differential oracle only.

pub(super) fn pickup_player(
    inventory: &mut [Option<horizon_items::items::ItemStack>], slot: usize,
    carried: &mut Option<horizon_items::items::ItemStack>, right: bool, item_limit: u32,
) -> bool {
    let Some(cell) = inventory.get_mut(slot) else { return false; };
    let Some(mut cursor) = carried.take() else {
        let Some(mut existing) = cell.take() else { return false; };
        if right && existing.count > 1 {
            let count=existing.count.div_ceil(2); let mut taken=existing.clone(); taken.count=count;
            existing.count-=count; *cell=Some(existing); *carried=Some(taken);
        } else { *carried=Some(existing); }
        return true;
    };
    if let Some(existing) = cell {
        if existing.same_item_same_components(&cursor) {
            let wanted=if right {1} else {cursor.count};
            let moved=wanted.min(item_limit.saturating_sub(existing.count));
            existing.count+=moved; cursor.count-=moved;
            if cursor.count>0 { *carried=Some(cursor); }
            return moved>0;
        }
        if cursor.count>item_limit { *carried=Some(cursor); return false; }
        *carried=cell.replace(cursor); return true;
    }
    let moved=if right {1} else {cursor.count.min(item_limit)};
    if moved==0 { *carried=Some(cursor); return false; }
    let mut inserted=cursor.clone(); inserted.count=moved; *cell=Some(inserted);
    cursor.count-=moved; if cursor.count>0 { *carried=Some(cursor); }
    true
}

pub(super) fn take_for_drop(slots: &mut [Option<horizon_items::items::ItemStack>], index: usize, all: bool) -> Option<horizon_items::items::ItemStack> {
    let cell=slots.get_mut(index)?;let mut stack=cell.take()?;
    if all || stack.count<=1 {return Some(stack);}
    stack.count-=1;let mut dropped=stack.clone();dropped.count=1;*cell=Some(stack);Some(dropped)
}

pub(super) fn pickup_all_ordered(
    side: &mut [Option<horizon_items::items::ItemStack>], inventory: &mut [Option<horizon_items::items::ItemStack>],
    carried: &mut Option<horizon_items::items::ItemStack>, order: &[(bool,usize)],item_limit: u32,
) -> bool {
    let Some(cursor)=carried.as_mut() else {return false;};
    if cursor.count>=item_limit {return false;}
    let before=cursor.count;
    for full_pass in [false,true] {
        for &(is_side,index) in order {
            let cell=if is_side {side.get_mut(index)} else {inventory.get_mut(index)};
            let Some(cell)=cell else {continue;};
            let Some(stack)=cell.as_mut() else {continue;};
            if !stack.same_item_same_components(cursor) || (!full_pass && stack.count>=item_limit) {continue;}
            let moved=(item_limit-cursor.count).min(stack.count);cursor.count+=moved;stack.count-=moved;
            if stack.count==0 {*cell=None;}
            if cursor.count==item_limit {return cursor.count!=before;}
        }
    }
    cursor.count!=before
}

use crate::consumer::{ChestKind, insert_slot};

pub(super) fn pickup_slot(
    kind: ChestKind, slots: &mut [Option<horizon_items::items::ItemStack>],
    slot: usize, carried: &mut Option<horizon_items::items::ItemStack>,
    right: bool, item_limit: u32, tagged_part: bool,
) -> bool {
    if slots.len() != kind.slots() || slot >= slots.len() { return false; }
    let Some(mut cursor) = carried.take() else {
        let Some(mut existing) = slots[slot].take() else { return false; };
        if right && existing.count > 1 {
            let count = existing.count.div_ceil(2);
            let mut taken = existing.clone(); taken.count = count;
            existing.count -= count; slots[slot] = Some(existing); *carried = Some(taken);
        } else { *carried = Some(existing); }
        return true;
    };
    if slots[slot].as_ref().is_some_and(|existing| !existing.same_item_same_components(&cursor)) {
        let first = slots.iter().position(|current| current.as_ref().is_some_and(|s| s.same_item_same_components(&cursor)));
        if !kind.accepts(slot, first, tagged_part) || cursor.count > kind.slot_limit(item_limit) { *carried = Some(cursor); return false; }
        *carried = slots[slot].replace(cursor);
        return true;
    }
    let requested = if right { 1 } else { cursor.count };
    let mut offered = cursor.clone(); offered.count = requested;
    let remainder = insert_slot(kind, slots, slot, offered, item_limit, tagged_part);
    let unmoved = remainder.map_or(0, |stack| stack.count);
    let moved = requested - unmoved;
    cursor.count -= moved;
    if cursor.count != 0 { *carried = Some(cursor); }
    moved != 0
}
