fn tconstruct_reload_recipes(value: R<i32>, game: &mut crate::game::Game<'_>) -> R<i32> {
    if value.is_ok() { tconstruct_recipe_reload::capture(game); }
    value
}
