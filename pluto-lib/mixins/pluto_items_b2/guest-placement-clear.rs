fn pluto_clear_guest_placement_policy(
    game: &mut crate::game::Game<'_>,
    swap: &crate::game::lan_actions::GuestSwap,
) {
    game.world.chunks.__horizon_mixins_pluto_items_b2_placement_guest_check_mut().remove(&swap.conn);
}
