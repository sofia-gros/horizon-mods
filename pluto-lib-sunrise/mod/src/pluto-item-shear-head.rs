// Adapted from pinned MIT Horizon game/creatures.rs::interact_shear_mob.
// License: pluto-lib/LICENSE.horizon. Unregistered queries run the original.
fn pluto_shear_mob(game: &mut Game<'_>, index: usize, shears: bool) -> core::ops::ControlFlow<bool> {
    if shears { return core::ops::ControlFlow::Continue(()); }
    // NeoForge asks the stack (`ItemStack.canPerformAction(ToolActions.SHEARS_HARVEST)`)
    // where vanilla names the shears item. A stack that answers yes runs the original
    // body as if it held shears, so its loot, sound and tool damage stay the game's.
    let granted = game.held().is_some_and(|stack| {
        #[cfg(test)]
        let data = match stack.data().and_then(|d| d.other("minecraft:custom_data")) {
            Some(tag @ sunrise_world::nbt::Tag::Compound(_)) => sunrise_client_core::scoreboard::tag_to_json(tag),
            _ => serde_json::Value::Object(serde_json::Map::new()),
        };
        #[cfg(not(test))]
        let data = match stack.data().and_then(|d| d.other("minecraft:custom_data")) {
            Some(tag @ sunrise_core::nbt::Tag::Compound(_)) => sunrise_client_core::scoreboard::tag_to_json(tag),
            _ => serde_json::Value::Object(serde_json::Map::new()),
        };
        #[cfg(test)]
        { sunrise_world::level::pluto_item_behavior_access::tool_action(&stack.item, &data, "shears_harvest") == Some(true) }
        #[cfg(not(test))]
        { crate::pluto_item_behavior_access::tool_action(&stack.item, &data, "shears_harvest") == Some(true) }
    });
    if granted { return core::ops::ControlFlow::Break(game.interact_shear_mob(index, true)); }
    core::ops::ControlFlow::Continue(())
}
