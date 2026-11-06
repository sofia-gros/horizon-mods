// Isolated candidate inside lan::guest. Not installed in the normal package.
fn preserve_name_removal(
    item_id: &dyn Fn(&str) -> i32,
    synced_id: &dyn Fn(&str, &str) -> Option<i32>,
    stack: &Option<ItemStack>,
) -> std::ops::ControlFlow<Option<Stack>> {
    // No stack observation or callback is allowed before this consumption.
    if pluto_name_entry_gate::take_native_entry() {
        return std::ops::ControlFlow::Continue(());
    }
    if !stack.as_ref().and_then(|s| s.data())
        .is_some_and(|d| d.other("!minecraft:item_name").is_some())
    {
        return std::ops::ControlFlow::Continue(());
    }
    // The next call re-enters this HEAD and consumes the one-shot flag before
    // the native body. Nested calls occur after consumption and are independent.
    // No ItemStack clone or input mutation is introduced here.
    let result = pluto_name_entry_gate::call_native(|| network_stack(item_id, synced_id, stack));
    let Ok(mut output) = result else {
        // An impossible duplicate arm must not become unchecked transmission.
        return std::ops::ControlFlow::Break(None);
    };
    if let Some(value) = &mut output {
        let name = horizon_protocol::item::component::ITEM_NAME;
        value.added.retain(|(id, _)| *id != name);
        value.removed.retain(|id| *id != name);
        value.removed.push(name);
    }
    std::ops::ControlFlow::Break(output)
}
