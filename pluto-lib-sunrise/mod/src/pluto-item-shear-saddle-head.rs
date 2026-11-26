// Adapted from pinned MIT Horizon game/mounts.rs::shear_saddle.
// License: pluto-lib/LICENSE.horizon. Unregistered queries run the original.
fn pluto_shear_saddle(game: &mut Game<'_>, index: usize) -> core::ops::ControlFlow<Option<bool>> {
    // As in `interact_shear_mob`: NeoForge asks the stack for SHEARS_HARVEST where
    // vanilla names the shears item; a stack that answers yes runs the original body.
    let held = game.held();
    if held.as_ref().is_some_and(|stack| stack.item == "minecraft:shears") {
        return core::ops::ControlFlow::Continue(());
    }
    let granted = held.is_some_and(|stack| {
        let data = match stack.data().and_then(|d| d.other("minecraft:custom_data")) {
            Some(tag @ sunrise_core::nbt::Tag::Compound(_)) => sunrise_client_core::scoreboard::tag_to_json(tag),
            _ => serde_json::Value::Object(serde_json::Map::new()),
        };
        crate::pluto_item_behavior_access::tool_action(&stack.item, &data, "shears_harvest") == Some(true)
    });
    if granted { return core::ops::ControlFlow::Break(game.shear_saddle(index)); }
    core::ops::ControlFlow::Continue(())
}
