fn raw_fixture_save(value: R<i32>, game: &mut crate::game::Game<'_>) -> R<i32> {
    use horizon_world::nbt::Tag;
    let pos = (1, 70, 2);
    if &*game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name != "minecraft:decorated_pot" { return value; }
    let Some(chunk) = game.world.chunks.get_mut(&(0, 0)) else { return value; };
    let chunk = std::sync::Arc::make_mut(chunk);
    let saved = std::sync::Arc::make_mut(chunk.saved.get_or_insert_default());
    let position = saved.block_entities.iter().position(|tag| tag.get("id") == Some(&Tag::String("tconstruct:material_block".into())));
    eprintln!("TCONSTRUCT_RAW_ENTITY_BEFORE {}", position.is_some());
    if position.is_none() {
        saved.block_entities.push(Tag::compound([
            ("id", Tag::String("tconstruct:material_block".into())),
            ("x", Tag::Int(pos.0)), ("y", Tag::Int(pos.1)), ("z", Tag::Int(pos.2)),
            ("Material", Tag::String("tconstruct:bronze".into())),
        ]));
    }
    game.world.modified.insert((0, 0));
    value
}
