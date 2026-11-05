fn tconstruct_fake_material_book_placement(value: Option<Placement>, recipes: &Recipes, id: &str) -> Option<Placement> {
    if recipes.crafting.iter().any(|(name, recipe)| name == id
        && crate::recipe_book::tconstruct_source_fake_material_book::marked(recipe)) {
        // The native place engine cannot select by Material. Refuse before it
        // clears/moves slots; hand-arranged recipes still use the assembly guard.
        None
    } else { value }
}
