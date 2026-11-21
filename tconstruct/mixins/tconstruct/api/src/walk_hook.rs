//! Connects the boot walk transform modules (ToolActionWalkerTransformModule, SlimeKnights, MIT; see LICENSE)
//! to PlutoLib's player tick: when a player wearing such boots moves into a new block, the ground under each
//! replaceable cell around them takes its transformed state, with the module's sound and one point of wear
//! per cell, as ArmorWalkRadiusModule.onWalk and the module's walkOn do in the original.
//!
//! The original's other walk modules stay unbound, each for a named gap at the walk seam: fire walker
//! (BaseFireBlock.canBePlacedAt) and cover ground (BlockState.canSurvive) both turn on whether the face of
//! the block below is sturdy or full, a fact only the block callbacks' level view answers — the player tick
//! seam is handed the mod host, whose queries carry no such call; glow walker places the `tconstruct:glow`
//! block, which this port never registered as a block. Two further differences from the original, both
//! inherited from this port's surfaces: the cell above the ground is cleared without dropping it (the world
//! operations have no destroy-with-drops call; the original destroys it with drops), and whether a cell can
//! be replaced is answered by the port's replaceable table, the same one the click transform's ground check
//! uses, which names air and the common plants but not fluids.
use crate::{break_speed_hook::with_held_tool, tool_actions::replaceable_above, tool_slot, tool_stack};
use pluto_lib::{ActionTarget, BlockState, EntityAction, Identifier, ItemSlot, ServerLivingEntityEvents, StackSlot, World};
use std::collections::BTreeMap;
use std::sync::Mutex;

/// The block each player's feet were last seen in, by uuid: the original hears a walk when the position changes.
static PREVIOUS: Mutex<BTreeMap<u128, [i32; 3]>> = Mutex::new(BTreeMap::new());

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
}
