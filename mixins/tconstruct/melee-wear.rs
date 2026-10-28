fn tconstruct_melee_wear(game: &mut crate::game::Game<'_>, stack: &ItemStack) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    if !matches!(&*stack.item, "tconstruct:flint_pickaxe" | "tconstruct:pickaxe") { return Continue(()); }
    if !game.creative {
        // ToolAttackUtil: a pickaxe is not MELEE_PRIMARY, so a living hit costs two.
        let loss = game.durability_loss(stack, 2);
        if let Some(held) = game.held_mut().as_mut().filter(|held| held.item == stack.item) {
            crate::game::tconstruct_tx::fixed_damage(held, loss);
        }
    }
    Break(())
}
