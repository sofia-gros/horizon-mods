// Lossless native item storage adapters, independent of any machine schema.
use horizon_items::{
    item_nbt::{item_from_nbt, item_to_nbt},
    items::ItemStack,
};
use horizon_world::nbt::Tag;

// Every ItemStack-bearing field in official StackData. Keep this traversal in
// one place: native component equality intentionally ignores nested Wire too.
pub(crate) fn nested_stacks(item: &ItemStack) -> impl Iterator<Item = &ItemStack> {
    item.data().into_iter().flat_map(|data| {
        data.container.iter().map(|(_, stack)| stack)
            .chain(data.charged.iter())
            .chain(data.pot_decorations.iter().flat_map(|slots| slots.iter().flatten()))
            .chain(data.bundle_contents.iter())
    })
}

/// Whether this stack or any nested stack retains opaque wire backing.
/// Includes stale backing for which `wire_patch()` is already unavailable.
/// This is a read-only query, not a test of snapshot equality or transport.
pub fn contains_retained_wire(item: &ItemStack) -> bool {
    let mut pending = vec![item];
    while let Some(item) = pending.pop() {
        if item.data().is_some_and(|data| data.wire.patch().is_some()) {
            return true;
        }
        pending.extend(nested_stacks(item));
    }
    false
}

/// Stage the native NBT representation of a nonempty, representable item.
///
/// Retained wire backing anywhere in the stack tree is opaque to the saved-item
/// codec, including after `wire_patch()` becomes stale. Refuse it before
/// serialization. Count, damage
/// and complete native component identity must survive decoding the proposal.
/// Neither the source stack nor any live storage is changed.
///
/// This does not recover fields discarded by an earlier raw-storage read,
/// authorize replacement of an opaque saved item, or guarantee wire transport.
/// Callers must validate the original raw record and recheck live snapshots
/// before committing a replacement.
pub fn encode_item_lossless(item: &ItemStack) -> Option<Tag> {
    if item.count == 0 || contains_retained_wire(item) {
        return None;
    }
    super::pluto_inventory::encode_lossless(item, item_to_nbt, item_from_nbt, |before, after| {
        before.count == after.count
            && before.damage == after.damage
            && before.same_item_same_components(after)
    })
}

/// Decode only a saved item whose native re-encoding equals the supplied Tag.
///
/// This uses the host's existing item codec and Tag equality, including Tag
/// types, UTF-16 keys and floating-point bit patterns. It is not a comparison
/// of serialized bytes and does not preserve fields the codec discards.
/// Empty-slot representations, aliases and additional count/component policy
/// belong to callers. It neither validates a live snapshot nor proves opaque
/// wire preservation, transport safety or authority to replace storage.
pub fn decode_item_canonical(raw: &Tag) -> Option<ItemStack> {
    let decoded = item_from_nbt(raw)?;
    (item_to_nbt(&decoded) == *raw).then_some(decoded)
}

/// Compare observable native item state and count, including exposed wire facts.
///
/// Horizon's normal component comparison excludes opaque wire backing. Compare
/// both its retained bytes and currently usable patch at every nested stack so
/// a callback cannot add, remove, or stale that backing unnoticed. Also compare
/// runtime-only bundle selection, which native component equality omits but a
/// replacement from an older snapshot could overwrite.
///
/// The host keeps Wire's decoded baseline private. Two different baselines can
/// expose identical retained bytes and both currently be stale; this comparison
/// cannot distinguish them. It is not arbitrary live write authority or proof
/// of complete private Wire identity. Persistence rejects all retained backing.
pub fn same_item_snapshot(before: &ItemStack, after: &ItemStack) -> bool {
    if before.count != after.count || before.damage != after.damage
        || !before.same_item_same_components(after) {
        return false;
    }
    // Native equality above already checks the full nested structure, slot
    // positions and values. Check its omitted wire identity without repeating
    // whole-subtree component equality at each level.
    let mut pending = vec![(before, after)];
    while let Some((before, after)) = pending.pop() {
        let same_omitted_state = before.data().and_then(|data| data.wire.patch())
            == after.data().and_then(|data| data.wire.patch())
            && before.wire_patch() == after.wire_patch()
            && before.data().map(|data| data.bundle_selected.0)
                == after.data().map(|data| data.bundle_selected.0);
        if !same_omitted_state {
            return false;
        }
        pending.extend(nested_stacks(before).zip(nested_stacks(after)));
    }
    true
}

/// Recheck every inventory slot using observable native state; refuse missing storage.
/// Empty slots, slot order, length, counts, damage, components and wire backing
/// must all agree. This does not lock or mutate the inventory.
pub fn inventory_snapshot_matches(
    before: &[Option<ItemStack>],
    current: Option<&[Option<ItemStack>]>,
) -> bool {
    let Some(current) = current else {
        return false;
    };
    before.len() == current.len()
        && before
            .iter()
            .zip(current)
            .all(|(before, after)| match (before, after) {
                (None, None) => true,
                (Some(before), Some(after)) => same_item_snapshot(before, after),
                _ => false,
            })
}
