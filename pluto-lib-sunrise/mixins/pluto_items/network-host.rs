// Adapted from Tinkers' network-custom-data.rs, horizon-mods commit 37c780c0d968756d5f6a0213b2f0fa798219b930.
fn pluto_host_data(hub: &mut crate::lan::hub::Hub, source: &Option<ItemStack>) -> std::ops::ControlFlow<Option<horizon_protocol::item::Stack>> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(item) = source.as_ref() else { return Continue(()); };
    let Some(custom) = item.data().and_then(|data| data.other("minecraft:custom_data")).cloned() else { return Continue(()); };
    if custom.get("plutolib:data").is_none() { return Continue(()); }
    let name = item.item.clone();
    let id = hub.item_id(&name);
    let sync = hub.sync.clone();
    let item_id = |other: &str| if other == &*name { id } else { sync.builtin.get("minecraft:item").and_then(|r| r.id(other)).unwrap_or(0) };
    let mut result = stack_to_network(item_id, |registry, value| sync.id(registry, value), source);
    if let Some(stack) = result.as_mut() { stack.added.push((horizon_protocol::item::component::CUSTOM_DATA, horizon_protocol::item::Component::Nbt(custom))); }
    Break(result)
}
