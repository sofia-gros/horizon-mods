fn tconstruct_client_data(game: &crate::game::Game<'_>, source: &Option<ItemStack>) -> std::ops::ControlFlow<Option<Stack>> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(item) = source.as_ref().filter(|s| s.item.starts_with("tconstruct:")) else { return Continue(()); };
    let custom = item.data().and_then(|data| data.other("minecraft:custom_data")).cloned();
    let lore = item.data().and_then(|data| data.other("minecraft:lore")).and_then(|tag| match tag {
        horizon_world::nbt::Tag::List(lines) => Some(lines.clone()),
        _ => None,
    });
    let model = item.data().and_then(|data| data.other("minecraft:item_model")).and_then(|tag| match tag {
        horizon_world::nbt::Tag::String(id) => Some(id.to_string_lossy()),
        _ => None,
    });
    if custom.is_none() && lore.is_none() && model.is_none() { return Continue(()); }
    let Some(remote) = game.remote.as_ref() else { return Continue(()); };
    let mut result = horizon_client::lan::guest::stack_to_network(
        |name| game.item_id(name),
        |registry, name| remote.builtin.get(registry).and_then(|ids| ids.id(name)).or_else(|| remote.registries.id(registry, name)),
        source,
    );
    if let Some(stack) = result.as_mut() {
        if let Some(custom) = custom {
            stack.added.push((horizon_protocol::item::component::CUSTOM_DATA, horizon_protocol::item::Component::Nbt(custom)));
        }
        if let Some(lore) = lore {
            stack.added.push((horizon_protocol::item::component::LORE, horizon_protocol::item::Component::Lore(lore)));
        }
        if let Some(model) = model {
            stack.added.push((horizon_protocol::item::component::ITEM_MODEL, horizon_protocol::item::Component::Ident(model)));
        }
    }
    Break(result)
}
