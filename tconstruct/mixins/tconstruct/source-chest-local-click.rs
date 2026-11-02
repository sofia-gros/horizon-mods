fn tconstruct_chest_local_click(game: &mut crate::game::Game<'_>, right: bool) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    if game.remote.is_some() { return Continue(()); }
    let Some(size) = game.renderer.as_ref().map(|renderer| [renderer.config.width, renderer.config.height]) else { return Continue(()); };
    let batch = horizon_client::hud::HudBatch::new(size);
    let Some(slot) = game.slot_at(&batch, game.cursor.0, game.cursor.1) else { return Continue(()); };
    let chest = crate::game::tconstruct_chest_ui::kind(game).is_some();
    let builder = crate::game::tconstruct_ui::side_chest_kind(game).is_some()
        && (slot < 36 || (300..302).contains(&slot) || (327..381).contains(&slot));
    let crafting = crate::game::tconstruct_crafting_station::side(game).is_some()
        && ((600..648).contains(&slot) || (slot < 36 && game.has_shift_down()));
    if !(chest || builder || crafting) { return Continue(()); }
    let input = if game.has_shift_down() { horizon_client::menu::Input::QuickMove } else { horizon_client::menu::Input::Pickup };
    game.menu_click(Some(slot), i32::from(right), input);
    Break(())
}
