//! Connects the modules that hear a block the player's tool breaks to PlutoLib's block-break and loot events: mining capacity (the bar a
//! break fills or drains, before the break with `run_before` and after it otherwise: BlockHarvestModifierHook), loot to capacity (the loot of
//! the block, eaten into a bar: ProcessLootModifierHook) and the block-break half of tool usage effects (MobEffectModule.ToolUsage's
//! afterBlockBreak). The edits land as `EntityAction::EditStackData` on the main hand. An area tool is not ported, so a break harvests one block.
use crate::{break_speed_hook::with_held_tool, common, tool_damage_hook, tool_stack, weapon_effect::Usage, weapon_effect_hook::usage_actions};
use pluto_lib::{ActionTarget, EntityAction, Identifier, LootTableEvents, PlayerBlockBreakEvents, StackData, StackDataEdit, StackSlot};

/// The action that puts `edits` on the stack of the tool in the main hand; None when there is nothing to change.
fn edit_action(held: &StackData, edits: Vec<StackDataEdit>) -> Option<EntityAction> {
    (!edits.is_empty()).then(|| EntityAction::EditStackData { target: ActionTarget::Attacker, slot: StackSlot::MainHand, item: held.item.clone(), edits })
}

/// Registers the callbacks. Called once from the product's hooks.
pub(crate) fn register() {
    // startHarvest: the modules that run before the break (overwield spends overslime as the tool starts to dig). Runs even when a later
    // callback vetoes the break, as the original runs it before the break is attempted.
    PlayerBlockBreakEvents::BEFORE.register(|broken| {
        if let Some(held) = broken.held.as_ref() {
            if let Ok(edits) = with_held_tool(held, |env, stack| tool_stack::mining_capacity_edits(&stack.custom_data, env, true)) {
                broken.actions.borrow_mut().extend(edit_action(held, edits));
            }
        }
        true
    });
    // finishHarvest (one block harvested) and afterBlockBreak: the capacity modules that run after the break and the effects the tool puts on
    // its holder. The tool is read as the break left it.
    PlayerBlockBreakEvents::AFTER.register(|broken| {
        let Some(held) = broken.held.as_ref() else { return };
        if let Ok(edits) = with_held_tool(held, |env, stack| tool_stack::mining_capacity_edits(&stack.custom_data, env, false)) {
            broken.actions.borrow_mut().extend(edit_action(held, edits));
        }
        broken.actions.borrow_mut().extend(usage_actions(&[&broken.held], Usage::BlockBreak));
    });
    // processLoot: the loot of a block a player's tool broke. ModifierLootModifier runs it for tools of the loot capable tag that are not broken.
    LootTableEvents::MODIFY_DROPS.register(|drops| {
        let (Some(tool), Some(_), true) = (drops.tool.as_ref(), drops.player, drops.block.is_some()) else { return };
        let Some(items) = common::item_snapshot() else { return };
        let in_tag = |item: &Identifier, tag: &Identifier| items.get_tag(tag).is_some_and(|members| members.contains(item));
        let (Ok(item), Ok(capable)) = (Identifier::parse(&tool.item), Identifier::new("tconstruct", "modifiable/loot_capable_tool")) else { return };
        if !in_tag(&item, &capable) { return; }
        let mut loot = drops.drops.borrow_mut();
        let eaten = with_held_tool(tool, |env, stack| tool_stack::loot_capacity(&stack.custom_data, env, &mut loot, &in_tag, &mut tool_damage_hook::random));
        if let Ok(edits) = eaten { drops.actions.borrow_mut().extend(edit_action(tool, edits)); }
    });
}
