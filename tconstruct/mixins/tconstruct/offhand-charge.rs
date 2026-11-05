fn tconstruct_offhand_charge(game: &crate::game::Game<'_>) -> std::ops::ControlFlow<f32> {
    match crate::game::tconstruct_offhand_attack_world::charge(game) {
        Some(value) => std::ops::ControlFlow::Break(value),
        None => std::ops::ControlFlow::Continue(()),
    }
}
