//! Connects the modules that hear a block the player's tool breaks to PlutoLib's block-break and loot events: mining capacity (the bar a
//! break fills or drains, before the break with `run_before` and after it otherwise: BlockHarvestModifierHook), loot to capacity (the loot of
//! the block, eaten into a bar: ProcessLootModifierHook) and the block-break half of tool usage effects (MobEffectModule.ToolUsage's
//! afterBlockBreak). The edits land as `EntityAction::EditStackData` on the main hand. An area tool's extra blocks ride the same
//! before-break callback through `break_extra` (see area_break).
use crate::{break_speed_hook::with_held_tool, common, tool_damage_hook, tool_mining, tool_stack, weapon_effect::Usage, weapon_effect_hook::usage_actions};
use pluto_lib::{ActionTarget, BlockBreakContext, EntityAction, Identifier, LootTableEvents, Nbt, PlayerBlockBreakEvents, StackData, StackDataEdit, StackSlot, World};

/// The action that puts `edits` on the stack of the tool in the main hand; None when there is nothing to change.
fn edit_action(held: &StackData, edits: Vec<StackDataEdit>) -> Option<EntityAction> {
    (!edits.is_empty()).then(|| EntityAction::EditStackData { target: ActionTarget::Attacker, slot: StackSlot::MainHand, item: held.item.clone(), edits })
}

/// ToolHarvestLogic.runBlockBreak's area half: when the tool is effective on the block being broken, its
/// definition's AOE iterator (computed now, before the break, as the original computes it) selects the
/// extra blocks and the break takes them with it. Box and circle candidates still owe the effectiveness
/// half of the default block predicate — vein and tree select by their own block rules, as their
/// getBlocks takes no predicate — and air is skipped, as the original skips it. The predicate's hardness
/// clause has no surface here (see aoe.rs). A break whose face the game did not report has no plane to
/// expand over, so it stays single-block.
fn area_break(broken: &BlockBreakContext) {
    let (Some(held), Some(face), Some(state)) = (broken.held.as_ref(), broken.face, broken.state.as_ref()) else { return; };
    let Some(world) = World::ticking() else { return; };
    let Ok(positions) = with_held_tool(held, |env, stack| {
        let Some(iterator) = env.definition.and_then(|definition| definition.aoe()) else { return Vec::new(); };
        let tier = tool_stack::harvest_tier(&stack.custom_data);
        let effective_on = |state: &pluto_lib::BlockState| env.definition.and_then(|definition| definition.effective())
            .is_some_and(|effective| tool_mining::is_effective(effective, &tier, &|tag: &Identifier| state.is_in_tag(&tag.to_string())));
        if !effective_on(state) { return Vec::new(); }
        let expanded = match stack.custom_data.get(tool_stack::TAG_VOLATILE) {
            Some(Nbt::Compound(volatile)) => crate::modifier_entries::nbt_int(volatile.get("tconstruct:expanded")),
            _ => 0,
        };
        let block_at = |pos: [i32; 3]| world.get_block_state(pos).map(|state| state.block.clone());
        let in_tag_at = |pos: [i32; 3], tag: &Identifier| world.get_block_state(pos).is_some_and(|state| state.is_in_tag(&tag.to_string()));
        let positions = iterator.positions(broken.pos, face, broken.yaw, broken.pitch, expanded, &block_at, &in_tag_at);
        if matches!(iterator, crate::aoe::Iterator::Box { .. } | crate::aoe::Iterator::Circle { .. }) {
            positions.into_iter().filter(|pos| world.get_block_state(*pos).as_ref().is_some_and(|state| state.block.to_string() != "minecraft:air" && effective_on(state))).collect()
        } else {
            positions
        }
    }) else { return; };
    for pos in positions { broken.break_extra(pos); }
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
        area_break(broken);
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
