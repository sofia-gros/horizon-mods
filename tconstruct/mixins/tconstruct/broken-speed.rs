fn broken_speed(held: Option<&ItemStack>) -> std::ops::ControlFlow<f32> {
    if held.is_some_and(|stack| stack.item.starts_with("tconstruct:") && match stack.data().and_then(|data| data.other("minecraft:custom_data")).and_then(|tag| tag.get("tic_max_damage")) {
        Some(horizon_world::nbt::Tag::Int(max)) if *max > 0 => stack.damage >= *max as u32,
        _ => stack.item.as_ref() == "tconstruct:flint_pickaxe" && stack.damage >= 85,
    }) {
        return std::ops::ControlFlow::Break(0.3);
    }
    std::ops::ControlFlow::Continue(())
}
