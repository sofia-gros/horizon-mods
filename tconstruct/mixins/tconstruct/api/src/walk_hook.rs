//! Connects the boot walk transform modules (ToolActionWalkerTransformModule, SlimeKnights, MIT; see LICENSE)
//! to PlutoLib's player tick: when a player wearing such boots moves into a new block, the ground under each
//! replaceable cell around them takes its transformed state, with the module's sound and one point of wear
//! per cell, as ArmorWalkRadiusModule.onWalk and the module's walkOn do in the original.
//!
//! The original's other walk modules bind over two PlutoLib reaches: fire walker
//! (BaseFireBlock.canBePlacedAt) and cover ground (BlockState.canSurvive) both turn on whether the face of
//! the block below is sturdy or full, which the block facts PlutoLib publishes from the game's own table
//! now answer by block name, and glow walker's per-cell brightness is answered by the world light read.
//! Reductions against the original: the sturdy fact answers by block name (a block's first state, as the
//! dig facts do), fire's flammable-neighbour placement path is not modelled (a sturdy face below, plus
//! soul fire over soul sand or soil as BaseFireBlock.getState picks it), glow's survival is read as the
//! sturdy face below and its place sound is the glow block's default (stone) place sound, and the three
//! forms run as their own tick hearing after the transform's, where the original's merger runs all walk
//! modules in one order. Two further differences from the original, both
//! inherited from this port's surfaces: the cell above the ground is cleared without dropping it (the world
//! operations have no destroy-with-drops call; the original destroys it with drops), and whether a cell can
//! be replaced is answered by the port's replaceable table, the same one the click transform's ground check
//! uses, which names air and the common plants but not fluids.
//!
//! ReplaceBlockWalkerModule rides the same tick as the fire, glow and cover forms: its target
//! predicate is tested against the block below an empty cell through the same world read, which
//! carries the state's properties, so still water (level 0) freezes and flowing water does not.
//! Reductions against the original: the replacement state's canSurvive and isUnobstructed checks
//! and the Forge onBlockPlace event are not modelled (placement follows the target match alone),
//! and the module's own scheduleTick for the melt is not repeated — the host's frosted ice
//! schedules the same 60–120 tick itself when a state is placed, as the vanilla block's onPlace
//! does, so the ice still melts on schedule. The original charges the boots no wear for a
//! replacement, and neither does this form.
//!
//! LightspeedAttributeModule rides the same tick: the boots' attribute is granted while the light at the
//! wearer (which the tick reports) is above the module's minimum, and the boots take wear by the module's
//! chance as the wearer walks into a new block, as the original's onWalk does both. The original grants the
//! attribute from onWalk, so it lands on the first step; the tick grants it from the first tick, the same
//! difference the max armor attributes have.
use crate::{break_speed_hook::with_held_tool, modifier_modules::WalkModule, tool_actions::replaceable_above, tool_slot, tool_stack};
use pluto_lib::{ActionTarget, BlockState, EntityAction, Identifier, ItemSlot, ServerLivingEntityEvents, StackSlot, World};
use std::collections::BTreeMap;
use std::sync::Mutex;

/// The block each player's feet were last seen in, by uuid: the original hears a walk when the position changes.
static PREVIOUS: Mutex<BTreeMap<u128, [i32; 3]>> = Mutex::new(BTreeMap::new());

/// The same, for the lightspeed wear roll, which is its own walk hearing.
static PREVIOUS_STEP: Mutex<BTreeMap<u128, [i32; 3]>> = Mutex::new(BTreeMap::new());

/// The same, for the fire, glow and cover walks.
static PREVIOUS_WALK: Mutex<BTreeMap<u128, [i32; 3]>> = Mutex::new(BTreeMap::new());

fn air() -> BlockState {
    BlockState::new(Identifier::new("minecraft", "air").expect("fixed block id"))
}

