// Adapted from pinned MIT Horizon items/enchant.rs::enchantable_of.
// License: pluto-lib/LICENSE.horizon. Unregistered queries run the original.
fn pluto_item_enchantability(stack: &ItemStack) -> std::ops::ControlFlow<u32> {
    // The stack's own custom data, as the item behaviour transport carries it (JSON).
    let data = match stack.data().and_then(|d| d.other("minecraft:custom_data")) {
        Some(tag @ horizon_world::nbt::Tag::Compound(_)) => horizon_client_core::scoreboard::tag_to_json(tag),
        _ => serde_json::Value::Object(serde_json::Map::new()),
    };
    // A registered per-stack answer is the stack's value: NeoForge's stack-sensitive
    // `getEnchantmentValue(ItemStack)` is the item's own override, ahead of the
    // `minecraft:enchantable` component the default reads. No answer runs the
    // original component/default lookup below.
    match horizon_world::level::pluto_item_behavior_access::enchantability(&stack.item, &data) {
        Some(value) => std::ops::ControlFlow::Break(value.max(0) as u32),
        None => std::ops::ControlFlow::Continue(()),
    }
}
