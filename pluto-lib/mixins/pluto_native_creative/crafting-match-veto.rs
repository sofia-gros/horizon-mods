fn pluto_crafting_match_veto(
    input: &Input<'_>,
    recipe: &Recipe,
) -> std::ops::ControlFlow<Option<Option<ItemStack>>> {
    match crate::crafting::pluto_matching::evaluate(input, recipe) {
        crate::crafting::pluto_matching::Verdict::Denied => {
            std::ops::ControlFlow::Break(None)
        }
        crate::crafting::pluto_matching::Verdict::Assembled(stack) => {
            std::ops::ControlFlow::Break(Some(stack))
        }
        crate::crafting::pluto_matching::Verdict::Native => {
            std::ops::ControlFlow::Continue(())
        }
    }
}
