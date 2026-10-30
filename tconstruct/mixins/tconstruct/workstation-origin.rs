fn workstation_origin(game: &crate::game::Game<'_>, batch: &horizon_client::hud::HudBatch) -> std::ops::ControlFlow<(f32, f32)> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(station) = crate::game::tconstruct_ui::kind(game) else { return Continue(()); };
    let (base, y) = batch.centered_origin(176.0, game.screen_height());
    let left = if station { -132.0 } else {
        let count = crate::game::tconstruct_ui::side_count(game);
        if count == 0 { 0.0 } else { crate::game::tconstruct_source_side_layout::left(count) }
    };
    let count = crate::game::tconstruct_ui::side_count(game);
    let width = crate::game::tconstruct_source_side_layout::panel_width(batch.width, count, station);
    let right = if station { 182.0 + width } else { 180.0 + width };
    Break((crate::game::tconstruct_source_side_layout::adjusted_origin(batch.width, base, left, right), y))
}
