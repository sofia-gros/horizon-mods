// HEAD on Game::place_armor_stand: a registered stand variant item places
// a fancy stand (variant properties, left-handed flag); anything else
// runs the host's vanilla placement.
fn pluto_hanging_stand_place(
    game: &mut Game<'_>,
    hit: &super::Hit,
    stack: &ItemStack,
) -> core::ops::ControlFlow<bool> {
    let Some(facts) = crate::pluto_hanging_access::stand_for_item(&stack.item) else {
        return std::ops::ControlFlow::Continue(());
    };
    std::ops::ControlFlow::Break(crate::game::pluto_hanging_game::place_fancy_stand(game, hit, stack, &facts))
}
