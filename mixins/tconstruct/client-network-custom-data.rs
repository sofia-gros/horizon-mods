fn tconstruct_client_data(game: &crate::game::Game<'_>, source: &Option<ItemStack>) -> std::ops::ControlFlow<Option<Stack>> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(item) = source.as_ref().filter(|s| &*s.item == "tconstruct:flint_pickaxe") else { return Continue(()); };
    let Some(custom) = item.data().and_then(|data| data.other("minecraft:custom_data")).cloned() else { return Continue(()); };
    let Some(remote) = game.remote.as_ref() else { return Continue(()); };
    let mut result = horizon_client::lan::guest::stack_to_network(
        |name| game.item_id(name),
        |registry, name| remote.builtin.get(registry).and_then(|ids| ids.id(name)).or_else(|| remote.registries.id(registry, name)),
        source,
    );
    if let Some(stack) = result.as_mut() {
        stack.added.push((horizon_protocol::item::component::CUSTOM_DATA, horizon_protocol::item::Component::Nbt(custom)));
    }
    Break(result)
}
