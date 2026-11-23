//! Connects the modules that hear a block the player's tool breaks to PlutoLib's block-break and loot events: mining capacity (the bar a
//! break fills or drains, before the break with `run_before` and after it otherwise: BlockHarvestModifierHook), loot to capacity (the loot of
//! the block, eaten into a bar: ProcessLootModifierHook) and the block-break half of tool usage effects (MobEffectModule.ToolUsage's
//! afterBlockBreak). The edits land as `EntityAction::EditStackData` on the main hand. An area tool's extra blocks ride the same
//! before-break callback through `break_extra` (see area_break).
use crate::{break_speed_hook::with_held_tool, common, melting_recipes::{MeltingKind, OreRate}, tool_damage_hook, tool_mining, tool_stack, weapon_effect::Usage, weapon_effect_hook::usage_actions};
use pluto_lib::{ActionTarget, BlockBreakContext, EntityAction, FluidStack, Identifier, LootTableEvents, Nbt, PlayerBlockBreakEvents, StackData, StackDataEdit, StackSlot, World};
use std::sync::Mutex;

/// Whether the tool that broke the block at (dimension, position) was effective on it, noted by the
/// before-break callback for the drops callback: at drops time the block is already air, so its state
/// cannot be read back. MeltingModule's force melting needs exactly this fact. One break at a time
/// runs on the server thread, as with loot_modifier_hook's broken-block note.
static MELT_EFFECTIVE: Mutex<Option<(String, [i32; 3], bool)>> = Mutex::new(None);

/// Notes the held tool's effectiveness on the block about to break (the same test area_break runs).
fn note_melt_effectiveness(broken: &BlockBreakContext) {
    let effective = (|| {
        let held = broken.held.as_ref()?;
        let state = broken.state.as_ref()?;
        with_held_tool(held, |env, stack| {
            let tier = tool_stack::harvest_tier(&stack.custom_data);
            env.definition.and_then(|definition| definition.effective())
                .is_some_and(|effective| tool_mining::is_effective(effective, &tier, &|tag: &Identifier| state.is_in_tag(&tag.to_string())))
        })
        .ok()
    })()
    .unwrap_or(false);
    if let Ok(mut noted) = MELT_EFFECTIVE.lock() { *noted = Some((broken.dimension.clone(), broken.pos, effective)); }
}

/// Takes the noted effectiveness for the block whose drops these are; false when nothing was noted.
fn take_melt_effective(dimension: &str, block: Option<[i32; 3]>) -> bool {
    let Ok(mut noted) = MELT_EFFECTIVE.lock() else { return false; };
    match (&*noted, block) {
        (Some((noted_dimension, pos, effective)), Some(pos_drops)) if noted_dimension == dimension && *pos == pos_drops => {
            let effective = *effective;
            *noted = None;
            effective
        }
        _ => false,
    }
}

/// MeltingModule's recipe answer for one dropped stack: the item's melting recipe, its output scaled
/// by the module's own ore rates (an ore recipe) or the foundry damage scale (a damageable one), and
/// the recipe's temperature. Byproducts are the melter's business, not the tool tank's.
fn melt_drop(slot: &pluto_lib::ItemSlot, module: &crate::tool_tank::Melting, level: f32, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Option<(FluidStack, i32)> {
    let data = common::smeltery_data()?.read().ok()?;
    let recipe = data.recipes.find_melting(&slot.item, in_tag, false)?;
    let amount = match &recipe.kind {
        MeltingKind::Ore { rate, .. } => module.ore_boost(level, *rate, recipe.output.amount),
        MeltingKind::Damageable { .. } => recipe.output_amount(&OreRate::FOUNDRY, slot.max_damage.map(|max| (slot.damage as i32, max))),
        MeltingKind::Plain => recipe.output.amount,
    };
    let fluid = recipe.output.resolve(&data.tags)?;
    let tag = recipe.output.nbt.as_ref().and_then(crate::fluid_world::value_to_nbt);
    Some((FluidStack { fluid, amount: amount.max(0) as u32, tag }, recipe.temperature))
}

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
        note_melt_effectiveness(broken);
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
        // processLoot's melting half: drops the tool's melting modules can melt become tank fluid.
        // Force melting (the volatile flag is set) counts only when the tool was effective on the
        // block, which the before-break callback noted.
        let melted = with_held_tool(tool, |env, stack| {
            let force = tool_stack::volatile_flag(&stack.custom_data, "tconstruct:force_melting") && take_melt_effective(&drops.dimension, drops.block);
            tool_stack::melting_loot(&mut stack.custom_data.clone(), env, &mut loot, force, &|slot, module, level| melt_drop(slot, module, level, &in_tag))
        });
        if let Ok(edits) = melted { drops.actions.borrow_mut().extend(edit_action(tool, edits)); }
    });
}
