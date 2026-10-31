fn smeltery_local_click(game: &mut crate::game::Game<'_>, right: bool) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    if game.remote.is_some() || !crate::game::tconstruct_source_smeltery_ui::is_smeltery(game) { return Continue(()); }
    let Some(size) = game.renderer.as_ref().map(|renderer| [renderer.config.width, renderer.config.height]) else { return Continue(()); };
    let batch = horizon_client::hud::HudBatch::new(size);
    let Some(slot) = game.slot_at(&batch, game.cursor.0, game.cursor.1) else { return Continue(()); };
    let input = if game.has_shift_down() { horizon_client::menu::Input::QuickMove } else { horizon_client::menu::Input::Pickup };
    game.menu_click(Some(slot), i32::from(right), input);
    Break(())
}
