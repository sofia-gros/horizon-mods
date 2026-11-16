//! Connects the conditional mining speed modifiers to PlutoLib's break-speed event: the held tool's
//! modifiers change how fast the player digs the block, as MiningSpeedToolHook does in the original.
use crate::{common, mining_speed::Context, tool_definitions, tool_mining, tool_parts, tool_stack, tool_station::{self, Context as Tools}};
use pluto_lib::{BreakSpeedCallback, BreakSpeedContext, Identifier, ItemSlot};

/// The speed after the held tool's modifiers; None when the held stack is not a loaded Tinkers tool.
fn compute(dig: &BreakSpeedContext) -> Option<f32> {
    let held = dig.held.as_ref()?;
    if !held.custom_data.contains_key(tool_stack::TAG_MATERIALS) { return None; }
    let item = Identifier::parse(&held.item).ok()?;
    let definitions = tool_definitions::register().read().ok()?;
    let materials = common::register().read().ok()?;
    let modifiers = common::server_modifiers()?.read().ok()?;
    if !materials.server_loaded || !modifiers.definitions_loaded || definitions.data.is_empty() { return None; }
    let tools: Vec<(Identifier, &tool_definitions::Data)> = definitions.data.iter().map(|(id, data)| (id.clone(), data)).collect();
    let item_max = |id: &Identifier| tool_parts::tool_stack_size(id);
    let item_tags = common::item_snapshot();
    let ctx = Tools { materials: &materials, modifiers: &modifiers, item_tags_loaded: item_tags.is_some(), item_tags: item_tags.as_ref(), tools: &tools, item_max: &item_max };
    let mut stack = ItemSlot::new(item.clone(), held.count);
    stack.custom_data = held.custom_data.clone();
    let definition = tool_station::definition_for(&ctx, &stack)?;
    let env = ctx.environment(&item, definition);
    if tool_stack::is_broken(&stack.custom_data) { return None; }
    let block_in_tag = |tag: &Identifier| dig.block.is_in_tag(&tag.to_string());
    let tier = tool_stack::harvest_tier(&stack.custom_data);
    let effective = definition.effective().is_some_and(|effective| tool_mining::is_effective(effective, &tier, &block_in_tag));
    // What vanilla applied on top of the tool's own rule speed: haste, fatigue, water and air.
    let rule_speed = if effective { tool_stack::stat(&stack.custom_data, crate::tool_stats::Stat::MiningSpeed).max(1.0) } else { 1.0 };
    let multiplier = if rule_speed > 0.0 { dig.original / rule_speed } else { 1.0 };
    let context = Context {
        block: &dig.block.block, block_in_tag: &block_in_tag, original: dig.original, multiplier, effective,
        on_ground: dig.player.map(|player| player.on_ground), eyes_in_water: dig.player.map(|player| player.eyes_in_water),
    };
    Some(tool_stack::break_speed(&stack.custom_data, &env, &context, dig.speed.get()))
}

/// Registers the callback. Called once from the product's hooks.
pub(crate) fn register() {
    BreakSpeedCallback::EVENT.register(|dig| {
        if let Some(speed) = compute(dig) { if speed.is_finite() && speed >= 0.0 { dig.speed.set(speed); } }
    });
}
