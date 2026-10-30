fn tconstruct_wear(game: &mut crate::game::Game<'_>, hardness: f32) -> std::ops::ControlFlow<()> {
    let Some(stack) = game.inventory[game.selected].clone() else { return std::ops::ControlFlow::Continue(()); };
    if stack.item.as_ref() != "tconstruct:flint_pickaxe" && !(stack.item.starts_with("tconstruct:") && matches!(stack.data().and_then(|data| data.other("minecraft:custom_data")).and_then(|tag| tag.get("tic_max_damage")), Some(horizon_world::nbt::Tag::Int(max)) if *max > 0)) { return std::ops::ControlFlow::Continue(()); }
    if hardness != 0.0 {
        let loss = game.durability_loss(&stack, 1);
        if let Some(held) = game.inventory[game.selected].as_mut() {
            crate::game::tconstruct_tx::fixed_damage(held, loss);
        }
    }
    game.stats.add("used", &stack.item, 1);
    std::ops::ControlFlow::Break(())
}
