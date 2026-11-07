fn pluto_crafting_match_veto(
    input: &Input<'_>,
    recipe: &Recipe,
) -> std::ops::ControlFlow<Option<Option<ItemStack>>> {
    if matches!(
        crate::crafting::pluto_matching::evaluate(input, recipe),
        pluto_inventory_api::crafting::matching::Outcome::Denied { .. }
    ) {
        std::ops::ControlFlow::Break(None)
    } else {
        std::ops::ControlFlow::Continue(())
    }
}
