fn tconstruct_network_data(hub: &mut crate::lan::hub::Hub, source: &Option<ItemStack>) -> std::ops::ControlFlow<Option<horizon_protocol::item::Stack>> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(item) = source.as_ref().filter(|s| &*s.item == "tconstruct:flint_pickaxe") else { return Continue(()); };
    let custom = item.data().and_then(|data| data.other("minecraft:custom_data")).cloned();
    let lore = item.data().and_then(|data| data.other("minecraft:lore")).and_then(|tag| match tag {
        horizon_world::nbt::Tag::List(lines) => Some(lines.clone()),
        _ => None,
    });
    if custom.is_none() && lore.is_none() { return Continue(()); }
    let name = item.item.clone();
    let id = hub.item_id(&name);
    let sync = hub.sync.clone();
    let item_id = |other: &str| {
        if other == &*name { return id; }
        sync.builtin.get("minecraft:item").and_then(|r| r.id(other)).unwrap_or(0)
    };
    let mut result = stack_to_network(item_id, |registry, value| sync.id(registry, value), source);
    if let Some(stack) = result.as_mut() {
        if let Some(custom) = custom {
            stack.added.push((horizon_protocol::item::component::CUSTOM_DATA, horizon_protocol::item::Component::Nbt(custom)));
        }
        if let Some(lore) = lore {
            stack.added.push((horizon_protocol::item::component::LORE, horizon_protocol::item::Component::Lore(lore)));
        }
    }
    Break(result)
}
