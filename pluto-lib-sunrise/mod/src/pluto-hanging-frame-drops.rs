// HEAD on Game::drop_frame_items: a fancy frame drops its variant's item
// (every break path — player hit, projectile, blast, losing its wall —
// funnels through here); anything else runs the host's vanilla drops.
fn pluto_hanging_frame_drops(
    game: &mut Game<'_>,
    f: &mut ItemFrame,
    with_frame: bool,
    by_player: bool,
    creative: bool,
) -> core::ops::ControlFlow<()> {
    if crate::game::pluto_hanging_game::fancy_frame_drops(game, f, with_frame, by_player, creative) {
        return std::ops::ControlFlow::Break(());
    }
    std::ops::ControlFlow::Continue(())
}
