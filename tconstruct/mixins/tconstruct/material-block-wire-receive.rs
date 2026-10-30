fn material_block_wire_receive(
    game: &mut crate::game::Game<'_>, pos: [i32; 3], kind: i32, tag: &horizon_world::nbt::Tag,
) -> std::ops::ControlFlow<()> {
    use crate::game::tconstruct_tx::SourceBlockMaterial;
    use horizon_world::nbt::{Compound, Tag};
    use std::ops::ControlFlow::{Break, Continue};
    let name = game.remote.as_ref().and_then(|r| r.builtin.get("minecraft:block_entity_type")?.name(kind));
    if name != Some("tconstruct:material_block") { return Continue(()); }
    *game.world.chunks.__horizon_mixins_tconstruct_material_wire_client_mut() = true;
    let state = game.world.block(pos[0], pos[1], pos[2]);
    if &*game.res.facts().get(state).name != "tconstruct:fake_storage_block" { return Break(()); }
    let Some(chunk) = game.world.chunks.get_mut(&(pos[0] >> 4, pos[2] >> 4)) else { return Break(()); };
    let chunk = std::sync::Arc::make_mut(chunk);
    let saved = std::sync::Arc::make_mut(chunk.saved.get_or_insert_default());
    let at = |t: &Tag| [t.get("x"), t.get("y"), t.get("z")]
        == [Some(&Tag::Int(pos[0])), Some(&Tag::Int(pos[1])), Some(&Tag::Int(pos[2]))];
    let existing = saved.block_entities.iter().find(|t| at(t));
    let mut material = SourceBlockMaterial::UnknownSingleton;
    if let Some(old) = existing { material.load_tag(old); }
    material.load_tag(tag);
    let mut entity = Compound::new();
    entity.insert("id".into(), Tag::String("tconstruct:material_block".into()));
    for (key, value) in ["x", "y", "z"].into_iter().zip(pos) { entity.insert(key.into(), Tag::Int(value)); }
    material.write_tag(&mut entity);
    let entity = Tag::Compound(entity);
    let changed = existing != Some(&entity);
    match saved.block_entities.iter_mut().find(|t| at(t)) {
        Some(old) => *old = entity,
        None => saved.block_entities.push(entity),
    }
    if changed {
        let sy = (pos[1] - horizon_client::chunk::MIN_Y) >> 4;
        if (0..horizon_client::chunk::SECTIONS as i32).contains(&sy) {
            game.world.dirty_sections.insert((pos[0] >> 4, sy, pos[2] >> 4));
        }
    }
    Break(())
}
