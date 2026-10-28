fn observe_client_stored_material(game: &crate::game::Game<'_>) {
    use horizon_world::nbt::Tag;
    let Some(saved) = game.world.chunks.get(&(0, 0)).and_then(|chunk| chunk.saved.as_deref()) else { return; };
    for tag in &saved.block_entities {
        if tag.get("id") == Some(&Tag::String("tconstruct:material_block".into()))
            && tag.get("x") == Some(&Tag::Int(0)) && tag.get("y") == Some(&Tag::Int(71)) && tag.get("z") == Some(&Tag::Int(2)) {
            let value = match tag.get("Material") { Some(Tag::String(material)) => material.to_string_lossy(), _ => "<none>".to_owned() };
            eprintln!("TCONSTRUCT_WIRE_STORED {}", value);
        }
    }
}
