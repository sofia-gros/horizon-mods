// Source-specific storage rules; callers supply component-aware equality and
// the resolved parts tag, so ordinary inventories keep their own limits.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ChestKind { Part, Tinkers, Plain(usize) }

impl ChestKind {
    pub(super) fn slots(self) -> usize {
        match self { Self::Part => 256, Self::Tinkers => 64, Self::Plain(slots) => slots }
    }
    pub(super) fn slot_limit(self, item_limit: u32) -> u32 {
        item_limit.min(match self { Self::Part => 8, Self::Tinkers => 16, Self::Plain(_) => item_limit })
    }
    /// `first_equal` must use the full item and component identity, not count.
    pub(super) fn accepts(self, slot: usize, first_equal: Option<usize>, tagged_part: bool) -> bool {
        if slot >= self.slots() { return false; }
        match self {
            Self::Tinkers | Self::Plain(_) => true,
            Self::Part => first_equal.map_or(tagged_part, |first| first == slot),
        }
    }
}

/// Place into a specific source chest slot. Return the untouched remainder;
/// item data is cloned as a whole and equality includes all components.
pub(super) fn insert_slot(
    kind: ChestKind, slots: &mut [Option<horizon_items::items::ItemStack>],
    slot: usize, mut incoming: horizon_items::items::ItemStack,
    item_limit: u32, tagged_part: bool,
) -> Option<horizon_items::items::ItemStack> {
    if slots.len() != kind.slots() || slot >= slots.len() || incoming.count == 0 { return Some(incoming); }
    let first_equal = slots.iter().position(|current| current.as_ref().is_some_and(|stack| stack.same_item_same_components(&incoming)));
    if !kind.accepts(slot, first_equal, tagged_part) { return Some(incoming); }
    let limit = kind.slot_limit(item_limit);
    let current = slots[slot].as_ref();
    if current.is_some_and(|stack| !stack.same_item_same_components(&incoming)) { return Some(incoming); }
    let room = limit.saturating_sub(current.map_or(0, |stack| stack.count));
    let moved = incoming.count.min(room);
    if moved == 0 { return Some(incoming); }
    if let Some(existing) = slots[slot].as_mut() { existing.count += moved; }
    else { let mut inserted = incoming.clone(); inserted.count = moved; slots[slot] = Some(inserted); }
    incoming.count -= moved;
    (incoming.count > 0).then_some(incoming)
}

/// Shift moves prefer a compatible stack before trying empty slots. Part
/// chests can have only one slot for each complete component identity.
pub(super) fn insert_across(
    kind: ChestKind, slots: &mut [Option<horizon_items::items::ItemStack>],
    incoming: horizon_items::items::ItemStack, item_limit: u32, tagged_part: bool,
) -> Option<horizon_items::items::ItemStack> {
    if slots.len() != kind.slots() { return Some(incoming); }
    let mut remaining = Some(incoming);
    for occupied in [true, false] {
        for slot in 0..slots.len() {
            if slots[slot].is_some() != occupied { continue; }
            let Some(stack) = remaining.take() else { return None; };
            remaining = insert_slot(kind, slots, slot, stack, item_limit, tagged_part);
            if remaining.is_none() { return None; }
        }
    }
    remaining
}

/// Source workstation results first refill existing side stacks, then use
/// player inventory, then try empty side slots.
pub(super) fn insert_existing(
    kind: ChestKind, slots: &mut [Option<horizon_items::items::ItemStack>],
    incoming: horizon_items::items::ItemStack, item_limit: u32, tagged_part: bool,
) -> Option<horizon_items::items::ItemStack> {
    let mut remaining = Some(incoming);
    for slot in 0..slots.len() {
        if slots[slot].is_none() { continue; }
        let Some(stack) = remaining.take() else { return None; };
        remaining = insert_slot(kind, slots, slot, stack, item_limit, tagged_part);
    }
    remaining
}

