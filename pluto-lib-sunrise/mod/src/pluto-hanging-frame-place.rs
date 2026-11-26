// HEAD on Game::place_item_frame: a registered frame variant item places
// a fancy frame (the variant is the item's); anything else runs the host's
// vanilla placement.
fn pluto_hanging_frame_place(
    game: &mut Game<'_>,
    hit: &Hit,
    stack: &ItemStack,
) -> std::ops::ControlFlow<bool> {
    let Some(facts) = crate::pluto_hanging_access::frame_for_item(&stack.item) else {
        return std::ops::ControlFlow::Continue(());
    };
    std::ops::ControlFlow::Break(crate::game::pluto_hanging_game::place_fancy_frame(game, hit, stack, &facts))
}
