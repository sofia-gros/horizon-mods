// The native return is Option<Option<ItemStack>>: None is "does not match"
// (the veto), Some(Some(stack)) is "matched, with this result".
fn pluto_crafting_match_veto(
    ctx: &Context<'_>,
    input: &Input<'_>,
    recipe: &Recipe,
) -> std::ops::ControlFlow<Option<Option<ItemStack>>> {
    match sunrise_client_lib::pluto_matching::evaluate(ctx, input, recipe) {
        sunrise_client_lib::pluto_matching::Verdict::Denied => {
            std::ops::ControlFlow::Break(None)
        }
        sunrise_client_lib::pluto_matching::Verdict::Unmatched => {
            std::ops::ControlFlow::Break(None)
        }
        sunrise_client_lib::pluto_matching::Verdict::Assembled(stack) => {
            std::ops::ControlFlow::Break(Some(Some(stack)))
        }
        sunrise_client_lib::pluto_matching::Verdict::Native => {
            std::ops::ControlFlow::Continue(())
        }
    }
}
