// A reversible projection of a Smeltery's real input slots into a 54-slot
// generic container. Callers must reject unsupported click modes before the
// inventory transaction and resync a stale view rather than consume items.
use horizon_items::items::ItemStack;
use super::tconstruct_source_smeltery_layout as layout;

pub(super) const MENU_SIZE: usize = 54;

pub(super) fn project(slots: &[Option<ItemStack>], offset: usize) -> Vec<Option<ItemStack>> {
    let mut view = vec![None; MENU_SIZE];
    for (menu_slot, item) in view.iter_mut().enumerate() {
        if let Some(index) = layout::melting_index(slots.len(), offset, menu_slot) {
            *item = slots[index].clone();
        }
    }
    view
}

/// Apply only a transaction based on the current page snapshot. The source
/// bucket slots are reserved for a separate transaction and must remain empty
/// here; all other padding is rejected rather than accepting a virtual item.
pub(super) fn commit(slots: &mut [Option<ItemStack>], offset: usize, expected: &[Option<ItemStack>], changed: &[Option<ItemStack>]) -> bool {
    if expected.len() != MENU_SIZE || changed.len() != MENU_SIZE || project(slots, offset) != expected { return false; }
    for (menu_slot, item) in changed.iter().enumerate() {
        if layout::melting_index(slots.len(), offset, menu_slot).is_none() && item.is_some() { return false; }
    }
    for (menu_slot, item) in changed.iter().enumerate() {
        if let Some(index) = layout::melting_index(slots.len(), offset, menu_slot) { slots[index] = item.clone(); }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn last_page_updates_only_real_slots() {
        let mut slots = vec![None; 71];
        slots[0] = Some(ItemStack::new("minecraft:stone", 1));
        let expected = project(&slots, 8);
        let mut changed = expected.clone();
        changed[40] = Some(ItemStack::new("minecraft:copper_ingot", 1));
        assert!(commit(&mut slots, 8, &expected, &changed));
        assert_eq!(slots[70].as_ref().unwrap().item.as_ref(), "minecraft:copper_ingot");
        assert_eq!(slots[0].as_ref().unwrap().item.as_ref(), "minecraft:stone");
    }
    #[test]
    fn rejects_ghost_and_stale_pages_without_changing_real_slots() {
        let mut slots = vec![None; 71];
        let expected = project(&slots, 8);
        let mut ghost = expected.clone(); ghost[41] = Some(ItemStack::new("minecraft:diamond", 1));
        assert!(!commit(&mut slots, 8, &expected, &ghost));
        assert!(slots.iter().all(Option::is_none));
        slots[32] = Some(ItemStack::new("minecraft:stone", 1));
        assert!(!commit(&mut slots, 8, &expected, &expected));
        assert_eq!(slots[32].as_ref().unwrap().item.as_ref(), "minecraft:stone");
    }
}
