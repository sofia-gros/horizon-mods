fn tconstruct_bottle_cauldron(game: &mut crate::game::Game<'_>, pos: [i32; 3]) -> std::ops::ControlFlow<Result<bool>> {
    use std::ops::ControlFlow::{Break, Continue};
    if game.remote.is_some() || game.keys.contains(&KeyCode::ShiftLeft) { return Continue(()); }
    let Some(stack) = game.held().cloned() else { return Continue(()); };
    let Some((state, _)) = game.cauldron_at(pos) else { return Continue(()); };
    let level = if state.name == "minecraft:water_cauldron" {
        match state.properties.get("level").and_then(|value| value.parse::<u8>().ok()) { Some(value @ 1..=3) => value, _ => return Continue(()) }
    } else if state.name == "minecraft:cauldron" { 0 } else { return Continue(()); };
    let (output, next, filling) = match stack.item.as_ref() {
        "tconstruct:splash_bottle" if level > 0 => ("minecraft:splash_potion", level - 1, true),
        "tconstruct:lingering_bottle" if level > 0 => ("minecraft:lingering_potion", level - 1, true),
        "minecraft:splash_potion" if level < 3 && stack.potion() == Some("minecraft:water") => ("tconstruct:splash_bottle", level + 1, false),
        "minecraft:lingering_potion" if level < 3 && stack.potion() == Some("minecraft:water") => ("tconstruct:lingering_bottle", level + 1, false),
        _ => return Continue(()),
    };
    if game.res.items().get(output).is_none() { return Continue(()); }
    let mut next_state = state.clone();
    if next == 0 { next_state.name = "minecraft:cauldron".into(); next_state.properties.clear(); }
    else { next_state.name = "minecraft:water_cauldron".into(); next_state.properties.insert("level".into(), next.to_string().into()); }
    let Ok(new_state) = game.pack.assets.registry.resolve(&next_state) else { return Continue(()); };
    let filled = if filling { ItemStack::new(output, 1).with_potion("minecraft:water") } else { ItemStack::new(output, 1) };
    crate::game::potions::tconstruct_source_bottle_fill::fill(game, filled);
    game.award_custom_stat("use_cauldron", 1);
    game.award_stat("used", stack.item.as_ref(), 1);
    if let Err(error) = game.level_set(pos, new_state) { return Break(Err(error)); }
    let sound = if filling { "minecraft:item.bottle.fill" } else { "minecraft:item.bottle.empty" };
    game.audio.play(sound, Some(pos.map(|value| f64::from(value) + 0.5)), 1.0, 1.0);
    game.post_block_game_event(if filling { "minecraft:fluid_pickup" } else { "minecraft:fluid_place" }, pos, None, None);
    Break(Ok(true))
}
