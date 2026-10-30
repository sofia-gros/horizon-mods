fn observe_client_material(game: &crate::game::Game<'_>, pos: [i32; 3], kind: i32, tag: &horizon_world::nbt::Tag) {
    use horizon_world::nbt::Tag;
    let name = game.remote.as_ref().and_then(|r| r.builtin.get("minecraft:block_entity_type")?.name(kind));
    if name == Some("tconstruct:material_block") {
        if let Some(Tag::String(material)) = tag.get("Material") {
            eprintln!("TCONSTRUCT_WIRE_OBSERVED {} {} {} {}", pos[0],pos[1],pos[2],material.to_string_lossy());
        }
    }
}
