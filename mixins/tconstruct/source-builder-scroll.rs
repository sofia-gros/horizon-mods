fn builder_scroll(game: &mut crate::game::Game<'_>, delta: f32) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    if crate::game::tconstruct_ui::kind(game) != Some(false) { return Continue(()); }
    let Some(pos) = game.container else { return Break(()); };
    let count = game.world.chunks.__horizon_mixins_tconstruct_builder_choices().get(&pos)
        .and_then(|(_, _, list)| list.as_ref()).map_or(0, Vec::len);
    let state = game.world.chunks.__horizon_mixins_tconstruct_builder_scroll_mut().entry(pos)
        .or_insert_with(|| (0.0, 0, false, Vec::new()));
    state.0 = crate::game::tconstruct_source_builder_layout::wheel(count, state.0, f64::from(delta));
    state.1 = crate::game::tconstruct_source_builder_layout::offset(count, state.0);
    Break(())
}
