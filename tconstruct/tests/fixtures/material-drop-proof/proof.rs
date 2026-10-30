fn add_drop_proof(pending: &mut crate::game::spill::Pending, pos: [i32; 3], tag: &horizon_world::nbt::Tag) {
    use horizon_world::nbt::{Compound, Tag};
    if tag.get("id") != Some(&Tag::String("tconstruct:material_block".into())) { return; }
    let mut data = Compound::new();
    data.insert("proof".into(), Tag::Int(17));
    let mut patch = Compound::new();
    patch.insert("minecraft:custom_data".into(), Tag::Compound(data));
    let item = if tag.get("Material") == Some(&Tag::String("tconstruct:bronze".into())) {
        "minecraft:diamond"
    } else {
        "tconstruct:fake_storage_block"
    };
    pending.copied.insert(pos, (item.to_owned(), patch));
}
