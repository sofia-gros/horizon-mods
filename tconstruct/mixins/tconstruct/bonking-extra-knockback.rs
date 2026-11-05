fn tconstruct_bonking_extra(game: &crate::game::Game<'_>) -> std::ops::ControlFlow<(f32, bool)> {
    if crate::game::tconstruct_bonking_world::attack_charge(game).is_some() {
        std::ops::ControlFlow::Break((0.0, false))
    } else {
        std::ops::ControlFlow::Continue(())
    }
}
