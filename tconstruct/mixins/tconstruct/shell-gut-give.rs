fn tconstruct_shell_gut_give(
    game: &crate::game::Game<'_>,
    effect: &str,
    amplifier: i32,
) -> std::ops::ControlFlow<()> {
    if crate::game::tconstruct_shell_gut_world::immune(game, effect, amplifier) {
        std::ops::ControlFlow::Break(())
    } else {
        std::ops::ControlFlow::Continue(())
    }
}
