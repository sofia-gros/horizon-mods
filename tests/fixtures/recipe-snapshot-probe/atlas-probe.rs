fn inspect_pattern_atlas(game: &crate::game::Game<'_>) {
    if crate::game::tconstruct_ui::kind(game) == Some(false) {
        let patterns = game.gui.atlas.sprites.keys().filter(|key| key.starts_with("gui/tconstruct_patterns/")).count();
        eprintln!("TCONSTRUCT_CLIENT_PATTERN_ATLAS {patterns}");
    }
}
