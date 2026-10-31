fn tconstruct_pointer(game: &mut crate::game::Game<'_>, right: bool) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(station) = crate::game::tconstruct_ui::kind(game) else { return Continue(()); };
    let Some(key) = game.container else { return Break(true); };
    let Some(size) = game.renderer.as_ref().map(|r| [r.config.width, r.config.height]) else { return Break(true); };
    let batch = horizon_client::hud::HudBatch::new(size);
    if !right {
        let (ox, oy) = game.inventory_origin(&batch);
        let start = crate::game::tconstruct_source_workstation_tabs::start(game).unwrap_or(key);
        let tabs = crate::game::tconstruct_source_workstation_tabs::from_game(game, start);
        if let Some(index) = crate::game::tconstruct_source_workstation_tabs::hit(tabs.len(), station, game.cursor.0 - ox, game.cursor.1 - oy) {
            let (pos, name) = &tabs[index];
            if *pos != start && crate::game::tconstruct_source_workstation_tabs::supported(name) {
                if let Some((window, _)) = game.remote.as_ref().and_then(|remote| remote.menu.as_ref()) {
                    game.send(horizon_protocol::packets::play::Serverbound::ContainerButtonClick(horizon_protocol::packets::play::ContainerButtonClick { container: *window, button: 31000 + index as i32 }));
                } else {
                    let carried = game.carried.take();
                    game.close_container();
                    game.carried = carried;
                    game.open_station([pos.0, pos.1, pos.2], name);
                }
            }
            return Break(true);
        }
    }
    if station && !right {
        let (ox, oy) = game.inventory_origin(&batch);
        let (x, y) = (game.cursor.0 - ox, game.cursor.1 - oy);
        if crate::game::tconstruct_name_ui::available(game) && (80.0..162.0).contains(&x) && (7.0..16.0).contains(&y) {
            let shift = game.has_shift_down();
            let edit = game.world.chunks.__horizon_mixins_tconstruct_station_name_edit_mut().entry(key)
                .or_insert_with(|| crate::game::tconstruct_name_ui::new_box(ox, oy));
            edit.x = ox + 80.0; edit.y = oy + 7.0;
            edit.click(&game.gui.atlas, game.cursor.0, shift, false);
            edit.focused = true;
            return Break(true);
        }
        if let Some(edit) = game.world.chunks.__horizon_mixins_tconstruct_station_name_edit_mut().get_mut(&key) { edit.focused = false; }
        if let Some(index) = crate::game::tconstruct_source_station_layout::button_at(
            crate::game::tconstruct_ui::station_layouts(game).len(), x.floor() as i32, y.floor() as i32,
        ) {
            if let Some(layout) = crate::game::tconstruct_ui::station_layouts(game).get(index) {
                game.world.chunks.__horizon_mixins_tconstruct_part_selections_mut().insert(key, layout.id.clone());
                return Break(true);
            }
        }
    }
    if station {
        if let Some(slot @ 500..=504) = game.slot_at(&batch, game.cursor.0, game.cursor.1) {
            let shift = game.has_shift_down();
            if let Some((window, _)) = game.remote.as_ref().and_then(|remote| remote.menu.as_ref()) {
                let action = (slot - 500) as i32 * 3 + if shift { 2 } else if right { 1 } else { 0 };
                game.send(horizon_protocol::packets::play::Serverbound::ContainerButtonClick(horizon_protocol::packets::play::ContainerButtonClick { container: *window, button: 31100 + action }));
            } else {
                // Use the existing InventoryMenu transaction and equipment rules.
                let previous = game.container.take();
                let previous_size = game.craft_size;
                game.craft_size = 2;
                game.menu_ops(&[(Some(slot), i32::from(right), if shift { horizon_client::menu::Input::QuickMove } else { horizon_client::menu::Input::Pickup })]);
                game.craft_size = previous_size;
                game.container = previous;
            }
            return Break(true);
        }
    }
    if game.has_shift_down() {
        if !station && game.slot_at(&batch, game.cursor.0, game.cursor.1) == Some(322) {
            if game.remote.is_some() { crate::game::remote::tconstruct_source_quick_packet::send(game, i32::from(right), 1); }
            else { crate::game::tconstruct_source_builder_take::quick_take(game, key); }
        }
        if !station && game.remote.is_some() && game.slot_at(&batch, game.cursor.0, game.cursor.1).is_some_and(|slot| slot < 36) { return Continue(()); }
        return Break(true);
    }
    if !station {
        let (ox, oy) = game.inventory_origin(&batch);
        let (x, y) = ((game.cursor.0 - ox) as i32, (game.cursor.1 - oy) as i32);
        let side_count = crate::game::tconstruct_ui::side_count(game);
        let side = crate::game::tconstruct_source_side_layout::left(side_count);
        let slider_x = side + 7.0 + crate::game::tconstruct_source_side_layout::columns(side_count) as f32 * 18.0;
        let slider_h = crate::game::tconstruct_source_side_layout::visible_rows(side_count) as f32 * 18.0;
        if !right && crate::game::tconstruct_source_side_layout::max_offset(side_count) > 0
            && (slider_x..slider_x + 10.0).contains(&(x as f32)) && (17.0..17.0 + slider_h).contains(&(y as f32)) {
            let state = game.world.chunks.__horizon_mixins_tconstruct_builder_side_scroll_mut().entry(key).or_default();
            state.0 = crate::game::tconstruct_source_side_layout::drag_offset(side_count, y as f32);
            state.1 = true;
            return Break(true);
        }
        let patterns = game.world.chunks.__horizon_mixins_tconstruct_builder_choices().get(&key)
            .and_then(|(_, _, patterns)| patterns.as_ref());
        let count = patterns.map_or(0, Vec::len);
        let start = game.world.chunks.__horizon_mixins_tconstruct_builder_scroll().get(&key).map_or(0, |state| state.1);
        if !right && count > 16 && (126..138).contains(&x) && (15..87).contains(&y) {
            game.world.chunks.__horizon_mixins_tconstruct_builder_scroll_mut().entry(key).or_insert_with(|| (0.0, 0, false, Vec::new())).2 = true;
            return Break(true);
        }
        if let Some(index) = crate::game::tconstruct_source_builder_layout::button_at(count, start, x, y) {
            if !right {
                let pattern = patterns.and_then(|patterns| patterns.get(index)).cloned();
                if let Some((window, _)) = game.remote.as_ref().and_then(|r| r.menu.as_ref()) {
                    game.send(horizon_protocol::packets::play::Serverbound::ContainerButtonClick(horizon_protocol::packets::play::ContainerButtonClick { container: *window, button: index as i32 }));
                } else if let Some(pattern) = pattern {
                    crate::game::tconstruct_source_builder_world::select(&mut game.world, key, pattern);
                }
            }
            return Break(true);
        }
    }
    let Some(slot) = game.slot_at(&batch, game.cursor.0, game.cursor.1) else { return Continue(()); };
    if !station && (324..=326).contains(&slot) { return Break(true); }
    if game.remote.is_some() {
        if station && slot == 324 {
            let output = game.container_kind().and_then(|entity| entity.slots().get(24)).and_then(Option::as_ref);
            if output.is_some_and(|stack| !crate::game::tconstruct_ui::station_accepts_output(game, stack)) { return Break(true); }
        }
        if !station && slot == 322 && right {
            crate::game::remote::tconstruct_source_quick_packet::send(game, 1, 0);
            return Break(true);
        }
        return Continue(());
    }
    let repair_slot = station && slot == 303 && crate::game::tconstruct_ui::station_layout(game).is_some_and(|layout| layout.tool_slot.is_some());
    if (302..=323).contains(&slot) && (!station && slot != 322 || station && slot != 302 && !repair_slot) { return Break(true); }
    if !station && slot == 322 {
        crate::game::tconstruct_source_builder_take::take(game, key);
        return Break(true);
    }
    let action = if station && (324..=325).contains(&slot) { Some((slot - 300) as i16) } else { None };
    let Some(action) = action else { return Continue(()); };
    if right { return Break(true); }
    let mut building = if action == 24 {
        game.world.block_entities.get(&key).and_then(|entity| {
            crate::game::tconstruct_source_station_world::building_plan(&game.world, entity.slots())
        })
    } else { None };
    if let Some((_, output)) = building.as_mut() {
        let name = game.world.chunks.__horizon_mixins_tconstruct_station_name_edit().get(&key).map(|edit| edit.value.as_str());
        crate::game::tconstruct_source_station_name::name_result(output, name);
    }
    let repair = if action == 25 {
        game.world.block_entities.get(&key).and_then(|entity| crate::game::tconstruct_source_station_world::operation_plan(&game.world, entity.slots()))
    } else { None };
    if action == 25 {
        if let Some(plan) = repair { crate::game::tconstruct_source_station_take_host::take(game, key, plan); }
        return Break(true);
    }
    if building.as_ref().is_some_and(|(_, output)| !crate::game::tconstruct_ui::station_accepts_output(game, output)) { return Break(true); }
    let max_stack = building.as_ref().and_then(|(_, output)| game.res.rest.get()?.items.get(&output.item).map(|info| info.max_stack)).unwrap_or(0);
    let Some(BlockEntity::Chest(stacks)) = game.world.block_entities.get_mut(&key) else { return Break(true); };
    let taken = if action == 24 {
        building.is_some_and(|(matched, output)| crate::game::tconstruct_source_station_recipes::take_built_tool(stacks, &mut game.carried, &matched, output, max_stack))
    } else { false };
    if taken {
        game.world.modified.insert((key.0 >> 4, key.2 >> 4));
        game.world.chunks.__horizon_mixins_tconstruct_station_name_edit_mut().remove(&key);
    }
    Break(true)
}
