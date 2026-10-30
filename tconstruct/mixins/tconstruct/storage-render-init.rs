fn tconstruct_storage_render_init(game: &mut Game<'_>) {
    let _ = game.world.chunks.__horizon_mixins_tconstruct_render_assets()
        .get_or_init(|| crate::game::tconstruct_source_render_info::load_render_assets(&game.options.resources));
}
