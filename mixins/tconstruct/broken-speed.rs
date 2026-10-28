fn broken_speed(held: Option<&ItemStack>) -> std::ops::ControlFlow<f32> {
    if held.is_some_and(|stack| matches!(&*stack.item, "tconstruct:flint_pickaxe" | "tconstruct:pickaxe") && stack.damage >= 85) {
        return std::ops::ControlFlow::Break(0.3);
    }
    std::ops::ControlFlow::Continue(())
}
