fn pluto_crafting_match_veto(
    input: &Input<'_>,
    recipe: &Recipe,
) -> std::ops::ControlFlow<Option<Option<ItemStack>>> {
    crate::crafting::pluto_derivations::after_matching(
        crate::crafting::pluto_matching::evaluate(input, recipe),
        input,
        recipe,
    )
}
