fn pluto_cooking_recipes_tick(game: &mut crate::game::Game<'_>) {
    crate::entry::pluto_cooking_recipes::refresh(game.recipes());
}
