fn tconstruct_pack_recipes(value: (), game: &mut crate::game::Game<'_>) -> () {
    tconstruct_recipe_reload::capture(game);
    value
}
