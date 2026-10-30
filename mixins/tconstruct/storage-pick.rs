fn tconstruct_storage_pick(game: &Game<'_>, pos: [i32; 3], include_data: bool) -> std::ops::ControlFlow<Option<ItemStack>> {
    use std::ops::ControlFlow::{Break, Continue};
    let state = game.world.block(pos[0], pos[1], pos[2]);
    let facts = game.res.facts().get(state);
    if &*facts.name != "tconstruct:fake_storage_block" { return Continue(()); }
    let Some(raw) = game.world.chunks.get(&(pos[0] >> 4, pos[2] >> 4)).and_then(|chunk| chunk.saved.as_deref()).and_then(|saved| saved.block_entities.iter().find(|tag| {
        [tag.get("x"), tag.get("y"), tag.get("z")] == [Some(&Tag::Int(pos[0])), Some(&Tag::Int(pos[1])), Some(&Tag::Int(pos[2]))]
    })) else { return Continue(()); };
    let Some(Tag::String(material)) = raw.get("Material") else { return Continue(()); };
    let material = material.to_string_lossy();
    let Some(items) = game.res.rest.get().map(|rest| &rest.items) else { return Continue(()); };
    if crate::game::tconstruct_source_builder_world::fake_storage_can_use(&game.world, items, &material) != Ok(true) { return Continue(()); }
    let mut stack = ItemStack::new(&facts.item, 1);
    if include_data {
        if let Some(saved) = game.saved_block_entity((pos[0], pos[1], pos[2])) {
            let mut all = components(&stack);
            all.extend(block_entity_item::pick(&saved));
            (stack.damage, stack.extra) = item_data::decode_components(Some(&Tag::Compound(all)));
        }
    }
    let mut data = match stack.data().and_then(|data| data.other("minecraft:custom_data")) {
        Some(Tag::Compound(data)) => data.clone(),
        _ => Compound::new(),
    };
    data.insert("Material".into(), Tag::String(material.as_str().into()));
    stack.data_mut().set_other("minecraft:custom_data", Some(Tag::Compound(data)));
    Break(Some(stack))
}
