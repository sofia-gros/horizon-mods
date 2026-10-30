fn tconstruct_remote_bow_held(
    stack: &horizon_client::items::ItemStack,
) -> std::ops::ControlFlow<bool> {
    if matches!(&*stack.item, "tconstruct:longbow" | "tconstruct:crossbow") {
        std::ops::ControlFlow::Break(true)
    } else {
        std::ops::ControlFlow::Continue(())
    }
}
