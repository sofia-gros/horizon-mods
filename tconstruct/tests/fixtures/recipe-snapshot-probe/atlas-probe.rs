fn inspect_pattern_atlas(game: &crate::game::Game<'_>) {
    if crate::game::tconstruct_ui::kind(game) == Some(false) {
        let patterns = game.gui.atlas.sprites.keys().filter(|key| key.starts_with("gui/tconstruct_patterns/")).count();
        eprintln!("TCONSTRUCT_CLIENT_PATTERN_ATLAS {patterns}");
        if game.remote.is_some() {
            if let Some(key) = game.container {
                if let Some((_, _, Some(patterns))) = game.world.chunks.__horizon_mixins_tconstruct_builder_choices().get(&key) {
                    eprintln!("TCONSTRUCT_GUEST_BUILDER_CHOICES {}", patterns.join(","));
                }
            }
        }
    }
}
