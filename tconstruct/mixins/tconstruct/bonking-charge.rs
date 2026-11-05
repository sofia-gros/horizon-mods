fn tconstruct_bonking_charge(game: &crate::game::Game<'_>) -> std::ops::ControlFlow<f32> {
    match crate::game::tconstruct_bonking_world::attack_charge(game) {
        Some(charge) => std::ops::ControlFlow::Break(charge),
        None => std::ops::ControlFlow::Continue(()),
    }
}
