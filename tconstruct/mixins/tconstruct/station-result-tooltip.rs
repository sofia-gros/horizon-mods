fn tconstruct_result_tooltip(game: &mut crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    if crate::game::tconstruct_ui::is_melter(game) {
        let (ox, oy) = game.inventory_origin(batch);
        let position = if game.remote.is_some() { crate::game::remote::tconstruct_source_machine_remote::position(game) } else { game.container };
        let state = position.and_then(|pos| crate::game::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:melter"))
            .and_then(|tag| crate::game::tconstruct_source_melter_state::Melter::from_nbt(&tag));
        if let Some(state) = state {
            let has_heater = if game.remote.is_some() { game.remote_chest_menu().is_some_and(|(rows, _)| rows == 6) } else { game.container_size() > 27 };
            if (ox + 152.0..ox + 166.0).contains(&game.cursor.0) && (oy + 31.0..oy + 69.0).contains(&game.cursor.1)
                && (!has_heater || game.cursor.1 > oy + 50.0) {
                let queued_temperature = has_heater.then(|| game.container_kind().and_then(|entity| entity.slots().get(27)).and_then(Option::as_ref))
                    .flatten().filter(|stack| game.res.providers().burn_time(game.res.items(), stack, false) > 0)
                    .and_then(|_| crate::game::tconstruct_source_machine_runtime::solid_fuel_rule(game).map(|(temperature, _)| temperature));
                let temperature = if state.fuel > 0 { Some(state.temperature) } else { queued_temperature };
                let lines = if let Some(temperature) = temperature {
                    let title = game.pack.names.display_get("gui.tconstruct.melting.fuel.solid").unwrap_or_else(|| "Solid fuel".to_owned());
                    let format = game.pack.names.display_get("gui.tconstruct.melting.fuel.temperature").unwrap_or_else(|| "Temperature: %s°C".to_owned());
                    vec![(title, u32::MAX), (format.replace("%s", &temperature.to_string()), 0xffaaaaaa)]
                } else {
                    vec![(game.pack.names.display_get("gui.tconstruct.melting.fuel.empty").unwrap_or_else(|| "No fuel found".to_owned()), u32::MAX)]
                };
                game.draw_tooltip_lines(batch, &lines, game.cursor.0, game.cursor.1);
                return Break(());
            }
            if (ox + 89.0..ox + 143.0).contains(&game.cursor.0) && (oy + 15.0..oy + 69.0).contains(&game.cursor.1) {
                let mut lines = Vec::new();
                if let Some(fluid) = state.tank.fluid.as_ref() {
                    let key = format!("fluid.{}", fluid.replace(':', "."));
                    lines.push((game.pack.names.display_get(&key).unwrap_or_else(|| fluid.clone()), u32::MAX));
                    lines.push((format!("{} mB", state.tank.amount), 0xffaaaaaa));
                } else {
                    let label = game.pack.names.display_get("gui.tconstruct.melting.capacity").unwrap_or_else(|| "Capacity:".to_owned());
                    lines.push((format!("{label} {} mB", state.tank.capacity), u32::MAX));
                }
                game.draw_tooltip_lines(batch, &lines, game.cursor.0, game.cursor.1);
                return Break(());
            }
            for slot in 0..3 {
                let y = oy + 16.0 + slot as f32 * 18.0;
                if !(ox + 17.0..ox + 21.0).contains(&game.cursor.0) || !(y - 1.0..y + 17.0).contains(&game.cursor.1) { continue; }
                if let Some(stack) = state.slots[slot].as_ref() {
                    let (_, _, key) = crate::game::tconstruct_source_melter_render::heat_bar(&game.world, stack, state.temperature, state.progress[slot], false);
                    if let Some(key) = key { if let Some(text) = game.pack.names.display_get(key) {
                        game.draw_tooltip_lines(batch, &[(text, u32::MAX)], game.cursor.0, game.cursor.1);
                    }}
                }
                return Break(());
            }
        }
        return Continue(());
    }
    if crate::game::tconstruct_source_smeltery_ui::is_smeltery(game) {
        let (ox, oy) = game.inventory_origin(batch);
        if (ox + 124.0..ox + 142.0).contains(&game.cursor.0) && (oy + 69.0..oy + 87.0).contains(&game.cursor.1) {
            let mode = crate::game::tconstruct_source_smeltery_ui::position(game)
                .and_then(|pos| game.world.chunks.__horizon_mixins_tconstruct_smeltery_local_containers().get(&pos).map(|(_, mode)| *mode)).unwrap_or(0);
            let (name, title, description) = match mode {
                1 => ("empty_item", "Mode: Empty Item", "Slot will attempt to empty the item"),
                2 => ("fill_item", "Mode: Fill Item", "Slot will attempt to fill the item"),
                _ => ("auto", "Mode: Auto", "Slot will attempt to empty then fill the item"),
            };
            let translate = |suffix: &str, fallback: &str| game.pack.names
                .display_get(&format!("gui.tconstruct.tank.bucket.{name}.{suffix}")).unwrap_or_else(|| fallback.to_owned());
            let lines = vec![(translate("title", title), u32::MAX), (translate("description", description), 0xffaaaaaa),
                (game.pack.names.display_get("gui.tconstruct.tank.bucket.clickable").unwrap_or_else(|| "Click to toggle fluid transfer mode".to_owned()), 0xffaaaaaa)];
            game.draw_tooltip_lines(batch, &lines, game.cursor.0, game.cursor.1);
            return Break(());
        }
        let state = crate::game::tconstruct_source_smeltery_ui::position(game)
            .and_then(|pos| crate::game::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:smeltery"))
            .and_then(|tag| crate::game::tconstruct_source_smeltery_state::Smeltery::from_nbt(&tag));
        if let Some(state) = state {
            if (ox + 151.0..ox + 169.0).contains(&game.cursor.0) && (oy + 31.0..oy + 123.0).contains(&game.cursor.1) {
                let view = &state.fuel_view;
                let mut lines = Vec::new();
                if let Some(fluid) = view.fluid.as_deref() {
                    let key = format!("fluid.{}", fluid.replace(':', "."));
                    lines.push((game.pack.names.display_get(&key).unwrap_or_else(|| fluid.to_owned()), u32::MAX));
                    if view.temperature > 0 {
                        let format = game.pack.names.display_get("gui.tconstruct.melting.fuel.temperature").unwrap_or_else(|| "Temperature: %s°C".to_owned());
                        lines.push((format.replace("%s", &view.temperature.to_string()), 0xffaaaaaa));
                    } else {
                        lines.push((game.pack.names.display_get("gui.tconstruct.melting.fuel.invalid").unwrap_or_else(|| "Invalid melting fuel!".to_owned()), 0xff5555ff));
                    }
                    lines.push((format!("{} mB", view.amount), 0xffaaaaaa));
                } else {
                    let (key, fallback) = if view.has_tank { ("gui.tconstruct.melting.fuel.empty", "No fuel found") } else { ("gui.tconstruct.melting.fuel.no_tank", "No tank in structure") };
                    lines.push((game.pack.names.display_get(key).unwrap_or_else(|| fallback.to_owned()), u32::MAX));
                }
                game.draw_tooltip_lines(batch, &lines, game.cursor.0, game.cursor.1);
                return Break(());
            }
            let (count, offset) = crate::game::tconstruct_source_smeltery_ui::page(game);
            let cols = crate::game::tconstruct_source_smeltery_layout::columns(count);
            let rows = crate::game::tconstruct_source_smeltery_layout::visible_rows(count);
            for visible in 0..rows * cols {
                let menu_slot = visible + 2;
                let Some(index) = crate::game::tconstruct_source_smeltery_layout::melting_index(count, offset, menu_slot) else { continue; };
                let (Some(stack), Some(progress), Some((x, y))) = (state.slots.get(index).and_then(Option::as_ref), state.progress.get(index),
                    crate::game::tconstruct_source_smeltery_layout::position(count, offset, menu_slot)) else { continue; };
                if !(ox + x - 5.0..ox + x - 1.0).contains(&game.cursor.0) || !(oy + y - 1.0..oy + y + 17.0).contains(&game.cursor.1) { continue; }
                let foundry = crate::game::tconstruct_source_smeltery_ui::is_foundry(game);
                let (_, _, key) = crate::game::tconstruct_source_melter_render::heat_bar(&game.world, stack, state.temperature, *progress, foundry);
                if let Some(key) = key { if let Some(text) = game.pack.names.display_get(key) {
                    game.draw_tooltip_lines(batch, &[(text, u32::MAX)], game.cursor.0, game.cursor.1);
                }}
                return Break(());
            }
        }
        if (ox + 8.0..ox + 114.0).contains(&game.cursor.0) && (oy + 16.0..oy + 122.0).contains(&game.cursor.1) {
            let tank = crate::game::tconstruct_source_smeltery_ui::position(game)
                .and_then(|pos| crate::game::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:smeltery"))
                .and_then(|tag| tag.get("tank").and_then(crate::game::tconstruct_source_smeltery_tank::SmelteryTank::from_nbt));
            if let Some(tank) = tank {
                let amounts = tank.fluids.iter().map(|fluid| fluid.amount).collect::<Vec<_>>();
                let hovered = crate::game::tconstruct_source_smeltery_layout::fluid_at(&amounts, tank.capacity, game.cursor.1 - oy);
                let mut lines = Vec::new();
                if let Some(fluid) = hovered.and_then(|index| tank.fluids.get(index)) {
                    let key = format!("fluid.{}", fluid.id.replace(':', "."));
                    lines.push((game.pack.names.display_get(&key).unwrap_or_else(|| fluid.id.clone()), u32::MAX));
                    lines.push((format!("{} mB", fluid.amount), 0xffaaaaaa));
                } else {
                    for (key, fallback, amount) in [("capacity", "Capacity:", tank.capacity),
                        ("available", "Free:", tank.capacity.saturating_sub(tank.total())),
                        ("used", "Used:", tank.total())] {
                        let label = game.pack.names.display_get(&format!("gui.tconstruct.melting.{key}"))
                            .unwrap_or_else(|| fallback.to_owned());
                        lines.push((format!("{label} {amount} mB"), u32::MAX));
                    }
                }
                game.draw_tooltip_lines(batch, &lines, game.cursor.0, game.cursor.1);
                return Break(());
            }
        }
        return Continue(());
    }
    let Some(station) = crate::game::tconstruct_ui::kind(game) else { return Continue(()); };
    if let Some(start) = crate::game::tconstruct_source_workstation_tabs::start(game) {
        let (ox, oy) = game.inventory_origin(batch);
        let tabs = crate::game::tconstruct_source_workstation_tabs::from_game(game, start);
        if let Some(index) = crate::game::tconstruct_source_workstation_tabs::hit(tabs.len(), station, game.cursor.0 - ox, game.cursor.1 - oy) {
            let name = &tabs[index].1;
            let key = format!("block.{}", name.replace(':', "."));
            let title = game.pack.names.display_get(&key).unwrap_or_else(|| name.clone());
            let mut lines = vec![(title, u32::MAX)];
            if !crate::game::tconstruct_source_workstation_tabs::supported(name) {
                lines.push(("This table is not available yet".to_owned(), 0xffaaaaaa));
            }
            game.draw_tooltip_lines(batch, &lines, game.cursor.0, game.cursor.1);
            return Break(());
        }
    }
    if station {
        let (ox, oy) = game.inventory_origin(batch);
        if (ox + 182.0..ox + 182.0 + crate::game::tconstruct_builder_info_ui::panel_width(game, true)).contains(&game.cursor.0) {
            if let Some((top, bottom)) = crate::game::tconstruct_station_info_ui::lines(game) {
                let offsets = game.container.and_then(|pos| game.world.chunks.__horizon_mixins_tconstruct_station_info_scroll().get(&pos)).copied().unwrap_or_default();
                let selected = if (oy + 10.0..oy + 78.0).contains(&game.cursor.1) { Some((&top, offsets.0, oy + 10.0, crate::game::tconstruct_station_info_ui::TOP_LINE_HEIGHT, crate::game::tconstruct_station_info_ui::TOP_VISIBLE)) }
                    else if (oy + 99.0..oy + 166.5).contains(&game.cursor.1) { Some((&bottom, offsets.1, oy + 99.0, crate::game::tconstruct_station_info_ui::BOTTOM_LINE_HEIGHT, crate::game::tconstruct_station_info_ui::BOTTOM_VISIBLE)) }
                    else { None };
                if let Some((lines, scroll, start, line_height, visible)) = selected {
                    let scroll = scroll.min(lines.len().saturating_sub(visible));
                    let row = ((game.cursor.1 - start) / line_height) as usize;
                    let tip = if row == 0 && game.cursor.0 >= ox + 182.0 + crate::game::tconstruct_builder_info_ui::panel_width(game, true) - 12.0 && lines.iter().any(|(_, tip)| tip.is_some()) {
                        game.pack.names.display_get("gui.tconstruct.general.hover")
                    } else { lines.get(if row == 0 { 0 } else { scroll + row }).and_then(|(_, tip)| tip.clone()) };
                    if let Some(tip) = tip {
                        let lines = horizon_client::multiplayer::wrap(&tip, 200.0, &game.gui.atlas).into_iter().map(|line| (line, u32::MAX)).collect::<Vec<_>>();
                        game.draw_tooltip_lines(batch, &lines, game.cursor.0, game.cursor.1);
                    }
                    return Break(());
                }
            }
        }
    }
    if !station {
        let (ox, oy) = game.inventory_origin(batch);
        if (ox + 180.0..ox + 180.0 + crate::game::tconstruct_builder_info_ui::panel_width(game, false)).contains(&game.cursor.0) && (oy + 5.0..oy + 178.0).contains(&game.cursor.1) {
            let entries = crate::game::tconstruct_builder_info_ui::lines(game, &game.gui.atlas);
            let scroll = game.container.and_then(|pos| game.world.chunks.__horizon_mixins_tconstruct_builder_info_scroll().get(&pos)).copied().unwrap_or(0).min(entries.len().saturating_sub(crate::game::tconstruct_builder_info_ui::VISIBLE_LINES));
            let row = ((game.cursor.1 - oy - 5.0) / crate::game::tconstruct_builder_info_ui::LINE_HEIGHT) as usize;
            let tip = if row == 0 && game.cursor.0 >= ox + 180.0 + crate::game::tconstruct_builder_info_ui::panel_width(game, false) - 12.0 && entries.iter().any(|(_, _, tip)| tip.is_some()) {
                game.pack.names.display_get("gui.tconstruct.general.hover")
            } else { crate::game::tconstruct_builder_info_ui::visible_index(&entries, scroll, row).and_then(|index| entries.get(index)).and_then(|(_, _, tip)| tip.clone()) };
            if let Some(tip) = tip {
                let lines = horizon_client::multiplayer::wrap(&tip, 200.0, &game.gui.atlas).into_iter().map(|line| (line, u32::MAX)).collect::<Vec<_>>();
                game.draw_tooltip_lines(batch, &lines, game.cursor.0, game.cursor.1);
            }
            return Break(());
        }
        if let Some(key) = game.container {
            let patterns = game.world.chunks.__horizon_mixins_tconstruct_builder_choices().get(&key)
                .and_then(|(_, _, patterns)| patterns.as_ref());
            let start = game.world.chunks.__horizon_mixins_tconstruct_builder_scroll().get(&key).map_or(0, |state| state.1);
            let (ox, oy) = game.inventory_origin(batch);
            if let Some(pattern) = crate::game::tconstruct_source_builder_layout::button_at(patterns.map_or(0, Vec::len), start,
                (game.cursor.0 - ox) as i32, (game.cursor.1 - oy) as i32)
                .and_then(|index| patterns.and_then(|patterns| patterns.get(index))) {
                if let Some((namespace, path)) = pattern.split_once(':') {
                    let key = format!("pattern.{namespace}.{}", path.replace('/', "."));
                    let label = game.pack.names.display_get(&key).unwrap_or(key);
                    game.draw_tooltip_lines(batch, &[(label, u32::MAX)], game.cursor.0, game.cursor.1);
                    return Break(());
                }
            }
        }
    }
    if game.carried.is_some() { return Continue(()); }
    let Some(slot) = game.slot_at(batch, game.cursor.0, game.cursor.1) else { return Continue(()); };
    if !(if station { (324..=325).contains(&slot) } else { slot == 322 }) { return Continue(()); }
    let Some(BlockEntity::Chest(slots)) = game.container_kind() else { return Break(()); };
    let mut result = if station && game.remote.is_some() {
        slots.get(slot - 300).cloned().flatten()
    } else if station && slot == 324 {
        crate::game::tconstruct_source_station_world::building_plan(&game.world, slots).map(|(_, output)| output)
    } else if station {
        crate::game::tconstruct_source_station_world::operation_plan(&game.world, slots).map(|plan| plan.output)
    } else {
        if game.remote.is_some() { slots.get(22).cloned().flatten() }
        else { game.container.and_then(|key| game.world.chunks.__horizon_mixins_tconstruct_builder_preview().get(&key).and_then(|(_, _, _, result)| result.clone())) }
    };
    if station && slot == 324 && game.remote.is_none() {
        if let Some(output) = result.as_mut() {
            let name = game.container.and_then(|pos| game.world.chunks.__horizon_mixins_tconstruct_station_name_edit().get(&pos)).map(|edit| edit.value.as_str());
            crate::game::tconstruct_source_station_name::name_result(output, name);
        }
    }
    if let Some(result) = result { game.draw_stack_tooltip(batch, &result, game.cursor.0, game.cursor.1); }
    Break(())
}
