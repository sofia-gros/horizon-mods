fn pluto_block_fire_odds(level: &GameLevel<'_>, _dimension: LevelDimension, state: BlockStateId, odds: &mut Option<(i32, i32)>) {
    super::pluto_block_level::fire_odds(level, state, odds);
}
