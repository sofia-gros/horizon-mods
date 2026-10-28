fn tconstruct_pointer(game: &mut crate::game::Game<'_>, right: bool) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(station) = crate::game::tconstruct_ui::kind(game) else { return Continue(()); };
    if game.remote.is_some() { return Continue(()); }
    if game.has_shift_down() { return Break(true); }
    let Some(key) = game.container else { return Break(true); };
    let Some(size) = game.renderer.as_ref().map(|r| [r.config.width, r.config.height]) else { return Break(true); };
    let batch = horizon_client::hud::HudBatch::new(size);
    let Some(slot) = game.slot_at(&batch, game.cursor.0, game.cursor.1) else { return Continue(()); };
    if (302..=323).contains(&slot) && (!station || slot != 302) { return Break(true); }
    if !(324..=326).contains(&slot) { return Continue(()); }
    if right { return Break(true); }
    let Some(BlockEntity::Chest(stacks)) = game.world.block_entities.get_mut(&key) else { return Break(true); };
    if crate::game::tconstruct_tx::take(stacks, &mut game.carried, station, (slot - 300) as i16) {
        game.world.modified.insert((key.0 >> 4, key.2 >> 4));
    }
    Break(true)
}
