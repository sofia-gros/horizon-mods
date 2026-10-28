fn tconstruct_wear(game: &mut crate::game::Game<'_>, hardness: f32) -> std::ops::ControlFlow<()> {
    let Some(stack) = game.inventory[game.selected].clone() else { return std::ops::ControlFlow::Continue(()); };
    if &*stack.item != "tconstruct:flint_pickaxe" { return std::ops::ControlFlow::Continue(()); }
    if hardness != 0.0 {
        let loss = game.durability_loss(&stack, 1);
        if let Some(held) = game.inventory[game.selected].as_mut() {
            held.damage = held.damage.saturating_add(loss).min(85);
        }
    }
    game.stats.add("used", &stack.item, 1);
    std::ops::ControlFlow::Break(())
}
