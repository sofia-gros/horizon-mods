// HEAD on Game::command_damage_stand: a fancy stand is fire immune at
// the class level, so a fire damage type is refused outright (the host's
// own fire path returns false too, but lights and burns the stand first).
fn pluto_hanging_stand_command(
    game: &mut Game<'_>,
    id: u32,
    damage_type: &str,
    causing: Option<super::commands::Target>,
) -> core::ops::ControlFlow<bool> {
    let _ = causing;
    if crate::game::pluto_hanging_game::fancy_stand_refuses(game, id, damage_type) {
        return std::ops::ControlFlow::Break(false);
    }
    std::ops::ControlFlow::Continue(())
}