/// Registers the callback. Called once from the product's hooks.
pub(crate) fn register() {
    ServerLivingEntityEvents::PLAYER_TICK.register(|tick| {
        let Some((_, feet)) = tick.armor.iter().find(|(slot, _)| slot == "feet") else { return };
        let block_pos = [tick.position[0].floor() as i32, tick.position[1].floor() as i32, tick.position[2].floor() as i32];
        let previous = PREVIOUS.lock().map(|mut map| map.insert(tick.player, block_pos)).unwrap_or(None);
        // ArmorWalkModifierHook hears a walk on a position change; onWalk itself then asks for the ground.
        if previous == Some(block_pos) || !tick.on_ground { return; }
        let world = World::new(&tick.dimension);
        let walked = with_held_tool(feet, |env, stack| {
            let mut slot: ItemSlot = stack.clone();
            let transforms = tool_stack::walk_transforms(&slot.custom_data, env);
            if transforms.is_empty() || tool_stack::is_broken(&slot.custom_data) { return None; }
            let before = slot.custom_data.clone();
            let center = [tick.position[0].floor() as i32, (tick.position[1] + 0.5).floor() as i32, tick.position[2].floor() as i32];
            'modules: for (cause, action, sound, radius) in &transforms {
                let true_radius = radius.min(16.0);
                let reach = true_radius.floor() as i32;
                for x in center[0] - reach..=center[0] + reach {
                    for z in center[2] - reach..=center[2] + reach {
                        let target = [x, center[1], z];
                        // BlockPos.closerToCenterThan: the cell's centre within the radius of the walker's feet.
                        let (dx, dy, dz) = (f64::from(x) + 0.5 - tick.position[0], f64::from(center[1]) + 0.5 - tick.position[1], f64::from(z) + 0.5 - tick.position[2]);
                        if dx * dx + dy * dy + dz * dz >= f64::from(true_radius) * f64::from(true_radius) { continue; }
                        let Some(cell) = world.get_block_state(target) else { continue };
                        if !replaceable_above(&cell.block) { continue; }
                        let below = [x, center[1] - 1, z];
                        let Some(ground) = world.get_block_state(below) else { continue };
                        let Some(transformed) = action.transform(&ground) else { continue };
                        if !world.set_block_state(below, &transformed) { continue; }
                        world.set_block_state(target, &air());
                        world.play_sound(sound, [f64::from(below[0]) + 0.5, f64::from(below[1]) + 0.5, f64::from(below[2]) + 0.5], 1.0, 1.0);
                        let _ = tool_slot::damage_slot(&mut slot, env, 1, true, true, Some(cause), &mut crate::tool_damage_hook::random);
                        // walkOn answers whether the tool is broken, which stops the iteration (and later modules).
                        if tool_stack::is_broken(&slot.custom_data) { break 'modules; }
                    }
                }
            }
            let edits = crate::wearer_modules::edits_between(&before, &slot.custom_data);
            (!edits.is_empty()).then_some(edits)
        });
        if let Ok(Some(edits)) = walked {
            tick.actions.borrow_mut().push(EntityAction::EditStackData { target: ActionTarget::Attacker, slot: StackSlot::Feet, item: feet.item.clone(), edits });
        }
    });
    // FireWalkerModule, GlowWalkerModule, CoverGroundWalkerModule and ReplaceBlockWalkerModule:
    // the radius walk over the sturdy-face fact and the world light read (see the file's head
    // note for the reductions).
    ServerLivingEntityEvents::PLAYER_TICK.register(|tick| {
        let Some((_, feet)) = tick.armor.iter().find(|(slot, _)| slot == "feet") else { return };
        let block_pos = [tick.position[0].floor() as i32, tick.position[1].floor() as i32, tick.position[2].floor() as i32];
        let previous = PREVIOUS_WALK.lock().map(|mut map| map.insert(tick.player, block_pos)).unwrap_or(None);
        if previous == Some(block_pos) || !tick.on_ground { return; }
        let world = World::new(&tick.dimension);
        let walked = with_held_tool(feet, |env, stack| {
            let mut slot: ItemSlot = stack.clone();
            let walkers = tool_stack::walk_modules(&slot.custom_data, env);
            if walkers.is_empty() || tool_stack::is_broken(&slot.custom_data) { return None; }
            let before = slot.custom_data.clone();
            let center = [tick.position[0].floor() as i32, (tick.position[1] + 0.5).floor() as i32, tick.position[2].floor() as i32];
            'modules: for (cause, walk) in &walkers {
                let radius = match walk { WalkModule::Fire { radius } | WalkModule::Glow { radius, .. } | WalkModule::Cover { radius, .. } | WalkModule::Replace { radius, .. } => *radius };
                let true_radius = radius.min(16.0);
                let reach = true_radius.floor() as i32;
                'cells: for x in center[0] - reach..=center[0] + reach {
                    for z in center[2] - reach..=center[2] + reach {
                        let target = [x, center[1], z];
                        let (dx, dy, dz) = (f64::from(x) + 0.5 - tick.position[0], f64::from(center[1]) + 0.5 - tick.position[1], f64::from(z) + 0.5 - tick.position[2]);
                        if dx * dx + dy * dy + dz * dz >= f64::from(true_radius) * f64::from(true_radius) { continue; }
                        let below = [x, center[1] - 1, z];
                        match walk {
                            WalkModule::Fire { .. } => {
                                let Some(cell) = world.get_block_state(target) else { continue };
                                if !replaceable_above(&cell.block) { continue; }
                                let Some(ground) = world.get_block_state(below) else { continue };
                                if pluto_lib::world_api_v1::face_sturdy(&ground.block.to_string(), 1) != Some(true) { continue; }
                                let fire = if matches!(ground.block.to_string().as_str(), "minecraft:soul_sand" | "minecraft:soul_soil") { "minecraft:soul_fire" } else { "minecraft:fire" };
                                let Ok(fire) = Identifier::parse(fire) else { continue };
                                if !world.set_block_state(target, &BlockState::new(fire)) { continue; }
                                let pitch = 0.8 + world.random_float().unwrap_or(0.0) * 0.4;
                                world.play_sound("minecraft:item.flintandsteel.use", [f64::from(target[0]) + 0.5, f64::from(target[1]) + 0.5, f64::from(target[2]) + 0.5], 1.0, pitch);
                                let _ = tool_slot::damage_slot(&mut slot, env, 1, true, true, Some(cause), &mut crate::tool_damage_hook::random);
                                if tool_stack::is_broken(&slot.custom_data) { break 'modules; }
                            }
                            WalkModule::Glow { min_light, damage, .. } => {
                                let Some(cell) = world.get_block_state(target) else { continue };
                                if !matches!(cell.block.to_string().as_str(), "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air") { continue; }
                                let Some((block_light, _)) = world.light(target) else { continue };
                                if i32::from(block_light) >= *min_light { continue; }
                                let Some(ground) = world.get_block_state(below) else { continue };
                                if pluto_lib::world_api_v1::face_sturdy(&ground.block.to_string(), 1) != Some(true) { continue; }
                                crate::throwables::place_glow(&world, target, pluto_lib::Direction::Down);
                                if world.get_block_state(target).is_some_and(|state| state.block.to_string() == "tconstruct:glow") {
                                    world.play_sound("minecraft:block.stone.place", [f64::from(target[0]) + 0.5, f64::from(target[1]) + 0.5, f64::from(target[2]) + 0.5], 1.0, 1.0);
                                    let _ = tool_slot::damage_slot(&mut slot, env, *damage, true, true, Some(cause), &mut crate::tool_damage_hook::random);
                                    // A single success per walk, as the original stops there to let the lighting settle.
                                    break 'cells;
                                }
                            }
                            WalkModule::Cover { block, .. } => {
                                let Some(cell) = world.get_block_state(target) else { continue };
                                if !matches!(cell.block.to_string().as_str(), "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air") { continue; }
                                let Some(ground) = world.get_block_state(below) else { continue };
                                if pluto_lib::world_api_v1::face_sturdy(&ground.block.to_string(), 1) != Some(true) { continue; }
                                world.set_block_state(target, &BlockState::new(block.clone()));
                            }
                            WalkModule::Replace { replacements, .. } => {
                                let Some(cell) = world.get_block_state(target) else { continue };
                                if !matches!(cell.block.to_string().as_str(), "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air") { continue; }
                                let Some(ground) = world.get_block_state(below) else { continue };
                                // ReplaceBlockWalkerModule.walkOn: the first replacement whose target
                                // matches the block below the empty cell takes that block's place.
                                for replacement in replacements {
                                    if replacement.target.matches(&ground) {
                                        world.set_block_state(below, &replacement.state);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            let edits = crate::wearer_modules::edits_between(&before, &slot.custom_data);
            (!edits.is_empty()).then_some(edits)
        });
        if let Ok(Some(edits)) = walked {
            tick.actions.borrow_mut().push(EntityAction::EditStackData { target: ActionTarget::Attacker, slot: StackSlot::Feet, item: feet.item.clone(), edits });
        }
    });
    // LightspeedAttributeModule.onWalk: while the light at the wearer is above the module's minimum the boots
    // grant their attribute, and walking rolls the module's wear chance against the excess light.
    ServerLivingEntityEvents::PLAYER_TICK.register(|tick| {
        let Some((_, feet)) = tick.armor.iter().find(|(slot, _)| slot == "feet") else { return };
        let block_pos = [tick.position[0].floor() as i32, tick.position[1].floor() as i32, tick.position[2].floor() as i32];
        let previous = PREVIOUS_STEP.lock().map(|mut map| map.insert(tick.player, block_pos)).unwrap_or(None);
        let walked = previous != Some(block_pos) && tick.on_ground;
        let world = World::new(&tick.dimension);
        let done = with_held_tool(feet, |env, stack| {
            let modules = tool_stack::lightspeed(&stack.custom_data, env);
            if modules.is_empty() || tool_stack::is_broken(&stack.custom_data) { return None; }
            let mut slot: ItemSlot = stack.clone();
            let before = slot.custom_data.clone();
            let mut additions = Vec::new();
            for (modifier, module, level) in &modules {
                let light = module.light(tick.block_light, tick.sky_light, tick.sky_darken);
                if let Some(boost) = module.boost(light, *level) { additions.push((module.attribute().to_owned(), f64::from(boost))); }
                if walked && world.random_float().is_some_and(|roll| roll < module.wear_chance(light)) {
                    let _ = tool_slot::damage_slot(&mut slot, env, 1, true, true, Some(modifier), &mut crate::tool_damage_hook::random);
                }
            }
            let edits = crate::wearer_modules::edits_between(&before, &slot.custom_data);
            Some((additions, edits))
        });
        if let Ok(Some((additions, edits))) = done {
            tick.attribute_additions.borrow_mut().extend(additions);
            if !edits.is_empty() {
                tick.actions.borrow_mut().push(EntityAction::EditStackData { target: ActionTarget::Attacker, slot: StackSlot::Feet, item: feet.item.clone(), edits });
            }
        }
    });
}
