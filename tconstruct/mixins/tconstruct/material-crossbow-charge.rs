fn tconstruct_crossbow_charge(
    stack: &horizon_client::items::ItemStack,
) -> std::ops::ControlFlow<u32> {
    if &*stack.item == "tconstruct:crossbow" {
        std::ops::ControlFlow::Break(crate::game::tconstruct_ranged_draw::draw_ticks(stack))
    } else {
        std::ops::ControlFlow::Continue(())
    }
}
