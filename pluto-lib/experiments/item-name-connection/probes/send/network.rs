fn unchanged_network(
    item_id: &dyn Fn(&str) -> i32,
    synced_id: &dyn Fn(&str, &str) -> Option<i32>,
    stack: &Option<ItemStack>,
) -> std::ops::ControlFlow<Option<Stack>> {
    let _ = (item_id, synced_id, stack);
    std::ops::ControlFlow::Continue(())
}
