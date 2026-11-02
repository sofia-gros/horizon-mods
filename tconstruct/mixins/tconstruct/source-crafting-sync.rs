fn tconstruct_crafting_sync(value: (), game: &mut crate::game::Game<'_>) {
    crate::game::tconstruct_crafting_station::sync(game);
    value
}
