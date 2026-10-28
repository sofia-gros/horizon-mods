fn broken_harvest(held: Option<&ItemStack>) -> std::ops::ControlFlow<bool> {
    if held.is_some_and(|stack| matches!(&*stack.item, "tconstruct:flint_pickaxe" | "tconstruct:pickaxe") && stack.damage >= 85) {
        return std::ops::ControlFlow::Break(false);
    }
    std::ops::ControlFlow::Continue(())
}
