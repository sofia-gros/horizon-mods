//! Connects the conditional mining speed modifiers to PlutoLib's break-speed event: the held tool's
//! modifiers change how fast the player digs the block, as MiningSpeedToolHook does in the original.
use crate::{common, mining_speed::Context, tool_definitions, tool_mining, tool_parts, tool_stack, tool_station::{self, Context as Tools}};
use pluto_lib::{BreakSpeedCallback, BreakSpeedContext, Identifier, ItemSlot};

/// Runs `run` over the held stack as a loaded Tinkers tool and its environment; `Err` names why the stack is left alone.
pub(crate) fn with_held_tool<R>(held: &pluto_lib::StackData, run: impl FnOnce(&tool_stack::Environment, &ItemSlot) -> R) -> Result<R, &'static str> {
    if !held.custom_data.contains_key(tool_stack::TAG_MATERIALS) { return Err("held stack is not a Tinkers tool"); }
    let item = Identifier::parse(&held.item).map_err(|_| "held item id")?;
    let definitions = tool_definitions::register().read().map_err(|_| "tool definitions locked")?;
    let materials = common::register().read().map_err(|_| "materials locked")?;
    let modifiers = common::server_modifiers().ok_or("modifier registry missing")?.read().map_err(|_| "modifiers locked")?;
    if !materials.server_loaded || !modifiers.definitions_loaded || definitions.data.is_empty() { return Err("registries not loaded"); }
    let tools: Vec<(Identifier, &tool_definitions::Data)> = tool_definitions::item_pairs(&definitions);
    let item_max = |id: &Identifier| tool_parts::tool_stack_size(id);
    let item_tags = common::item_snapshot();
    let ctx = Tools { materials: &materials, modifiers: &modifiers, item_tags_loaded: item_tags.is_some(), item_tags: item_tags.as_deref(), tools: &tools, item_max: &item_max };
    let mut stack = ItemSlot::new(item.clone(), held.count);
    stack.custom_data = held.custom_data.clone();
    let definition = tool_station::definition_for(&ctx, &stack).ok_or("no tool definition for the held item")?;
    let env = ctx.environment(&item, definition);
    if tool_stack::is_broken(&stack.custom_data) { return Err("the tool is broken"); }
    Ok(run(&env, &stack))
}

/// The speed after the held tool's modifiers; `Err` names why the held stack is left alone.
fn compute(dig: &BreakSpeedContext) -> Result<(f32, bool, f32, usize), &'static str> {
    let held = dig.held.as_ref().ok_or("no held stack")?;
    with_held_tool(held, |env, stack| {
        let block_in_tag = |tag: &Identifier| dig.block.is_in_tag(&tag.to_string());
        let tier = tool_stack::harvest_tier(&stack.custom_data);
        let effective = env.definition.and_then(|definition| definition.effective()).is_some_and(|effective| tool_mining::is_effective(effective, &tier, &block_in_tag));
        // What vanilla applied on top of the tool's own rule speed: haste, fatigue, water and air.
        let rule_speed = if effective { tool_stack::stat(&stack.custom_data, crate::tool_stats::Stat::MiningSpeed).max(1.0) } else { 1.0 };
        let multiplier = if rule_speed > 0.0 { dig.original / rule_speed } else { 1.0 };
        let context = Context {
            block: &dig.block.block, block_in_tag: &block_in_tag, original: dig.original, multiplier, effective,
            on_ground: dig.player.map(|player| player.on_ground), eyes_in_water: dig.player.map(|player| player.eyes_in_water),
            facts: crate::mining_speed::BlockFacts { block_light: dig.block_light, sky_light: dig.sky_light, sky_darken: dig.sky_darken, biome_temperature: dig.biome_temperature },
        };
        let modifier_count = tool_stack::saved_modifiers(&stack.custom_data, env.modifiers).len();
        (tool_stack::break_speed(&stack.custom_data, env, &context, dig.speed.get()), effective, multiplier, modifier_count)
    })
}

/// Registers the callback. Called once from the product's hooks.
pub(crate) fn register() {
    // The first reason a dig was left alone is logged once, so a silent no-op is visible in the game log.
    static LOGGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    static CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    static CHANGED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    static WITH_MODIFIERS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    BreakSpeedCallback::EVENT.register(|dig| {
        match compute(dig) {
            Ok((speed, effective, multiplier, modifiers)) => {
                // The first digs, the first digs with modifiers and the first digs a modifier changed are logged with their numbers, to see the hook work in a game log.
                let changed = (speed - dig.speed.get()).abs() > f32::EPSILON;
                if CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 2
                    || (changed && CHANGED.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 6)
                    || (modifiers > 0 && WITH_MODIFIERS.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 2) {
                    eprintln!("Tinkers: break speed {} original {} speed {} -> {} (effective {effective}, multiplier {multiplier}, modifiers {modifiers})", dig.block.block, dig.original, dig.speed.get(), speed);
                }
                if speed.is_finite() && speed >= 0.0 { dig.speed.set(speed); }
            }
            Err(reason) => {
                if dig.held.is_some() && !LOGGED.swap(true, std::sync::atomic::Ordering::Relaxed) { eprintln!("Tinkers: break speed hook left a dig alone: {reason}"); }
            }
        }
    });
}
