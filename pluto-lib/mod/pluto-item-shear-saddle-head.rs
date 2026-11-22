// Adapted from pinned MIT Horizon game/mounts.rs::shear_saddle.
// License: pluto-lib/LICENSE.horizon. Unregistered queries run the original.
fn pluto_shear_saddle(game: &mut Game<'_>, index: usize, item: &str) -> std::ops::ControlFlow<Option<bool>> {
    if item == "minecraft:shears" { return std::ops::ControlFlow::Continue(()); }
    // As in `interact_shear_mob`: NeoForge asks the stack for SHEARS_HARVEST where
    // vanilla names the shears item; a stack that answers yes runs the original
    // body under the vanilla name.
    let granted = game.held().is_some_and(|stack| {
        let data = match stack.data().and_then(|d| d.other("minecraft:custom_data")) {
            Some(tag @ horizon_world::nbt::Tag::Compound(_)) => horizon_client_core::scoreboard::tag_to_json(tag),
            _ => serde_json::Value::Object(serde_json::Map::new()),
        };
        horizon_world::level::pluto_item_behavior_access::tool_action(&stack.item, &data, "shears_harvest") == Some(true)
    });
    if granted { return std::ops::ControlFlow::Break(game.shear_saddle(index, "minecraft:shears")); }
    std::ops::ControlFlow::Continue(())
}
