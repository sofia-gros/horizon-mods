fn tconstruct_storage_pick_local(game: &mut Game<'_>, button: MouseButton, pressed: bool) -> std::ops::ControlFlow<Result<()>> {
    use horizon_world::nbt::Tag;
    use std::ops::ControlFlow::{Break, Continue};
    if button != MouseButton::Middle || !pressed || game.remote.is_some() || game.sleeping.is_some() || game.spectator() || game.pick_entity_hit().is_some() { return Continue(()); }
    let Some(hit) = game.pick() else { return Continue(()); };
    let pos = hit.pos;
    if &*game.res.facts().get(game.world.block(pos[0], pos[1], pos[2])).name != "tconstruct:fake_storage_block" { return Continue(()); }
    let Some(raw) = game.world.chunks.get(&(pos[0] >> 4, pos[2] >> 4)).and_then(|chunk| chunk.saved.as_deref()).and_then(|saved| saved.block_entities.iter().find(|tag| {
        [tag.get("x"), tag.get("y"), tag.get("z")] == [Some(&Tag::Int(pos[0])), Some(&Tag::Int(pos[1])), Some(&Tag::Int(pos[2]))]
    })) else { return Continue(()); };
    let Some(Tag::String(material)) = raw.get("Material") else { return Continue(()); };
    let material = material.to_string_lossy();
    let Some(items) = game.res.rest.get().map(|rest| &rest.items) else { return Continue(()); };
    if crate::game::tconstruct_source_builder_world::fake_storage_can_use(&game.world, items, &material) != Ok(true) { return Continue(()); }
    let Some(mut stack) = game.picked_stack(pos, false) else { return Continue(()); };
    stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("Material", Tag::String(material.as_str().into()))])));
    horizon_client::pick_item::pick(&mut game.inventory[..36], &mut game.selected, &stack, game.creative);
    Break(Ok(()))
}
