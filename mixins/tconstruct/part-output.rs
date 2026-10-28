fn tconstruct_part_output(game: &mut crate::game::Game<'_>, slot: Option<usize>, button: i32, input: horizon_client::menu::Input) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(key) = game.container else { return Continue(()); };
    let Some(station) = crate::game::tconstruct_ui::kind(game) else { return Continue(()); };
    if input != horizon_client::menu::Input::Pickup { return Break(true); }
    if !station && slot.is_some_and(|s| (324..=326).contains(&s)) {
        if button == 0 {
            let index = (slot.unwrap() - 324) as i16;
            if let Some((window, _)) = game.remote.as_ref().and_then(|r| r.menu.as_ref()) {
                game.send(horizon_protocol::packets::play::Serverbound::ContainerButtonClick(horizon_protocol::packets::play::ContainerButtonClick { container: *window, button: i32::from(index) }));
            } else if game.container_kind().and_then(|e| e.slots().first()).and_then(Option::as_ref).is_some_and(|s| &*s.item == "tconstruct:pattern") {
                if let Some(pattern) = crate::game::tconstruct_tx::preview_pattern(index) {
                    game.world.chunks.__horizon_mixins_tconstruct_part_selections_mut().insert(key, pattern.to_owned());
                }
            }
        }
        return Break(true);
    }
    if game.remote.is_some() { return Continue(()); }
    if (302..=323).contains(&slot.unwrap_or(0)) && (!station && slot != Some(322) || station && slot != Some(302)) { return Break(true); }
    let action = if !station && slot == Some(322) {
        game.world.chunks.__horizon_mixins_tconstruct_part_selections().get(&key).and_then(|s| crate::game::tconstruct_tx::preview_pattern_index(s)).map(|s| s + 24)
    } else if station { slot.filter(|s| (324..=325).contains(s)).map(|s| (s - 300) as i16) } else { None };
    let Some(action) = action else { return Continue(()); };
    if button != 0 { return Break(true); }
    let Some(BlockEntity::Chest(stacks)) = game.world.block_entities.get_mut(&key) else { return Break(true); };
    if crate::game::tconstruct_tx::take(stacks, &mut game.carried, station, action) {
        game.world.modified.insert((key.0 >> 4, key.2 >> 4));
    }
    Break(true)
}
