fn tconstruct_crafting_recipe_guard(game: &mut crate::game::Game<'_>) -> std::ops::ControlFlow<()> {
    if game.remote.is_none() && crate::game::tconstruct_crafting_station::position(game).is_some() {
        game.message = Some(("Recipe-book placement is unavailable at the Crafting Station in this development build".into(), std::time::Instant::now()));
        return std::ops::ControlFlow::Break(());
    }
    std::ops::ControlFlow::Continue(())
}
