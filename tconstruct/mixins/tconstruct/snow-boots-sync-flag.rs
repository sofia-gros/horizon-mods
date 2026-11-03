fn tconstruct_snow_boots_sync_flag(value: bool, game: &crate::game::Game<'_>) -> bool {
    value || crate::game::tconstruct_source_snow_boots::walks(game)
}
