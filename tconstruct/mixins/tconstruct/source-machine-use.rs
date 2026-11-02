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
        "tconstruct:copper_gauge" => crate::game::tconstruct_source_gauge::interact(game, pos),
        "tconstruct:crafting_station" => crate::game::tconstruct_crafting_station::open(game, hit.pos),
        "tconstruct:part_chest" | "tconstruct:tinkers_chest" | "tconstruct:tinkers_anvil" | "tconstruct:scorched_anvil" | "tconstruct:modifier_worktable" | "tconstruct:seared_melter" | "tconstruct:seared_heater" => { game.open_menu(hit.pos); true },
        "tconstruct:smeltery_controller" | "tconstruct:foundry_controller" => { game.open_menu(hit.pos); true },
        "tconstruct:seared_faucet" | "tconstruct:scorched_faucet" => crate::game::tconstruct_source_machine_runtime::activate_faucet(game, pos, &name),
        "tconstruct:seared_drain" | "tconstruct:scorched_drain" => crate::game::tconstruct_source_machine_runtime::interact_drain(game, pos),
        "tconstruct:seared_table" | "tconstruct:scorched_table" | "tconstruct:seared_basin" => crate::game::tconstruct_source_machine_runtime::interact_casting(game, pos, &name),
        "tconstruct:seared_fuel_tank" | "tconstruct:seared_fuel_gauge" | "tconstruct:scorched_fuel_tank" | "tconstruct:scorched_fuel_gauge" | "tconstruct:seared_ingot_tank" | "tconstruct:seared_ingot_gauge" | "tconstruct:scorched_ingot_tank" | "tconstruct:scorched_ingot_gauge" => crate::game::tconstruct_source_machine_runtime::interact_fuel_tank(game, pos, &name),
        _ => return Continue(()),
    };
    if handled { game.swing_arm(); }
    Break(Ok(handled))
}
