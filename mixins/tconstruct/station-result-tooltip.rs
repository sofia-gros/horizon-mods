fn tconstruct_result_tooltip(game: &mut crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(station) = crate::game::tconstruct_ui::kind(game) else { return Continue(()); };
    if !station {
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
    let result = if station && slot == 324 {
        crate::game::tconstruct_source_station_world::building_plan(&game.world, slots).map(|(_, output)| output)
    } else if station {
        crate::game::tconstruct_source_station_world::swap_plan(&game.world, slots).or_else(|| crate::game::tconstruct_source_station_world::repair_plan(&game.world, slots)).map(|(output, _)| output)
    } else {
        if game.remote.is_some() { slots.get(22).cloned().flatten() }
        else { game.container.and_then(|key| game.world.chunks.__horizon_mixins_tconstruct_builder_preview().get(&key).and_then(|(_, _, _, result)| result.clone())) }
    };
    if let Some(result) = result { game.draw_stack_tooltip(batch, &result, game.cursor.0, game.cursor.1); }
    Break(())
}
