fn tconstruct_part_output(game: &mut crate::game::Game<'_>, slot: Option<usize>, button: i32, input: horizon_client::menu::Input) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(key) = game.container else { return Continue(()); };
    let Some(station) = crate::game::tconstruct_ui::kind(game) else { return Continue(()); };
    if input == horizon_client::menu::Input::QuickMove && !station && slot == Some(322) {
        if button == 0 || button == 1 {
            if game.remote.is_some() { crate::game::remote::tconstruct_source_quick_packet::send(game, button, 1); }
            else { crate::game::tconstruct_source_builder_take::quick_take(game, key); }
        }
        return Break(true);
    }
    if input == horizon_client::menu::Input::QuickMove && !station && game.remote.is_some() && slot.is_some_and(|slot| slot < 36) { return Continue(()); }
    if input != horizon_client::menu::Input::Pickup { return Break(true); }
    if !station && slot.is_some_and(|s| (324..=326).contains(&s)) {
        if button == 0 {
            let start = game.world.chunks.__horizon_mixins_tconstruct_builder_scroll().get(&key).map_or(0, |state| state.1);
            let index = start + slot.unwrap() - 324;
            if let Some((window, _)) = game.remote.as_ref().and_then(|r| r.menu.as_ref()) {
                game.send(horizon_protocol::packets::play::Serverbound::ContainerButtonClick(horizon_protocol::packets::play::ContainerButtonClick { container: *window, button: index as i32 }));
            } else {
                let pattern = game.world.chunks.__horizon_mixins_tconstruct_builder_choices().get(&key)
                    .and_then(|(_, _, patterns)| patterns.as_ref()).and_then(|patterns| patterns.get(index)).cloned();
                if let Some(pattern) = pattern {
                    crate::game::tconstruct_source_builder_world::select(&mut game.world, key, pattern);
                }
            }
        }
        return Break(true);
    }
    if game.remote.is_some() {
        if station && slot == Some(324) {
            let output = game.container_kind().and_then(|entity| entity.slots().get(24)).and_then(Option::as_ref);
            if output.is_some_and(|stack| !crate::game::tconstruct_ui::station_accepts_output(game, stack)) { return Break(true); }
        }
        if !station && slot == Some(322) && button == 1 {
            crate::game::remote::tconstruct_source_quick_packet::send(game, button, 0);
            return Break(true);
        }
        return Continue(());
    }
    let repair_slot = station && slot == Some(303) && crate::game::tconstruct_ui::station_layout(game).is_some_and(|layout| layout.tool_slot.is_some());
    if (302..=323).contains(&slot.unwrap_or(0)) && (!station && slot != Some(322) || station && slot != Some(302) && !repair_slot) { return Break(true); }
    if !station && slot == Some(322) {
        if button == 0 || button == 1 { crate::game::tconstruct_source_builder_take::take(game, key); }
        return Break(true);
    }
    let action = if station { slot.filter(|s| (324..=325).contains(s)).map(|s| (s - 300) as i16) } else { None };
    let Some(action) = action else { return Continue(()); };
    if button != 0 { return Break(true); }
    let mut build = if action == 24 {
        game.world.block_entities.get(&key).and_then(|entity| crate::game::tconstruct_source_station_world::building_plan(&game.world, entity.slots()))
    } else { None };
    if let Some((_, output)) = build.as_mut() {
        let name = game.world.chunks.__horizon_mixins_tconstruct_station_name_edit().get(&key).map(|edit| edit.value.as_str());
        crate::game::tconstruct_source_station_name::name_result(output, name);
    }
    let operation = if action == 25 {
        game.world.block_entities.get(&key).and_then(|entity| crate::game::tconstruct_source_station_world::operation_plan(&game.world, entity.slots()))
    } else { None };
    if action == 25 {
        if let Some(plan) = operation { crate::game::tconstruct_source_station_take_host::take(game, key, plan); }
        return Break(true);
    }
    if build.as_ref().is_some_and(|(_, output)| !crate::game::tconstruct_ui::station_accepts_output(game, output)) { return Break(true); }
    let Some(BlockEntity::Chest(stacks)) = game.world.block_entities.get_mut(&key) else { return Break(true); };
    let taken = if action == 24 {
        build.is_some_and(|(matched, output)| crate::game::tconstruct_source_station_recipes::take_built_tool(stacks, &mut game.carried, &matched, output, 1))
    } else { false };
    if taken {
        game.world.modified.insert((key.0 >> 4, key.2 >> 4));
        game.world.chunks.__horizon_mixins_tconstruct_station_name_edit_mut().remove(&key);
    }
    Break(true)
}
