fn tconstruct_crop_use(game: &mut crate::game::Game<'_>, hit: Option<&crate::game::Hit>) -> std::ops::ControlFlow<Result<bool>> {
    crate::game::tconstruct_source_crop_harvest::harvest(game, hit)
}
