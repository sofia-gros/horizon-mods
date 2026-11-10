fn tconstruct_shell_gut_add(
    game: &crate::game::Game<'_>,
    effect: &str,
    amplifier: i32,
) -> std::ops::ControlFlow<bool> {
    if crate::game::tconstruct_shell_gut_world::immune(game, effect, amplifier) {
        std::ops::ControlFlow::Break(false)
    } else {
        std::ops::ControlFlow::Continue(())
    }
}
