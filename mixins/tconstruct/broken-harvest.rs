fn broken_harvest(held: Option<&ItemStack>) -> std::ops::ControlFlow<bool> {
    if held.is_some_and(|stack| matches!(&*stack.item, "tconstruct:flint_pickaxe" | "tconstruct:pickaxe") && match stack.data().and_then(|data| data.other("minecraft:custom_data")).and_then(|tag| tag.get("tic_max_damage")) {
        Some(horizon_world::nbt::Tag::Int(max)) if *max > 0 => stack.damage >= *max as u32,
        _ => stack.damage >= 85,
    }) {
        return std::ops::ControlFlow::Break(false);
    }
    std::ops::ControlFlow::Continue(())
}
