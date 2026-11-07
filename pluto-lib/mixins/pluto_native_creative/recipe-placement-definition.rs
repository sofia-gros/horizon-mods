fn pluto_recipe_placement_definition(
    recipes: &Recipes,
    id: &str,
) -> std::ops::ControlFlow<Option<Placement>> {
    if matches!(
        crate::crafting::pluto_placement::definition(recipes, id),
        pluto_inventory_api::recipe_book::placement::Outcome::Denied { .. }
    ) {
        std::ops::ControlFlow::Break(None)
    } else {
        std::ops::ControlFlow::Continue(())
    }
}
