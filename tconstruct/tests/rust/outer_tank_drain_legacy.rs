pub(super) fn interact_drain(game: &mut super::Game<'_>, pos: Pos) -> bool {
    game.ensure_datapacks();
    let Some(controller) = game.world.chunks.__horizon_mixins_tconstruct_smeltery_owners().get(&pos).copied() else { return false; };
    let block = game.res.facts().get(game.world.block(controller.0, controller.1, controller.2)).name.clone();
    if !matches!(&*block, "tconstruct:smeltery_controller" | "tconstruct:foundry_controller") { return false; }
    let Some((min, max, _)) = super::tconstruct_source_smeltery_world::detect(game, controller) else { return false; };
    let (x, y, z) = pos;
    let wall = y > min[1] && y <= max[1] && ((x == min[0] || x == max[0]) && z > min[2] && z < max[2]
        || (z == min[2] || z == max[2]) && x > min[0] && x < max[0]);
    let floor = y == min[1] && x > min[0] && x < max[0] && z > min[2] && z < max[2];
    let foundry_shell = &*block == "tconstruct:foundry_controller" && x >= min[0] && x <= max[0]
        && z >= min[2] && z <= max[2] && y >= min[1] && y <= max[1]
        && (y == min[1] || x == min[0] || x == max[0] || z == min[2] || z == max[2]);
    if !wall && !floor && !foundry_shell { return false; }
    let Some(raw_controller) = store::read(&game.world, controller, "tconstruct:smeltery") else { return false; };
    let Some(mut state) = Smeltery::from_nbt(&raw_controller) else { return false; };
    // Keep tank/row extensions by exact fluid identity, without serializing
    // unrelated inventory, heat, fuel or controller extension data.
    let Some(raw_tank) = raw_controller.get("tank") else { return false; };
    if state.tank.to_nbt_preserving(raw_tank).as_ref() != Some(raw_tank) { return false; }
    let Some(held) = game.held().cloned() else { return false; };
    if held.wire_patch().is_some() || held.data().is_some_and(|data| data.wire.patch().is_some()) { return false; };

    let fluids = game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref()
        .and_then(|value| value.as_ref().ok()).map(|(fluids, _)| fluids.clone()).unwrap_or_default();
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|value| value.as_ref().ok())
        .map(|all| all.iter().filter(|(id, _)| matches!(id.as_str(), "forge:bottles/splash" | "forge:bottles/lingering"))
            .map(|(id, items)| (id.clone(), items.clone())).collect());
    // Mantle returns CONTAINER (handled) even when no fluid can move.
    // Limit recognition to containers supported by our existing transfer path.
    let container = super::tconstruct_source_fluid_container::accepts_with_color(&held, &|fluid| fluids.contains(fluid), true);
    let before = state.tank.total();
    let mut slots = [Some(held), None];
    if !super::tconstruct_source_fluid_container::transfer_selected_with_color(&mut state.tank, &mut slots,
        super::tconstruct_source_fluid_container::Direction::Auto, None,
        &|item| game.res.items().get(item).is_some(), &|fluid| fluids.contains(fluid), tags.as_ref(), game.lan.is_none() && game.lan_acting_uuid.is_none()) { return container; }
    let Some(tank) = state.tank.to_nbt_preserving(raw_tank) else { return false; };
    let horizon_world::nbt::Tag::Compound(mut fields) = raw_controller.clone() else { return false; };
    fields.insert("tank".into(), tank);
    let saved = horizon_world::nbt::Tag::Compound(fields);
    if store::read(&game.world, controller, "tconstruct:smeltery").as_ref() != Some(&raw_controller) { return false; }
    if !store::write(&mut game.world, controller, "tconstruct:smeltery", saved) { return false; }
    if game.creative {
        if state.tank.total() < before {
            if let Some(result) = slots[1].take() { game.give_or_drop(result); }
        }
    } else if let Some(rest) = slots[0].take() {
        *game.held_mut() = Some(rest);
        if let Some(result) = slots[1].take() { game.give_or_drop(result); }
    } else { *game.held_mut() = slots[1].take(); }
    true
}
