fn raw_fixture_mesh(world: &World, x: i32, z: i32) {
    use horizon_world::nbt::Tag;
    if let Some(saved) = world.chunks.get(&(x, z)).and_then(|chunk| chunk.saved.as_deref()) {
        if saved.block_entities.iter().any(|tag| tag.get("id") == Some(&Tag::String("tconstruct:material_block".into())) && tag.get("Material") == Some(&Tag::String("tconstruct:bronze".into()))) {
            eprintln!("TCONSTRUCT_RAW_ENTITY_MESH {x} {z}");
        }
    }
}
