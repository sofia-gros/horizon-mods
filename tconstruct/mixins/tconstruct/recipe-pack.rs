fn tconstruct_pack_recipes(value: (), game: &mut crate::game::Game<'_>) -> () {
    tconstruct_recipe_reload::invalidate_parked_item_tags(game);
    tconstruct_recipe_reload::capture(game);
    value
}
