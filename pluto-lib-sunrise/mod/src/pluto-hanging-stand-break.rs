// HEAD on Game::break_armor_stand_by_hit: a fancy stand broken by a
// player pops its variant's item (brokenByPlayer, custom name kept);
// anything else runs the host's vanilla break.
fn pluto_hanging_stand_break(
    game: &mut Game<'_>,
    i: usize,
    creative: bool,
    always_kills: bool,
    by: Option<super::commands::Target>,
) -> std::ops::ControlFlow<bool> {
    let Some(facts) = crate::game::pluto_hanging_game::stand_facts(game, &game.armor_stands[i]) else {
        return std::ops::ControlFlow::Continue(());
    };
    std::ops::ControlFlow::Break(crate::game::pluto_hanging_game::break_fancy_stand(
        game, i, creative, always_kills, by, &facts,
    ))
}
