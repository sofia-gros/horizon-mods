fn source_material_block_place(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    state: horizon_world::registry::BlockStateId,
    stack: &horizon_client::items::ItemStack,
) {
    use horizon_world::nbt::{Compound, Tag};
    use crate::game::tconstruct_tx::SourceBlockMaterial;
    // Placement and its persistent data are authoritative on the server.
    if game.remote.is_some() { return; }
    if &*stack.item != "tconstruct:fake_storage_block"
        || &*game.res.facts().get(state).name != "tconstruct:fake_storage_block" { return; }
    // MaterialBlock.setPlacedBy only assigns a non-singleton item material.
    // Preserve the singleton versus parsed-unknown identity in the saved NBT.
    let tag = stack.data().and_then(|d| d.other("minecraft:custom_data"));
    let material = match tag.and_then(|t| t.get("Material")) {
        Some(Tag::String(s)) => Some(s.to_string_lossy()),
        _ => None,
    };
    let material = SourceBlockMaterial::placed_material(material.as_deref());
    let Some(chunk) = game.world.chunks.get_mut(&(pos[0] >> 4, pos[2] >> 4)) else { return; };
    let chunk = std::sync::Arc::make_mut(chunk);
    let saved = std::sync::Arc::make_mut(chunk.saved.get_or_insert_default());
    let at = |tag: &Tag| [tag.get("x"), tag.get("y"), tag.get("z")]
        == [Some(&Tag::Int(pos[0])), Some(&Tag::Int(pos[1])), Some(&Tag::Int(pos[2]))];
    let mut entity = Compound::new();
    entity.insert("id".into(), Tag::String("tconstruct:material_block".into()));
    for (key, value) in ["x", "y", "z"].into_iter().zip(pos) {
        entity.insert(key.into(), Tag::Int(value));
    }
    material.write_tag(&mut entity);
    let entity = Tag::Compound(entity);
    match saved.block_entities.iter_mut().find(|tag| at(tag)) {
        Some(old) => *old = entity,
        None => saved.block_entities.push(entity),
    }
    game.world.modified.insert((pos[0] >> 4, pos[2] >> 4));
}
