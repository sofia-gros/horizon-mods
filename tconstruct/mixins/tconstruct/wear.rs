fn tconstruct_wear(game: &mut crate::game::Game<'_>, hardness: f32) -> std::ops::ControlFlow<()> {
    let Some(mut stack) = game.inventory[game.selected].clone() else { return std::ops::ControlFlow::Continue(()); };
    if stack.item.as_ref() != "tconstruct:flint_pickaxe" && !(stack.item.starts_with("tconstruct:") && matches!(stack.data().and_then(|data| data.other("minecraft:custom_data")).and_then(|tag| tag.get("tic_max_damage")), Some(horizon_world::nbt::Tag::Int(max)) if *max > 0)) { return std::ops::ControlFlow::Continue(()); }
    // The host calls this hook only after a successful non-creative removal.
    // Spend before durability wear; the area marker prevents extra-block spends.
    let actor = game.lan_acting;
    let active_area = game.world.chunks.__horizon_mixins_tconstruct_area_active().contains(&actor);
    let already_spent = active_area && game.world.chunks.__horizon_mixins_tconstruct_area_overwield_spent().contains(&actor);
    let tags = &game.items().item_tags;
    let broad = tags.get("tconstruct:modifiable/broad").is_some_and(|items| items.contains(stack.item.as_ref()));
    let harvest = tags.get("tconstruct:modifiable/harvest").is_some_and(|items| items.contains(stack.item.as_ref()));
    let intact = matches!(stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|d| d.get("tic_max_damage")), Some(horizon_world::nbt::Tag::Int(max)) if *max > 0 && stack.damage < *max as u32);
    let cost = if !already_spent && intact && (broad || harvest) {
        stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|d| d.get("tic_mining_traits"))
            .and_then(|tag| if let horizon_world::nbt::Tag::List(values) = tag { Some(values) } else { None })
            .and_then(|values| values.iter().find_map(|v| match (v.get("id"), v.get("level")) {
                (Some(horizon_world::nbt::Tag::String(id)), Some(horizon_world::nbt::Tag::Int(level))) if id.to_string_lossy() == "tconstruct:overwield" => Some((*level).max(0) as u32),
                _ => None,
            })).unwrap_or(0).saturating_mul(if broad { 5 } else { 1 })
    } else { 0 };
    if cost > 0 {
        if let Some(held) = game.inventory[game.selected].as_mut() {
            crate::game::tconstruct_source_overslime::spend(held, cost);
            stack = held.clone();
        }
        if active_area { game.world.chunks.__horizon_mixins_tconstruct_area_overwield_spent_mut().insert(actor); }
    }
    if hardness != 0.0 {
        let (loss, overslime) = crate::game::tconstruct_source_overslime_host::damage(game, &stack, 1);
        if let Some(held) = game.inventory[game.selected].as_mut() {
            crate::game::tconstruct_source_overslime::spend(held, overslime);
            crate::game::tconstruct_tx::fixed_damage(held, loss);
        }
    }
    game.stats.add("used", &stack.item, 1);
    std::ops::ControlFlow::Break(())
}
