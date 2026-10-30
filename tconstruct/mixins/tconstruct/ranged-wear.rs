fn tconstruct_ranged_wear(game: &mut crate::game::Game<'_>, amount: u32) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(stack) = game.held().cloned() else { return Continue(()); };
    if !crate::game::tconstruct_source_ranged_wear::is_material_bow(&stack) { return Continue(()); }
    if !game.creative {
        let loss = game.durability_loss(&stack, amount);
        if let Some(held) = game.held_mut().as_mut().filter(|held| held.item == stack.item) {
            crate::game::tconstruct_tx::fixed_damage(held, loss);
        }
    }
    Break(())
}
