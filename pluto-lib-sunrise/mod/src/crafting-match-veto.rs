fn pluto_crafting_match_veto(
    input: &Input<'_>,
    recipe: &Recipe,
) -> std::ops::ControlFlow<Option<Option<ItemStack>>> {
    match sunrise_client_lib::pluto_matching::evaluate(input, recipe) {
        sunrise_client_lib::pluto_matching::Verdict::Denied => {
            std::ops::ControlFlow::Break(None)
        }
        sunrise_client_lib::pluto_matching::Verdict::Assembled(stack) => {
            std::ops::ControlFlow::Break(Some(stack))
        }
        sunrise_client_lib::pluto_matching::Verdict::Native => {
            std::ops::ControlFlow::Continue(())
        }
    }
}
