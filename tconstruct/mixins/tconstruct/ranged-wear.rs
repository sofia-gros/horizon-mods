fn tconstruct_ranged_wear(game: &mut crate::game::Game<'_>, amount: u32) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(stack) = game.held().cloned() else { return Continue(()); };
    if !crate::game::tconstruct_source_ranged_wear::is_material_bow(&stack) { return Continue(()); }
    if !game.creative {
        let (loss, overslime) = crate::game::tconstruct_source_overslime_host::damage(game, &stack, amount);
        if let Some(held) = game.held_mut().as_mut().filter(|held| held.item == stack.item) {
            crate::game::tconstruct_source_overslime::spend(held, overslime);
            crate::game::tconstruct_tx::fixed_damage(held, loss);
        }
    }
    Break(())
}
