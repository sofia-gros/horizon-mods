fn tconstruct_pointer(game: &mut crate::game::Game<'_>, right: bool) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(station) = crate::game::tconstruct_ui::kind(game) else { return Continue(()); };
    let Some(key) = game.container else { return Break(true); };
    let Some(size) = game.renderer.as_ref().map(|r| [r.config.width, r.config.height]) else { return Break(true); };
    let batch = horizon_client::hud::HudBatch::new(size);
    if game.has_shift_down() {
        if !station && game.slot_at(&batch, game.cursor.0, game.cursor.1) == Some(322) {
            if game.remote.is_some() { crate::game::remote::tconstruct_source_quick_packet::send(game, i32::from(right), 1); }
            else { crate::game::tconstruct_source_builder_take::quick_take(game, key); }
        }
        return Break(true);
    }
    if !station {
        let (ox, oy) = game.inventory_origin(&batch);
        let (x, y) = ((game.cursor.0 - ox) as i32, (game.cursor.1 - oy) as i32);
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
        if !station && slot == 322 && right {
            crate::game::remote::tconstruct_source_quick_packet::send(game, 1, 0);
            return Break(true);
        }
        return Continue(());
    }
    if (302..=323).contains(&slot) && (!station && slot != 322 || station && slot != 302) { return Break(true); }
    if !station && slot == 322 {
        crate::game::tconstruct_source_builder_take::take(game, key);
        return Break(true);
    }
    let action = if station && (324..=325).contains(&slot) { Some((slot - 300) as i16) } else { None };
    let Some(action) = action else { return Continue(()); };
    if right { return Break(true); }
    let building = if action == 24 {
        game.world.block_entities.get(&key).and_then(|entity| {
            crate::game::tconstruct_source_station_world::building_plan(&game.world, entity.slots())
        })
    } else { None };
    let Some(BlockEntity::Chest(stacks)) = game.world.block_entities.get_mut(&key) else { return Break(true); };
    let taken = if action == 24 {
        building.is_some_and(|(matched, output)| crate::game::tconstruct_source_station_recipes::take_built_tool(stacks, &mut game.carried, &matched, output, 1))
    } else { crate::game::tconstruct_tx::take(stacks, &mut game.carried, station, action) };
    if taken {
        game.world.modified.insert((key.0 >> 4, key.2 >> 4));
    }
    Break(true)
}
