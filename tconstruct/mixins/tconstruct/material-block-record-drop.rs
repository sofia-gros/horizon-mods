fn material_block_record_drop(
    pending: &mut crate::game::spill::Pending,
    pos: [i32; 3],
    tag: &horizon_world::nbt::Tag,
) {
    use horizon_world::nbt::{Compound, Tag};
    if tag.get("id") != Some(&Tag::String("tconstruct:material_block".into())) { return; }
    // Pending is owned by the game; keep the opaque copy alongside, rather
    // than overwriting, any component patch another feature has recorded.
    if let Some(material) = tag.get("Material") {
        let entry = pending.copied.entry(pos).or_insert_with(||
            ("tconstruct:fake_storage_block".to_owned(), Compound::new()));
        entry.1.insert("tconstruct:internal_material_copy".into(), material.clone());
    } else if let Some((_, patch)) = pending.copied.get_mut(&pos) {
        patch.remove(&"tconstruct:internal_material_copy".into());
    }
}
