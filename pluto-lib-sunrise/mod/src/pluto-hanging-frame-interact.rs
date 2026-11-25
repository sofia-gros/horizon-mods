// HEAD on Game::interact_frame: a fancy frame runs its variant's
// interaction (clear click-through, rotation rules); anything else runs
// the host's vanilla interaction.
fn pluto_hanging_frame_interact(
    game: &mut Game<'_>,
    target: Option<InteractTarget>,
) -> std::ops::ControlFlow<bool> {
    let Some((i, facts, distance)) = crate::game::pluto_hanging_game::fancy_frame_target(game, &target) else {
        return std::ops::ControlFlow::Continue(());
    };
    std::ops::ControlFlow::Break(crate::game::pluto_hanging_game::interact_fancy_frame(
        game, target.is_some(), i, &facts, distance,
    ))
}
