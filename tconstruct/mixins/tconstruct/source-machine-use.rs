fn tconstruct_machine_use(game: &mut crate::game::Game<'_>, hit: Option<&Hit>) -> std::ops::ControlFlow<Result<bool>> {
    use std::ops::ControlFlow::{Break, Continue};
    if game.remote.is_some() { return Continue(()); }
    let Some(hit) = hit else { return Continue(()); };
    // Match the existing use-item ordering: sneaking with a held item lets
    // that item act instead of opening the machine.
    if game.keys.contains(&KeyCode::ShiftLeft) && (game.stack_in(game.selected).is_some() || game.offhand.is_some()) { return Continue(()); }
    let state = game.world.block(hit.pos[0], hit.pos[1], hit.pos[2]);
    let name = game.res.facts().get(state).name.clone();
    let pos = (hit.pos[0], hit.pos[1], hit.pos[2]);
    if &*name == "tconstruct:part_chest" {
        if let Some(stack)=game.inventory.get(game.selected).cloned().flatten() {
            game.ensure_datapacks();
            let tagged=game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|r|r.as_ref().ok()).and_then(|t|t.get("tconstruct:chest_parts")).is_some_and(|v|v.contains(stack.item.as_ref()));
            let max=game.res.items().get(&stack.item).map_or(64,|info|info.max_stack).max(1);
            if let Some(horizon_client::containers::BlockEntity::Chest(items))=game.world.block_entities.get_mut(&pos) {
                let before=stack.count;
                let rest=crate::game::tconstruct_chest_rules::insert_across(crate::game::tconstruct_chest_rules::ChestKind::Part,items,stack,max,tagged);
                if rest.as_ref().map_or(0,|s|s.count)!=before {
                    game.inventory[game.selected]=rest;game.world.modified.insert((pos.0>>4,pos.2>>4));game.swing_arm();return Break(Ok(true));
                }
            }
        }
    }
    let handled = match &*name {
        "tconstruct:seared_casting_tank" => crate::game::tconstruct_source_machine_runtime::interact_casting_tank(game, pos, hit.point[1] - f64::from(pos.1) < 0.6875),
        "tconstruct:seared_fluid_cannon" => {
            let mut tank = hit.point[1] - f64::from(pos.1) > 0.5;
            if tank && hit.face == [0,1,0] && game.pack.assets.registry.state(state).and_then(|state| state.properties.get("facing")).is_some_and(|facing| facing == "up") {
                let x = hit.point[0] - f64::from(pos.0); let z = hit.point[2] - f64::from(pos.2);
                tank = x < 0.25 || x > 0.75 || z < 0.25 || z > 0.75;
            }
            crate::game::tconstruct_source_machine_runtime::interact_cannon(game,pos,tank)
        },
        "tconstruct:seared_melter" => {
            // Source TinyMultiblockControllerBlock tries the held container
            // first, then opens its normal menu when no transfer is possible.
            use crate::game::tconstruct_source_melter_store as store;
            if !crate::game::tconstruct_source_machine_runtime::ensure(game, pos, &name) { return Break(Ok(false)); }
            game.ensure_datapacks();
            let plan = game.held().and_then(|held| {
                let raw = store::read(&game.world, pos, "tconstruct:melter")?;
                let (fluids, _) = game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref()?.as_ref().ok()?;
                let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|value| value.as_ref().ok());
                crate::game::tconstruct_source_fluid_container::plan_melter_held(&raw, held,
                    &|item| game.res.items().get(item).is_some(), &|fluid| fluids.contains(fluid), tags,
                    game.lan.is_none() && game.lan_acting_uuid.is_none())
            });
            if let Some(mut plan) = plan {
                // A rejected save must not consume, give or drop any item.
                if !store::write(&mut game.world, pos, "tconstruct:melter", plan.saved) { return Break(Ok(false)); }
                if game.creative {
                    if plan.drained { if let Some(result) = plan.result.take() { game.give_or_drop(result); } }
                } else if let Some(rest) = plan.remaining.take() {
                    *game.held_mut() = Some(rest);
                    if let Some(result) = plan.result.take() { game.give_or_drop(result); }
                } else {
                    *game.held_mut() = plan.result.take();
                }
                game.container_changed(pos);
            } else {
                game.open_menu(hit.pos);
            }
            true
        },
        "tconstruct:copper_gauge" => crate::game::tconstruct_source_gauge::interact(game, pos),
        "tconstruct:crafting_station" => crate::game::tconstruct_crafting_station::open(game, hit.pos),
        "tconstruct:part_chest" | "tconstruct:tinkers_chest" | "tconstruct:tinkers_anvil" | "tconstruct:scorched_anvil" | "tconstruct:modifier_worktable" | "tconstruct:seared_heater" => { game.open_menu(hit.pos); true },
        "tconstruct:smeltery_controller" | "tconstruct:foundry_controller" => { game.open_menu(hit.pos); true },
        "tconstruct:seared_faucet" | "tconstruct:scorched_faucet" => crate::game::tconstruct_source_machine_runtime::activate_faucet(game, pos, &name),
        "tconstruct:seared_drain" | "tconstruct:scorched_drain" => {
            // A drain only consumes supported fluid transfers. If no transfer
            // happened, let the held item act (for example, place a faucet).
            if !crate::game::tconstruct_source_machine_runtime::interact_drain(game, pos) { return Continue(()); }
            true
        },
        "tconstruct:seared_table" | "tconstruct:scorched_table" | "tconstruct:seared_basin" | "tconstruct:scorched_basin" => crate::game::tconstruct_source_machine_runtime::interact_casting(game, pos, &name),
        "tconstruct:seared_fuel_tank" | "tconstruct:seared_fuel_gauge" | "tconstruct:scorched_fuel_tank" | "tconstruct:scorched_fuel_gauge" | "tconstruct:seared_ingot_tank" | "tconstruct:seared_ingot_gauge" | "tconstruct:scorched_ingot_tank" | "tconstruct:scorched_ingot_gauge" => crate::game::tconstruct_source_machine_runtime::interact_fuel_tank(game, pos, &name),
        _ => return Continue(()),
    };
    if handled { game.swing_arm(); }
    Break(Ok(handled))
}
