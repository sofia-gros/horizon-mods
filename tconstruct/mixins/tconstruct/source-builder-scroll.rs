fn builder_scroll(game: &mut crate::game::Game<'_>, delta: f32) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    if crate::game::tconstruct_ui::kind(game) != Some(false) { return Continue(()); }
    let Some(pos) = game.container else { return Break(()); };
    if let Some(size) = game.renderer.as_ref().map(|renderer| [renderer.config.width, renderer.config.height]) {
        let batch = horizon_client::hud::HudBatch::new(size);
        let (ox, oy) = game.inventory_origin(&batch);
        let count = crate::game::tconstruct_ui::side_count(game);
        let columns = crate::game::tconstruct_source_side_layout::columns(count);
        let max = crate::game::tconstruct_source_side_layout::max_offset(count);
        let left = crate::game::tconstruct_source_side_layout::left(count);
        let width = columns as f32 * 18.0 + 24.0;
        let height = crate::game::tconstruct_source_side_layout::visible_rows(count) as f32 * 18.0 + 14.0;
        if max > 0 && (ox + left..ox + left + width).contains(&game.cursor.0) && (oy..oy + height).contains(&game.cursor.1) {
            let state = game.world.chunks.__horizon_mixins_tconstruct_builder_side_scroll_mut().entry(pos).or_default();
            state.0 = (state.0 as i32 - delta.signum() as i32).clamp(0, max as i32) as usize;
            return Break(());
        }
        if (ox + 180.0..ox + 306.0).contains(&game.cursor.0) && (oy..oy + 184.0).contains(&game.cursor.1) {
            let count = crate::game::tconstruct_builder_info_ui::lines(game, &game.gui.atlas).len();
            let scroll = game.world.chunks.__horizon_mixins_tconstruct_builder_info_scroll_mut().entry(pos).or_default();
            *scroll = (*scroll as i32 - delta.signum() as i32).clamp(0, count.saturating_sub(17) as i32) as usize;
            return Break(());
        }
    }
    let count = game.world.chunks.__horizon_mixins_tconstruct_builder_choices().get(&pos)
        .and_then(|(_, _, list)| list.as_ref()).map_or(0, Vec::len);
    let state = game.world.chunks.__horizon_mixins_tconstruct_builder_scroll_mut().entry(pos)
        .or_insert_with(|| (0.0, 0, false, Vec::new()));
    state.0 = crate::game::tconstruct_source_builder_layout::wheel(count, state.0, f64::from(delta));
    state.1 = crate::game::tconstruct_source_builder_layout::offset(count, state.0);
    Break(())
}
