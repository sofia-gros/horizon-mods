fn tconstruct_pointer(game: &mut crate::game::Game<'_>, right: bool) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(station) = crate::game::tconstruct_ui::kind(game) else { return Continue(()); };
    if game.has_shift_down() { return Break(true); }
    let Some(key) = game.container else { return Break(true); };
    let Some(size) = game.renderer.as_ref().map(|r| [r.config.width, r.config.height]) else { return Break(true); };
    let batch = horizon_client::hud::HudBatch::new(size);
    let Some(slot) = game.slot_at(&batch, game.cursor.0, game.cursor.1) else { return Continue(()); };
    if !station && (324..=326).contains(&slot) {
        if !right {
            let index = (slot - 324) as i16;
            if let Some((window, _)) = game.remote.as_ref().and_then(|r| r.menu.as_ref()) {
                game.send(horizon_protocol::packets::play::Serverbound::ContainerButtonClick(horizon_protocol::packets::play::ContainerButtonClick { container: *window, button: i32::from(index) }));
            } else if game.container_kind().and_then(|e| e.slots().first()).and_then(Option::as_ref).is_some_and(|s| &*s.item == "tconstruct:pattern") {
                game.world.chunks.__horizon_mixins_tconstruct_part_selections_mut().insert(key, index);
            }
        }
        return Break(true);
    }
    if game.remote.is_some() { return Continue(()); }
    if (302..=323).contains(&slot) && (!station && slot != 322 || station && slot != 302) { return Break(true); }
    let action = if !station && slot == 322 {
        game.world.chunks.__horizon_mixins_tconstruct_part_selections().get(&key).copied().map(|s| s + 24)
    } else if station && (324..=325).contains(&slot) { Some((slot - 300) as i16) } else { None };
    let Some(action) = action else { return Continue(()); };
    if right { return Break(true); }
    let Some(BlockEntity::Chest(stacks)) = game.world.block_entities.get_mut(&key) else { return Break(true); };
    if crate::game::tconstruct_tx::take(stacks, &mut game.carried, station, action) {
        game.world.modified.insert((key.0 >> 4, key.2 >> 4));
    }
    Break(true)
}
