//! Connects the blocking module to PlutoLib's per-stack item use: a right click with a shield tool whose
//! modifiers carry it raises the tool like a shield for as long as it is held (BlockingModule.onToolUse,
//! with its use duration of 72000 and the BLOCK pose), as the game's own shields block. The three shield
//! items are also edible tools, so this registration — made after the edible one, replacing it for these
//! items — chooses the pose per stack as the original's hook order does (the edible hook runs first, at
//! the higher priority): a stack that can be eaten is eaten, a blocking stack blocks. One difference the
//! per-stack use cannot express: it has a single hunger gate, and this registration sets none, so a
//! player at full hunger holding an edible shield starts the eat pose instead of blocking; the finish
//! event still refuses to feed them, as EdibleModule.eat does. No shipped shield is edible by default.
use crate::{break_speed_hook::with_held_tool, edible, edible_hook, tool_stack};
use pluto_lib::{Identifier, ItemUseRegistry, StackData, StackUse, UseAction};

/// The shield items, the tools that ship with the blocking modifier (BlockingModule's tools).
const SHIELD_ITEMS: [&str; 3] = ["battlesign", "travelers_shield", "plate_shield"];

/// BlockingModule.getUseDuration.
const BLOCK_TICKS: i32 = 72000;

/// Whether the stack is a whole Tinkers tool whose modifiers carry the blocking module.
fn blocking(stack: &StackData) -> bool {
    with_held_tool(stack, |env, tool| tool_stack::blocking(&tool.custom_data, env)).unwrap_or(false)
}

/// Whether the stack can start being eaten, as the edible registration asks (EdibleModule.onToolUse).
fn eatable(stack: &StackData) -> bool {
    edible_hook::eating(stack).is_some_and(|(hunger, _)| edible::can_start(hunger, false))
}

/// Registers the use for the shield items. Called once from the product's hooks, after the edible hook.
pub(crate) fn register() {
    for path in SHIELD_ITEMS {
        let Ok(item) = Identifier::new("tconstruct", path) else { continue };
        ItemUseRegistry::register_stack(item, StackUse::new(UseAction::Block)
            .can_use(|stack| blocking(stack) || eatable(stack))
            .action_for(|stack| {
                if eatable(stack) { return Some(UseAction::Eat); }
                blocking(stack).then_some(UseAction::Block)
            })
            .use_time(|stack| {
                if eatable(stack) { return edible_hook::eating(stack).map_or(0, |(_, duration)| edible::use_ticks(duration)); }
                if blocking(stack) { BLOCK_TICKS } else { 0 }
            }));
    }
}
