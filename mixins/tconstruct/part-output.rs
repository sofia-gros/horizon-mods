fn tconstruct_part_output(game: &mut crate::game::Game<'_>, slot: Option<usize>, button: i32, input: horizon_client::menu::Input) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(key) = game.container else { return Continue(()); };
    let Some(station) = crate::game::tconstruct_ui::kind(game) else { return Continue(()); };
    if input == horizon_client::menu::Input::QuickMove && !station && slot == Some(322) {
        if button == 0 {
            if game.remote.is_some() { crate::game::remote::tconstruct_source_quick_packet::send(game); }
            else { crate::game::tconstruct_source_builder_take::quick_take(game, key); }
        }
        return Break(true);
    }
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
    if game.remote.is_some() { return Continue(()); }
    if (302..=323).contains(&slot.unwrap_or(0)) && (!station && slot != Some(322) || station && slot != Some(302)) { return Break(true); }
    if !station && slot == Some(322) {
        if button == 0 { crate::game::tconstruct_source_builder_take::take(game, key); }
        return Break(true);
    }
    let action = if station { slot.filter(|s| (324..=325).contains(s)).map(|s| (s - 300) as i16) } else { None };
    let Some(action) = action else { return Continue(()); };
    if button != 0 { return Break(true); }
    let Some(BlockEntity::Chest(stacks)) = game.world.block_entities.get_mut(&key) else { return Break(true); };
    if crate::game::tconstruct_tx::take(stacks, &mut game.carried, station, action) {
        game.world.modified.insert((key.0 >> 4, key.2 >> 4));
    }
    Break(true)
}
