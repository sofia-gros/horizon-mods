fn smeltery_scroll(game: &mut crate::game::Game<'_>, delta: f32) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    if !crate::game::tconstruct_source_smeltery_ui::is_smeltery(game) { return Continue(()); }
    let Some(pos) = crate::game::tconstruct_source_smeltery_ui::position(game) else { return Break(()); };
    let (count, offset) = crate::game::tconstruct_source_smeltery_ui::page(game);
    let max = crate::game::tconstruct_source_smeltery_layout::max_offset(count);
    if max == 0 || delta == 0.0 { return Break(()); }
    let Some(size) = game.renderer.as_ref().map(|r| [r.config.width, r.config.height]) else { return Break(()); };
    let batch = horizon_client::hud::HudBatch::new(size);
    let (ox, oy) = game.inventory_origin(&batch);
    let left = crate::game::tconstruct_source_smeltery_layout::panel_left(count);
    let height = crate::game::tconstruct_source_smeltery_layout::visible_rows(count) as f32 * 18.0 + 14.0;
    if !(ox + left..ox).contains(&game.cursor.0) || !(oy + 4.0..oy + 4.0 + height).contains(&game.cursor.1) { return Continue(()); }
    let next = (offset as i32 - delta.signum() as i32).clamp(0, max as i32) as usize;
    crate::game::tconstruct_source_smeltery_ui::set_offset(game, pos, count, next);
    Break(())
}
