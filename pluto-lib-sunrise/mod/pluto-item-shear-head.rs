// Adapted from pinned MIT Horizon game/creatures.rs::interact_shear_mob.
// License: pluto-lib/LICENSE.horizon. Unregistered queries run the original.
fn pluto_shear_mob(game: &mut Game<'_>, index: usize, item: &str) -> std::ops::ControlFlow<bool> {
    if item == "minecraft:shears" { return std::ops::ControlFlow::Continue(()); }
    // NeoForge asks the stack (`ItemStack.canPerformAction(ToolActions.SHEARS_HARVEST)`)
    // where vanilla names the shears item. A stack that answers yes runs the original
    // body under the vanilla name, so its loot, sound and tool damage stay the game's.
    let granted = game.held().is_some_and(|stack| {
        let data = match stack.data().and_then(|d| d.other("minecraft:custom_data")) {
            Some(tag @ sunrise_world::nbt::Tag::Compound(_)) => sunrise_client_core::scoreboard::tag_to_json(tag),
            _ => serde_json::Value::Object(serde_json::Map::new()),
        };
        sunrise_world::level::pluto_item_behavior_access::tool_action(&stack.item, &data, "shears_harvest") == Some(true)
    });
    if granted { return std::ops::ControlFlow::Break(game.interact_shear_mob(index, "minecraft:shears")); }
    std::ops::ControlFlow::Continue(())
}