pub(super) fn plan_crafted_output(
    kind: ChestKind, side: &[Option<horizon_items::items::ItemStack>],
    inventory: &[Option<horizon_items::items::ItemStack>], output: horizon_items::items::ItemStack,
    item_limit: u32, tagged_part: bool,
    move_player: impl FnOnce(&mut [Option<horizon_items::items::ItemStack>],horizon_items::items::ItemStack) -> Option<horizon_items::items::ItemStack>,
) -> Option<(Vec<Option<horizon_items::items::ItemStack>>,Vec<Option<horizon_items::items::ItemStack>>,Option<horizon_items::items::ItemStack>)> {
    if side.len()!=kind.slots() || inventory.len()!=36 {return None;}
    let original=output.count;let mut side=side.to_vec();let mut inventory=inventory.to_vec();
    let rest=insert_existing(kind,&mut side,output,item_limit,tagged_part);
    let rest=rest.and_then(|stack|move_player(&mut inventory,stack));
    let rest=rest.and_then(|stack|insert_across(kind,&mut side,stack,item_limit,tagged_part));
    if rest.as_ref().map_or(0,|stack|stack.count)==original {return None;}
    Some((side,inventory,rest))
}

/// Apply a normal left/right click to a chest slot; all other input modes are
/// handled separately by the caller. Every stack mutation keeps its components.
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

/// Normal click in the player's own inventory while a source chest is open.
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

