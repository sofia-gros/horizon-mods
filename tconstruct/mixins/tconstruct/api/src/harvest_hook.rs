//! HarvestModule (SlimeKnights, MIT; see LICENSE): right-clicking a harvestable block with a
//! tool that carries the harvest module harvests it in place — berry bushes and cave vines by
//! re-dispatching the block's own use, sugar cane and kelp by breaking the segment the stalk
//! grows from, and full-grown crops by breaking them and replanting — over the clicked block
//! and the tool's area iterator, wearing the tool by one for each block harvested.
//!
//! Reductions against the original, by construction: the breaks, the replant and the
//! re-dispatched use are queued for the level's tick seam (PlutoLib's block actions, a tick
//! late at most), so a re-dispatched use counts as harvested when the level accepts it rather
//! than when it consumes the action, as the original knows at once. A crop's drops are the
//! level's own break drops, rolled without the tool (its fortune does not reach them), the
//! seed a replant costs the original is not taken from the drops, and the replant always
//! lands; a crop whose oldest age the table below does not name is left standing, as the
//! state's age maximum cannot be read from the state a use callback is handed. Sneaking is
//! not reported by the use telling, so a sneaking click harvests too, where the original
//! passes. The area pass runs the definition's break iterator from the click face (the
//! telling reports no look angles), not the original's transform match. The plant harvest
//! hooks of other modifiers (enderporting's plant form) do not run: no callback here can
//! move the holder.
use crate::{break_speed_hook::with_held_tool, tool_damage_hook, tool_slot, tool_stack};
use pluto_lib::{ActionResult, Direction, Identifier, ItemSlot, Nbt, StackData, UseItemOnBlockCallback, UseItemOnBlockContext, World};

/// The block tag of everything the module harvests (TinkerTags.Blocks.HARVESTABLE).
const HARVESTABLE: &str = "tconstruct:harvestable";

/// The oldest age a shipped crop is harvested at (its age property's maximum): the vanilla
/// members of the crops tag. A crop the table does not name is left standing.
fn crop_max_age(block: &Identifier) -> Option<i32> {
    if block.namespace() != "minecraft" { return None; }
    match block.path() {
        "wheat" | "carrots" | "potatoes" => Some(7),
        "beetroots" | "torchflower_crop" | "nether_wart" => Some(3),
        "pitcher_crop" => Some(4),
        _ => None,
    }
}

/// Harvests the block at `pos` the way its kind asks; true when a harvest was dispatched.
fn harvest(world: &World, held_item: &str, pos: [i32; 3], face: [i32; 3]) -> bool {
    let Some(state) = world.get_block_state(pos) else { return false; };
    if !state.is_in_tag(HARVESTABLE) { return false; }
    // Crops that work on right click (berry bushes, cave vines): the block's own use, re-dispatched
    // with the tool in hand, at the centre of the clicked face (the telling reports no click point).
    if state.is_in_tag("tconstruct:harvestable/interact") {
        let hit = [f64::from(pos[0]) + 0.5 + f64::from(face[0]) * 0.5, f64::from(pos[1]) + 0.5 + f64::from(face[1]) * 0.5, f64::from(pos[2]) + 0.5 + f64::from(face[2]) * 0.5];
        return world.use_block(pos, face, hit, Some(held_item));
    }
    // Sugar cane like blocks: break this segment when the one below is the same block, else the
    // segment above when that one is.
    if state.is_in_tag("tconstruct:harvestable/stackable") {
        let below = [pos[0], pos[1] - 1, pos[2]];
        if world.get_block_state(below).is_some_and(|below| below.block == state.block) { return world.break_block(pos, true); }
        let above = [pos[0], pos[1] + 1, pos[2]];
        if world.get_block_state(above).is_some_and(|above| above.block == state.block) { return world.break_block(above, true); }
        return false;
    }
    // Normal crops: at their oldest age they break (the level's own drops) and replant at age 0,
    // the replant queued behind the break so it lands on the emptied cell.
    if state.is_in_tag("tconstruct:harvestable/crops") {
        let Some(max) = crop_max_age(&state.block) else { return false; };
        if state.property("age").and_then(|age| age.parse::<i32>().ok()) != Some(max) { return false; }
        if !world.break_block(pos, true) { return false; }
        world.set_block_state_queued(pos, &state.clone().with("age", "0"));
        return true;
    }
    false
}

