fn tconstruct_ensure_recipes(value: (), game: &mut crate::game::Game<'_>) -> () {
    if game.fns.loaded && game.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().is_none() {
        tconstruct_recipe_reload::capture(game);
    }
    value
}
