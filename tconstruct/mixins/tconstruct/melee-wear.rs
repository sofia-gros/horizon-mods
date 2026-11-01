fn tconstruct_melee_wear(game: &mut crate::game::Game<'_>, stack: &ItemStack) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let dynamic = stack.item.starts_with("tconstruct:") && matches!(stack.data().and_then(|data| data.other("minecraft:custom_data")).and_then(|tag| tag.get("tic_max_damage")), Some(horizon_world::nbt::Tag::Int(max)) if *max > 0);
    let legacy = matches!(&*stack.item, "tconstruct:flint_pickaxe" | "tconstruct:pickaxe");
    if !legacy && !dynamic { return Continue(()); }
    if !game.creative {
        // Preserve each registered tool's attack cost while keeping the
        // material-derived broken tool instead of deleting its stack.
        let cost = if dynamic {
            crate::game::tconstruct_source_melee_shape::builtin_attack_wear(stack.item.as_ref())
                .unwrap_or_else(|| game.res.items().get(&stack.item).and_then(|item| item.weapon_damage).unwrap_or(0))
        } else { 2 };
        let (loss, overslime) = crate::game::tconstruct_source_overslime_host::damage(game, stack, cost);
        if let Some(held) = game.held_mut().as_mut().filter(|held| held.item == stack.item) {
            crate::game::tconstruct_source_overslime::spend(held, overslime);
            crate::game::tconstruct_tx::fixed_damage(held, loss);
        }
    }
    Break(())
}