/// SmeltingModule.afterHarvest: the tool cooks by the blocks this click harvested, written back
/// to the slot that holds it (found by item and data, as the wear may have changed its damage).
fn cook(used: &UseItemOnBlockContext, held: &StackData, amount: i32) {
    if amount <= 0 { return; }
    let slot = (0..36).find(|slot| used.player.stack(*slot).is_some_and(|stack| stack.item == held.item && stack.custom_data == held.custom_data));
    let Some(slot) = slot else { return; };
    let Some(mut stack) = used.player.stack(slot) else { return; };
    let cooked = with_held_tool(&stack, |env, tool| {
        let mut data = tool.custom_data.clone();
        tool_stack::cook_smelting(&mut data, env, amount as f32);
        data
    });
    if let Ok(data) = cooked {
        stack.custom_data = data;
        used.player.update_stack(slot, &stack);
    }
}

/// Wears the used tool by `amount`, written back to the slot that held it (as the tool action
/// transforms wear theirs).
fn wear(used: &UseItemOnBlockContext, held: &StackData, amount: i32) {
    let slot = (0..36).find(|slot| used.player.stack(*slot).is_some_and(|stack| stack.item == held.item && stack.custom_data == held.custom_data && stack.damage == held.damage));
    let Some(slot) = slot else { return; };
    let worn = with_held_tool(held, |env, stack| {
        let mut worn: ItemSlot = stack.clone();
        tool_slot::damage_slot(&mut worn, env, amount, true, true, None, &mut tool_damage_hook::random).ok().map(|_| worn)
    });
    if let Ok(Some(worn)) = worn {
        used.player.update_stack(slot, &StackData { item: held.item.clone(), count: worn.count, damage: worn.damage, max_damage: worn.max_damage, custom_data: worn.custom_data.clone(), custom_model_data: worn.custom_model_data.clone(), can_place_on: held.can_place_on.clone(), enchantments: held.enchantments.clone() });
    }
}

/// Registers the callback. Called once from the product's hooks.
pub(crate) fn register() {
    UseItemOnBlockCallback::EVENT.register(|used| {
        let Some(held) = used.held.as_ref() else { return ActionResult::Pass; };
        if !matches!(with_held_tool(held, |env, stack| tool_stack::harvests(&stack.custom_data, env)), Ok(true)) { return ActionResult::Pass; }
        let world = World::new(&used.dimension);
        let Some(state) = world.get_block_state(used.pos) else { return ActionResult::Pass; };
        if !state.is_in_tag(HARVESTABLE) { return ActionResult::Pass; }
        let mut harvested = 0;
        if harvest(&world, &held.item, used.pos, used.face) { harvested += 1; }
        // The area the tool covers, from its definition's iterator, as the area break takes it.
        let positions = with_held_tool(held, |env, stack| {
            let Some(iterator) = env.definition.and_then(|definition| definition.aoe()) else { return Vec::new(); };
            let expanded = match stack.custom_data.get(tool_stack::TAG_VOLATILE) {
                Some(Nbt::Compound(volatile)) => crate::modifier_entries::nbt_int(volatile.get("tconstruct:expanded")),
                _ => 0,
            };
            let face = Direction::from_offset(used.face).unwrap_or(Direction::Up);
            let block_at = |pos: [i32; 3]| world.get_block_state(pos).map(|state| state.block.clone());
            let in_tag_at = |pos: [i32; 3], tag: &Identifier| world.get_block_state(pos).is_some_and(|state| state.is_in_tag(&tag.to_string()));
            iterator.positions(used.pos, face, 0.0, 0.0, expanded, &block_at, &in_tag_at)
        }).unwrap_or_default();
        for pos in positions {
            if pos != used.pos && harvest(&world, &held.item, pos, used.face) { harvested += 1; }
        }
        if harvested > 0 {
            // EnderportingModifier.afterHarvest: a harvest with an enderporting tool pulls the
            // holder onto the clicked block's centre (queued for their next tick, as the use
            // telling carries no entity id), and wears the tool by 2. The two wears are one call:
            // the slot lookup keys on the pre-wear stack, so a second wear with the same held
            // would not find the slot after the first.
            let teleported = crate::enderporting_hook::plant_harvest(used, held, harvested);
            if !used.creative { wear(used, held, harvested + if teleported { 2 } else { 0 }); }
            cook(used, held, harvested);
        }
        // The click is consumed whenever the clicked block is harvestable, harvested or not.
        ActionResult::Success
    });
}
