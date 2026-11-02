// Source-specific storage rules; callers supply component-aware equality and
// the resolved parts tag, so ordinary inventories keep their own limits.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ChestKind { Part, Tinkers }

impl ChestKind {
    pub(super) fn slots(self) -> usize {
        match self { Self::Part => 256, Self::Tinkers => 64 }
    }
    pub(super) fn slot_limit(self, item_limit: u32) -> u32 {
        item_limit.min(match self { Self::Part => 8, Self::Tinkers => 16 })
    }
    /// `first_equal` must use the full item and component identity, not count.
    pub(super) fn accepts(self, slot: usize, first_equal: Option<usize>, tagged_part: bool) -> bool {
        if slot >= self.slots() { return false; }
        match self {
            Self::Tinkers => true,
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

/// Original scaling inventory shows one spare slot after the last occupied
/// position, capped by its own capacity. It does not limit other inventories.
pub(super) fn part_visual_size(occupied: impl DoubleEndedIterator<Item = bool> + ExactSizeIterator, capacity: usize) -> usize {
    occupied.enumerate().rev().find_map(|(index, present)| present.then_some(index + 2))
        .unwrap_or(1).min(capacity)
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