/// Double-click collection follows the visible menu order and leaves full
/// stacks for the second pass, matching the host container engine.
pub(super) fn pickup_all(
    side: &mut [Option<horizon_items::items::ItemStack>], visible: &[usize],
    inventory: &mut [Option<horizon_items::items::ItemStack>], carried: &mut Option<horizon_items::items::ItemStack>,
    reverse: bool, item_limit: u32,
) -> bool {
    let Some(cursor)=carried.as_mut() else {return false;};
    if cursor.count>=item_limit {return false;}
    let mut order:Vec<_>=visible.iter().copied().map(|i|(true,i)).chain((9..inventory.len()).chain(0..inventory.len().min(9)).map(|i|(false,i))).collect();
    if reverse {order.reverse();}
    let before=cursor.count;
    for full_pass in [false,true] {
        for &(is_side,index) in &order {
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

/// Hotbar/offhand swap against a chest slot, including a capped incoming
/// stack and return of the displaced item through the player inventory.
pub(super) fn swap_slot(
    kind: ChestKind, slots: &mut [Option<horizon_items::items::ItemStack>], index: usize,
    inventory: &mut [Option<horizon_items::items::ItemStack>], offhand: &mut Option<horizon_items::items::ItemStack>,
    button: i32, item_limit: u32, tagged_part: bool,
    move_player: impl FnOnce(&mut [Option<horizon_items::items::ItemStack>],horizon_items::items::ItemStack) -> Option<horizon_items::items::ItemStack>,
) -> Option<Option<horizon_items::items::ItemStack>> {
    if slots.len()!=kind.slots() || index>=slots.len() || inventory.len()<36 || !(button==40 || (0..9).contains(&button)) {return None;}
    let source=if button==40 {offhand.clone()} else {inventory[button as usize].clone()};
    if let Some(stack)=source.as_ref() {
        let first=slots.iter().position(|current|current.as_ref().is_some_and(|s|s.same_item_same_components(stack)));
        if !kind.accepts(index,first,tagged_part) {return None;}
    }
    let taken=slots[index].take();
    let mut displaced=None;
    match source {
        None => if button==40 {*offhand=taken;} else {inventory[button as usize]=taken;},
        Some(mut stack) => {
            let moved=stack.count.min(kind.slot_limit(item_limit));
            if moved==0 {slots[index]=taken;return None;}
            let mut placed=stack.clone();placed.count=moved;slots[index]=Some(placed);stack.count-=moved;
            if stack.count==0 {if button==40 {*offhand=taken;} else {inventory[button as usize]=taken;}}
            else {
                if button==40 {*offhand=Some(stack);} else {inventory[button as usize]=Some(stack);}
                if let Some(taken)=taken {displaced=move_player(inventory,taken);}
            }
        },
    }
    Some(displaced)
}

/// Original scaling inventory shows one spare slot after the last occupied
/// position, capped by its own capacity. It does not limit other inventories.
pub(super) fn part_visual_size(occupied: impl DoubleEndedIterator<Item = bool> + ExactSizeIterator, capacity: usize) -> usize {
    occupied.enumerate().rev().find_map(|(index, present)| present.then_some(index + 2))
        .unwrap_or(1).min(capacity)
}

/// A guest uses the existing 54-slot vanilla transport, with 32 real source
/// slots on the current page. Padding never maps to storage.
pub(super) fn project(slots: &[Option<horizon_items::items::ItemStack>], scroll: usize) -> Vec<Option<horizon_items::items::ItemStack>> {
    let mut out = vec![None;54]; let first=scroll*CHEST_COLUMNS;
    for (visible, target) in out.iter_mut().take(CHEST_COLUMNS*CHEST_ROWS).enumerate() {
        if let Some(item)=slots.get(first+visible) { *target=item.clone(); }
    }
    out
}

/// The side inventory uses six columns and at most eight visible rows.
/// Six further transport slots are padding for the existing double menu.
pub(super) fn project_side(slots: &[Option<horizon_items::items::ItemStack>], scroll: usize) -> Vec<Option<horizon_items::items::ItemStack>> {
    let mut out=vec![None;54]; let first=scroll*6;
    for (visible,target) in out.iter_mut().take(48).enumerate() {
        if let Some(item)=slots.get(first+visible) { *target=item.clone(); }
    }
    out
}

/// TinkerChestScreen's ScalingChestScreen: 162×72, 12-pixel slider,
/// 18-pixel cells, at (7,17) within the original 176-wide parent.
pub(super) const CHEST_COLUMNS: usize = 8;
pub(super) const CHEST_ROWS: usize = 4;
pub(super) fn max_scroll(visual_size: usize) -> usize {
    visual_size.div_ceil(CHEST_COLUMNS).saturating_sub(CHEST_ROWS)
}
pub(super) fn visible_slot(visual_size: usize, scroll: usize, slot: usize) -> Option<(f32, f32)> {
    let first = scroll.min(max_scroll(visual_size)) * CHEST_COLUMNS;
    if slot >= visual_size || slot < first || slot >= first + CHEST_COLUMNS * CHEST_ROWS { return None; }
    let shown = slot - first;
    Some((8.0 + (shown % CHEST_COLUMNS) as f32 * 18.0, 18.0 + (shown / CHEST_COLUMNS) as f32 * 18.0))
}
pub(super) fn hit_slot(visual_size: usize, scroll: usize, x: f32, y: f32) -> Option<usize> {
    if x < 8.0 || y < 18.0 { return None; }
    let col = ((x - 8.0) / 18.0).floor() as usize;
    let row = ((y - 18.0) / 18.0).floor() as usize;
    if col >= CHEST_COLUMNS || row >= CHEST_ROWS || (x - 8.0) % 18.0 >= 16.0 || (y - 18.0) % 18.0 >= 16.0 { return None; }
    let slot = scroll.min(max_scroll(visual_size)) * CHEST_COLUMNS + row * CHEST_COLUMNS + col;
    (slot < visual_size).then_some(slot)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn per_chest_limits_and_component_identity() {
        assert_eq!((ChestKind::Part.slots(), ChestKind::Tinkers.slots()), (256, 64));
        assert_eq!(ChestKind::Part.slot_limit(64), 8);
        assert_eq!(ChestKind::Tinkers.slot_limit(64), 16);
        assert_eq!(ChestKind::Part.slot_limit(1), 1);
        assert!(ChestKind::Part.accepts(7, None, true));
        assert!(!ChestKind::Part.accepts(7, None, false));
        assert!(ChestKind::Part.accepts(7, Some(7), true));
        assert!(!ChestKind::Part.accepts(8, Some(7), true));
        assert!(ChestKind::Tinkers.accepts(8, Some(7), false));
        assert!(!ChestKind::Tinkers.accepts(64, None, true));
    }
    #[test]
    fn inserts_keep_components_and_reject_duplicate_variants() {
        use horizon_items::items::ItemStack;
        use horizon_world::nbt::Tag;
        let mut wood = ItemStack::new("tconstruct:tool_handle", 11);
        wood.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("Material", Tag::String("tconstruct:wood".into()))])));
        let mut stone = wood.clone();
        stone.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("Material", Tag::String("tconstruct:stone".into()))])));
        let mut slots = vec![None; 256];
        let rest = insert_slot(ChestKind::Part, &mut slots, 0, wood.clone(), 64, true).unwrap();
        assert_eq!(rest.count, 3);
        assert_eq!(slots[0].as_ref().unwrap().count, 8);
        assert!(slots[0].as_ref().unwrap().same_item_same_components(&wood));
        assert_eq!(insert_slot(ChestKind::Part, &mut slots, 1, rest, 64, true).unwrap().count, 3);
        assert!(slots[1].is_none());
        assert_eq!(insert_slot(ChestKind::Part, &mut slots, 1, stone, 64, true).unwrap().count, 3);
        assert_eq!(slots[1].as_ref().unwrap().count, 8);
        let invalid = ItemStack::new("minecraft:stone", 2);
        assert_eq!(insert_slot(ChestKind::Part, &mut slots, 2, invalid, 64, false).unwrap().count, 2);
        let mut tinkers = vec![None; 64];
        assert_eq!(insert_slot(ChestKind::Tinkers, &mut tinkers, 0, ItemStack::new("minecraft:stone", 20), 64, false).unwrap().count, 4);
        assert_eq!(tinkers[0].as_ref().unwrap().count, 16);
    }
    #[test]
    fn player_inventory_pickup_preserves_overflow() {
        use horizon_items::items::ItemStack;
        let mut inv=vec![None;36]; let mut hand=Some(ItemStack::new("minecraft:stone", 8));
        assert!(pickup_player(&mut inv,0,&mut hand,true,64));
        assert_eq!((inv[0].as_ref().unwrap().count,hand.as_ref().unwrap().count),(1,7));
        assert!(pickup_player(&mut inv,0,&mut hand,false,64));
        assert!(hand.is_none()); assert_eq!(inv[0].as_ref().unwrap().count,8);
        assert!(pickup_player(&mut inv,0,&mut hand,true,64));
        assert_eq!((inv[0].as_ref().unwrap().count,hand.as_ref().unwrap().count),(4,4));
    }
    #[test]
    fn pickup_right_click_and_swap_respect_part_rules() {
        use horizon_items::items::ItemStack;
        let mut slots = vec![None; 256];
        let mut carried = Some(ItemStack::new("tconstruct:tool_handle", 10));
        assert!(pickup_slot(ChestKind::Part, &mut slots, 0, &mut carried, false, 64, true));
        assert_eq!((slots[0].as_ref().unwrap().count, carried.as_ref().unwrap().count), (8, 2));
        assert!(!pickup_slot(ChestKind::Part, &mut slots, 1, &mut carried, false, 64, true));
        assert_eq!(carried.as_ref().unwrap().count, 2);
        carried = None;
        assert!(pickup_slot(ChestKind::Part, &mut slots, 0, &mut carried, true, 64, true));
        assert_eq!((slots[0].as_ref().unwrap().count, carried.as_ref().unwrap().count), (4, 4));
        assert!(pickup_slot(ChestKind::Part, &mut slots, 0, &mut carried, true, 64, true));
        assert_eq!((slots[0].as_ref().unwrap().count, carried.as_ref().unwrap().count), (5, 3));
        carried = Some(ItemStack::new("minecraft:stone", 1));
        assert!(!pickup_slot(ChestKind::Part, &mut slots, 0, &mut carried, false, 64, false));
        assert_eq!(carried.as_ref().unwrap().item.as_ref(), "minecraft:stone");
        assert_eq!(slots[0].as_ref().unwrap().count, 5);
    }
    #[test]
    fn shift_move_merges_first_and_preserves_overflow() {
        use horizon_items::items::ItemStack;
        let mut parts = vec![None; 256];
        parts[7] = Some(ItemStack::new("tconstruct:tool_handle", 7));
        let remaining = insert_across(ChestKind::Part, &mut parts, ItemStack::new("tconstruct:tool_handle", 5), 64, true).unwrap();
        assert_eq!((parts[7].as_ref().unwrap().count, remaining.count), (8, 4));
        assert_eq!(parts.iter().flatten().count(), 1);
        let mut tinkers = vec![None; 64];
        tinkers[7] = Some(ItemStack::new("minecraft:stone", 15));
        assert!(insert_across(ChestKind::Tinkers, &mut tinkers, ItemStack::new("minecraft:stone", 5), 64, false).is_none());
        assert_eq!(tinkers[7].as_ref().unwrap().count, 16);
        assert_eq!(tinkers[0].as_ref().unwrap().count, 4);
    }
    #[test]
    fn refill_stage_keeps_empty_slots_for_after_player_inventory() {
        use horizon_items::items::ItemStack;
        let mut side=vec![None;64];side[7]=Some(ItemStack::new("minecraft:stone",15));
        let rest=insert_existing(ChestKind::Tinkers,&mut side,ItemStack::new("minecraft:stone",5),64,false).unwrap();
        assert_eq!((side[7].as_ref().unwrap().count,rest.count),(16,4));
        assert!(side[0].is_none());
    }
    #[test]
    fn plain_side_inventory_keeps_its_own_stack_limit() {
        use horizon_items::items::ItemStack;
        let mut side=vec![None;27];
        assert!(insert_across(ChestKind::Plain(27),&mut side,ItemStack::new("minecraft:stone",64),64,false).is_none());
        assert_eq!(side[0].as_ref().unwrap().count,64);
    }
    #[test]
    fn hotbar_swap_obeys_chest_limit_and_returns_displaced_stack() {
        use horizon_items::items::ItemStack;
        let mut side=vec![None;64];side[0]=Some(ItemStack::new("minecraft:diamond",2));
        let mut inventory=vec![None;36];inventory[1]=Some(ItemStack::new("minecraft:stone",20));let mut offhand=None;
        let result=swap_slot(ChestKind::Tinkers,&mut side,0,&mut inventory,&mut offhand,1,64,false,|inventory,stack|{inventory[2]=Some(stack);None});
        assert!(matches!(result,Some(None)));
        assert_eq!((side[0].as_ref().unwrap().count,inventory[1].as_ref().unwrap().count),(16,4));
        assert_eq!(inventory[2].as_ref().unwrap().item.as_ref(),"minecraft:diamond");
        let mut parts=vec![None;256];
        assert!(swap_slot(ChestKind::Part,&mut parts,0,&mut inventory,&mut offhand,1,64,false,|_,s|Some(s)).is_none());
    }
    #[test]
    fn dropped_single_item_keeps_its_components() {
        use horizon_items::items::ItemStack;
        use horizon_world::nbt::Tag;
        let mut stack=ItemStack::new("minecraft:stone",3);
        stack.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("mark",Tag::Int(4))])));
        let mut slots=vec![Some(stack.clone())];let taken=take_for_drop(&mut slots,0,false).unwrap();
        assert_eq!((taken.count,slots[0].as_ref().unwrap().count),(1,2));
        assert!(taken.same_item_same_components(&stack));
        assert_eq!(take_for_drop(&mut slots,0,true).unwrap().count,2);
        assert!(slots[0].is_none());
    }
    #[test]
    fn double_click_only_collects_visible_matching_items() {
        use horizon_items::items::ItemStack;
        let mut side=vec![None;64];side[0]=Some(ItemStack::new("minecraft:stone",4));side[40]=Some(ItemStack::new("minecraft:stone",7));
        let mut inventory=vec![None;36];inventory[9]=Some(ItemStack::new("minecraft:stone",3));let mut hand=Some(ItemStack::new("minecraft:stone",1));
        assert!(pickup_all(&mut side,&(0..32).collect::<Vec<_>>(),&mut inventory,&mut hand,false,64));
        assert_eq!(hand.unwrap().count,8);assert!(side[0].is_none());assert_eq!(side[40].as_ref().unwrap().count,7);
    }
    #[test]
    fn crafted_result_fills_side_then_player_and_rejects_full_destinations() {
        use horizon_items::items::ItemStack;
        let mut side=vec![None;64];side[0]=Some(ItemStack::new("minecraft:stone",15));
        let inventory=vec![None;36];
        let give=|inventory:&mut [Option<ItemStack>],mut stack:ItemStack| {
            let index=inventory.iter().position(Option::is_none)?;
            let count=stack.count.min(64);let mut placed=stack.clone();placed.count=count;inventory[index]=Some(placed);stack.count-=count;
            (stack.count>0).then_some(stack)
        };
        let (next,player,left)=plan_crafted_output(ChestKind::Tinkers,&side,&inventory,ItemStack::new("minecraft:stone",5),64,false,give).unwrap();
        assert_eq!(next[0].as_ref().unwrap().count,16);
        assert_eq!(player[0].as_ref().unwrap().count,4);
        assert!(next[1].is_none() && left.is_none());
        let full=vec![Some(ItemStack::new("minecraft:stone",16));64];
        let packed=vec![Some(ItemStack::new("minecraft:stone",64));36];
        assert!(plan_crafted_output(ChestKind::Tinkers,&full,&packed,ItemStack::new("minecraft:stone",1),64,false,|_,s|Some(s)).is_none());
    }
    #[test]
    fn guest_projection_has_only_real_page_slots() {
        use horizon_items::items::ItemStack;
        let mut slots=vec![None;256];slots[0]=Some(ItemStack::new("minecraft:stone",1));slots[255]=Some(ItemStack::new("minecraft:diamond",1));
        let first=project(&slots,0);let last=project(&slots,28);
        assert_eq!(first.len(),54);assert_eq!(first[0].as_ref().unwrap().item.as_ref(),"minecraft:stone");
        assert_eq!(last[31].as_ref().unwrap().item.as_ref(),"minecraft:diamond");assert!(last[32..].iter().all(Option::is_none));
    }
    #[test]
    fn builder_side_projection_never_exposes_transport_padding() {
        use horizon_items::items::ItemStack;
        let mut slots=vec![None;256];slots[255]=Some(ItemStack::new("tconstruct:tool_handle",1));
        let page=project_side(&slots,35);
        assert_eq!(page.len(),54);assert_eq!(page[45].as_ref().unwrap().item.as_ref(),"tconstruct:tool_handle");
        assert!(page[48..].iter().all(Option::is_none));
    }
    #[test]
    fn source_chest_slots_scroll_and_hit_the_same_visible_cells() {
        assert_eq!(max_scroll(256), 28);
        assert_eq!(max_scroll(64), 4);
        assert_eq!(max_scroll(1), 0);
        assert_eq!(visible_slot(256, 28, 255), Some((134.0, 72.0)));
        assert_eq!(hit_slot(256, 28, 134.0, 72.0), Some(255));
        assert_eq!(hit_slot(256, 28, 150.0, 72.0), None);
        assert_eq!(hit_slot(1, 0, 8.0, 18.0), Some(0));
        assert_eq!(hit_slot(1, 0, 26.0, 18.0), None);
        assert_eq!(visible_slot(256, 28, 223), None);
    }
    #[test]
    fn part_inventory_grows_and_shrinks_to_one_spare_slot() {
        assert_eq!(part_visual_size([false; 256].into_iter(), 256), 1);
        let mut slots = [false; 256]; slots[13] = true;
        assert_eq!(part_visual_size(slots.into_iter(), 256), 15);
        slots[255] = true;
        assert_eq!(part_visual_size(slots.into_iter(), 256), 256);
        slots[255] = false; slots[13] = false;
        assert_eq!(part_visual_size(slots.into_iter(), 256), 1);
    }
}
