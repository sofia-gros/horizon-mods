fn refresh_local_builder_choices(game: &mut crate::game::Game<'_>) {
    if game.remote.is_some() || crate::game::tconstruct_ui::kind(game) != Some(false) { return; }
    if let Some(pos) = game.container {
        let patterns = crate::game::tconstruct_source_builder_world::refresh(&mut game.world, pos).unwrap_or_default();
        let scroll = game.world.chunks.__horizon_mixins_tconstruct_builder_scroll_mut().entry(pos)
            .or_insert_with(|| (0.0, 0, false, Vec::new()));
        if scroll.3 != patterns {
            crate::game::tconstruct_source_builder_layout::update_display(patterns.len(), scroll);
            scroll.3 = patterns;
        }
        if let Some(rest) = game.res.rest.get() {
            let items = &rest.items.registry_order;
            crate::game::tconstruct_source_builder_world::refresh_preview(&mut game.world, pos,
                || items.iter().map(|id| id.to_string()).collect());
        }
    }
}
