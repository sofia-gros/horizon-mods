fn material_block_attach_drop(
    pending: &mut crate::game::spill::Pending,
    pos: [i32; 3],
    stack: &mut horizon_client::items::ItemStack,
) {
    use horizon_client::item_data;
    use horizon_world::nbt::{Compound, Tag};
    let marker = "tconstruct:internal_material_copy";
    let fake = "tconstruct:fake_storage_block";
    let Some((item, patch)) = pending.copied.get(&pos) else { return; };
    if !patch.contains_key(&marker.into()) { return; }
    let matches_entry = item == &*stack.item;
    if &*stack.item != fake && !matches_entry { return; }
    let Some((item, mut patch)) = pending.copied.remove(&pos) else { return; };
    let Some(material) = patch.remove(&marker.into()) else { return; };
    if matches_entry {
        // Perform the host's original component patch before copying Material.
        let mut components = match item_data::encode_components(stack) {
            Some(Tag::Compound(existing)) => existing,
            _ => Compound::new(),
        };
        components.extend(patch);
        (stack.damage, stack.extra) = item_data::decode_components(Some(&Tag::Compound(components)));
    } else {
        pending.copied.insert(pos, (item, patch));
    }
    if &*stack.item == fake {
        let mut data = match stack.data().and_then(|d| d.other("minecraft:custom_data")) {
            Some(Tag::Compound(existing)) => existing.clone(),
            _ => Compound::new(),
        };
        data.insert("Material".into(), material);
        stack.data_mut().set_other("minecraft:custom_data", Some(Tag::Compound(data)));
    } else {
        // Another item at this position took its patch first. Keep Material
        // pending for the fake block item; the normal clear hook bounds it.
        let mut retained = Compound::new();
        retained.insert(marker.into(), material);
        pending.copied.insert(pos, (fake.to_owned(), retained));
    }
}
