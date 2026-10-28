fn tconstruct_broken(game: &mut crate::game::Game<'_>) -> std::ops::ControlFlow<Result<()>> {
    if game.held().is_some_and(|stack| &*stack.item == "tconstruct:flint_pickaxe" && stack.damage >= 85) {
        return std::ops::ControlFlow::Break(Ok(()));
    }
    std::ops::ControlFlow::Continue(())
}
