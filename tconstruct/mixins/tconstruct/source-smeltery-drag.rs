fn smeltery_drag(game: &mut crate::game::Game<'_>) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    if !crate::game::tconstruct_source_smeltery_ui::is_smeltery(game) { return Continue(()); }
    let Some(pos) = crate::game::tconstruct_source_smeltery_ui::position(game) else { return Continue(()); };
    if !game.world.chunks.__horizon_mixins_tconstruct_smeltery_ui_dragging().contains(&pos) { return Continue(()); }
    let Some(size) = game.renderer.as_ref().map(|r| [r.config.width, r.config.height]) else { return Break(()); };
    let batch = horizon_client::hud::HudBatch::new(size);
    let (_, oy) = game.inventory_origin(&batch);
    let (count, _) = crate::game::tconstruct_source_smeltery_ui::page(game);
    let next = crate::game::tconstruct_source_smeltery_layout::drag_offset(count, game.cursor.1 - oy);
    crate::game::tconstruct_source_smeltery_ui::set_offset(game, pos, count, next);
    Break(())
}
