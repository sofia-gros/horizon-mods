fn tconstruct_result_tooltip(game: &mut crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(station) = crate::game::tconstruct_ui::kind(game) else { return Continue(()); };
    if station {
        let (ox, oy) = game.inventory_origin(batch);
        if (ox + 182.0..ox + 308.0).contains(&game.cursor.0) {
            if let Some((top, bottom)) = crate::game::tconstruct_station_info_ui::lines(game) {
                let offsets = game.container.and_then(|pos| game.world.chunks.__horizon_mixins_tconstruct_station_info_scroll().get(&pos)).copied().unwrap_or_default();
                let selected = if (oy + 10.0..oy + 78.0).contains(&game.cursor.1) { Some((&top, offsets.0, oy + 10.0, crate::game::tconstruct_station_info_ui::TOP_LINE_HEIGHT, crate::game::tconstruct_station_info_ui::TOP_VISIBLE)) }
                    else if (oy + 99.0..oy + 166.5).contains(&game.cursor.1) { Some((&bottom, offsets.1, oy + 99.0, crate::game::tconstruct_station_info_ui::BOTTOM_LINE_HEIGHT, crate::game::tconstruct_station_info_ui::BOTTOM_VISIBLE)) }
                    else { None };
                if let Some((lines, scroll, start, line_height, visible)) = selected {
                    let scroll = scroll.min(lines.len().saturating_sub(visible));
                    let row = ((game.cursor.1 - start) / line_height) as usize;
                    let tip = if row == 0 && game.cursor.0 >= ox + 296.0 && lines.iter().any(|(_, tip)| tip.is_some()) {
                        game.pack.names.display_get("gui.tconstruct.general.hover")
                    } else { lines.get(scroll + row).and_then(|(_, tip)| tip.clone()) };
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
        if (ox + 180.0..ox + 306.0).contains(&game.cursor.0) && (oy + 5.0..oy + 178.0).contains(&game.cursor.1) {
            let entries = crate::game::tconstruct_builder_info_ui::lines(game, &game.gui.atlas);
            let scroll = game.container.and_then(|pos| game.world.chunks.__horizon_mixins_tconstruct_builder_info_scroll().get(&pos)).copied().unwrap_or(0).min(entries.len().saturating_sub(crate::game::tconstruct_builder_info_ui::VISIBLE_LINES));
            let row = ((game.cursor.1 - oy - 5.0) / crate::game::tconstruct_builder_info_ui::LINE_HEIGHT) as usize;
            let tip = if row == 0 && game.cursor.0 >= ox + 294.0 && entries.iter().any(|(_, _, tip)| tip.is_some()) {
                game.pack.names.display_get("gui.tconstruct.general.hover")
            } else { entries.get(scroll + row).and_then(|(_, _, tip)| tip.clone()) };
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
