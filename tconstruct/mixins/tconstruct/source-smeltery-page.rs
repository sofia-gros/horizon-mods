// A reversible projection of a Smeltery's real input slots into a 54-slot
// generic container. Callers must reject unsupported click modes before the
// inventory transaction and resync a stale view rather than consume items.
use horizon_items::items::ItemStack;
use super::tconstruct_source_smeltery_layout as layout;

pub(super) const MENU_SIZE: usize = 54;

pub(super) fn project(slots: &[Option<ItemStack>], offset: usize) -> Vec<Option<ItemStack>> {
    project_with_containers(slots, offset, &[None, None])
}

pub(super) fn project_with_containers(slots: &[Option<ItemStack>], offset: usize, containers: &[Option<ItemStack>; 2]) -> Vec<Option<ItemStack>> {
    let mut view = super::tconstruct_inventory_api::project_slots(slots,
        (0..MENU_SIZE).map(|menu_slot| layout::melting_index(slots.len(), offset, menu_slot)));
    view[..2].clone_from_slice(containers);
    view
}

/// Apply only a transaction based on the current page snapshot. The source
/// bucket slots are reserved for a separate transaction and must remain empty
/// here; all other padding is rejected rather than accepting a virtual item.
pub(super) fn commit(slots: &mut [Option<ItemStack>], offset: usize, expected: &[Option<ItemStack>], changed: &[Option<ItemStack>]) -> bool {
    commit_with_containers(slots, offset, expected, changed, &[None, None])
}

pub(super) fn commit_with_containers(slots: &mut [Option<ItemStack>], offset: usize, expected: &[Option<ItemStack>], changed: &[Option<ItemStack>], containers: &[Option<ItemStack>; 2]) -> bool {
    if expected.len() != MENU_SIZE || changed.len() != MENU_SIZE || project_with_containers(slots, offset, containers) != expected { return false; }
    for (menu_slot, item) in changed.iter().enumerate() {
        if menu_slot < 2 {
            if item != &containers[menu_slot] { return false; }
            continue;
        }
        if layout::melting_index(slots.len(), offset, menu_slot).is_none() && item.is_some() { return false; }
    }
    for (menu_slot, item) in changed.iter().enumerate() {
        if let Some(index) = layout::melting_index(slots.len(), offset, menu_slot) { slots[index] = item.clone(); }
    }
    true
}

/// Equality ignores retained wire bytes, including after a stack was edited.
/// Do not infer a lossless or current page from equality when those bytes exist.
pub(super) fn no_retained_wire(page: &[Option<ItemStack>]) -> bool {
    page.iter().flatten().all(|stack| !super::tconstruct_inventory_api::contains_retained_wire(stack))
}
pub(super) fn sent_page_matches(sent: &[Option<ItemStack>], expected: &[Option<ItemStack>]) -> bool {
    expected.len() == MENU_SIZE && sent.get(..MENU_SIZE).is_some_and(|page|
        no_retained_wire(page) && no_retained_wire(expected) && page == expected)
}

/// Stage a single insertion into an empty raw melting slot. Keep every other
/// field byte-for-byte and reject stacks that the item codec cannot preserve.
pub(super) fn prepare_single_insert(raw: &horizon_world::nbt::Tag, index: usize, held: &ItemStack)
    -> Option<(horizon_world::nbt::Tag, ItemStack, ItemStack)> {
    use horizon_world::nbt::Tag;
    if held.count <= 1 || super::tconstruct_inventory_api::contains_retained_wire(held) { return None; }
    let mut one = held.clone(); one.count = 1;
    let encoded = horizon_items::item_nbt::item_to_nbt(&one);
    let decoded = super::tconstruct_inventory_api::decode_item_canonical(&encoded)?;
    if decoded.count != 1 || !one.same_item_same_components(&decoded) { return None; }
    let Tag::Compound(mut fields) = raw.clone() else { return None; };
    let Some(Tag::List(slots)) = fields.get_mut(&"slots".into()) else { return None; };
    // The local path reprojects the page. An opaque saved item elsewhere
    // must not be turned into a lossy live representation by this click.
    for raw_slot in slots.iter() {
        if matches!(raw_slot, Tag::Compound(map) if map.is_empty()) { continue; }
        super::tconstruct_inventory_api::decode_item_canonical(raw_slot)?;
    }
    let slot = slots.get_mut(index)?;
    if !matches!(slot, Tag::Compound(map) if map.is_empty()) { return None; }
    *slot = encoded;
    let Some(Tag::IntArray(progress)) = fields.get_mut(&"progress".into()) else { return None; };
    *progress.get_mut(index)? = 0;
    let mut remainder = held.clone(); remainder.count -= 1;
    Some((Tag::Compound(fields), one, remainder))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projection_preserves_each_viewers_container_slots() {
        let mut melting = vec![None; 1];
        let buckets = [Some(ItemStack::new("minecraft:bucket", 1)), None];
        let expected = project_with_containers(&melting, 0, &buckets);
        let mut changed = expected.clone();
        changed[2] = Some(ItemStack::new("minecraft:copper_ore", 1));
        assert!(commit_with_containers(&mut melting, 0, &expected, &changed, &buckets));
        changed[0] = None;
        assert!(!commit_with_containers(&mut melting, 0, &expected, &changed, &buckets));
    }
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
