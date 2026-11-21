//! Connects the tool action transform modifiers (stripping, pathing, tilling) to PlutoLib's use-item-on-block event:
//! right-clicking a block with a Tinkers tool whose modifier names an action the block takes turns the block into its
//! transformed state, plays the sound and wears the tool by one, as ToolActionTransformModule does in the original.
use crate::{break_speed_hook::with_held_tool, tool_actions::{replaceable_above, ToolAction}, tool_slot, tool_stack};
use pluto_lib::{ActionResult, BlockState, Identifier, ItemSlot, StackData, UseItemOnBlockCallback, UseItemOnBlockContext, World};

/// Finds the player's hotbar or inventory slot holding exactly the stack that was used.
fn find_slot(used: &UseItemOnBlockContext, held: &StackData) -> Option<i32> {
    (0..36).find(|slot| used.player.stack(*slot).is_some_and(|stack| stack.item == held.item && stack.custom_data == held.custom_data && stack.damage == held.damage))
}

/// A cheap random float in [0, 1) for the tool damage's chance to be cancelled.
fn random() -> f32 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static STATE: AtomicU64 = AtomicU64::new(0x9E37_79B9_7F4A_7C15);
    let mut x = STATE.load(Ordering::Relaxed) ^ std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.subsec_nanos() as u64);
    x ^= x << 13; x ^= x >> 7; x ^= x << 17;
    STATE.store(x, Ordering::Relaxed);
    (x >> 40) as f32 / (1u64 << 24) as f32
}

/// The action the click performs: transforms the block and returns the sound, or None when nothing applies.
fn perform(used: &UseItemOnBlockContext, actions: &[(ToolAction, String, bool)]) -> Option<String> {
    let world = World::new(&used.dimension);
    let state = world.get_block_state(used.pos)?;
    let above_pos = [used.pos[0], used.pos[1] + 1, used.pos[2]];
    for (action, sound, require_ground) in actions {
        let above = world.get_block_state(above_pos);
        if *require_ground && !above.as_ref().is_some_and(|above| replaceable_above(&above.block)) { continue; }
        let Some(transformed) = action.transform(&state) else { continue; };
        if !world.set_block_state(used.pos, &transformed) { continue; }
        if *require_ground && above.is_some_and(|above| above.block.path() != "air") {
            world.set_block_state(above_pos, &BlockState::new(Identifier::new("minecraft", "air").expect("valid id")));
        }
        world.play_sound(sound, [f64::from(used.pos[0]) + 0.5, f64::from(used.pos[1]) + 0.5, f64::from(used.pos[2]) + 0.5], 1.0, 1.0);
        return Some(sound.clone());
    }
    None
}

static LOGGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Registers the callback. Called once from the product's hooks.
pub(crate) fn register() {
    UseItemOnBlockCallback::EVENT.register(|used| {
        let Some(held) = used.held.as_ref() else { return ActionResult::Pass; };
        let actions = match with_held_tool(held, |env, stack| tool_stack::tool_actions(&stack.custom_data, env)) {
            Ok(actions) => actions,
            Err(reason) => {
                if !LOGGED.swap(true, std::sync::atomic::Ordering::Relaxed) { eprintln!("Tinkers: tool action click on {} left alone: {reason}", held.item); }
                return ActionResult::Pass;
            }
        };
        let performed = if actions.is_empty() { None } else { perform(used, &actions) };
        if let Some(sound) = &performed { eprintln!("Tinkers: tool action {sound} at {:?}", used.pos); }
        else if !actions.is_empty() { eprintln!("Tinkers: tool action found no block to transform at {:?} ({} actions)", used.pos, actions.len()); }
        if performed.is_none() { return ActionResult::Pass; }
        // The tool wears by one (creative mode keeps it whole), written back to the slot that held it.
        if !used.creative {
            if let Some(slot) = find_slot(used, held) {
                let worn = with_held_tool(held, |env, stack| {
                    let mut worn: ItemSlot = stack.clone();
                    tool_slot::damage_slot(&mut worn, env, 1, true, true, None, &mut random).ok().map(|_| worn)
                });
                if let Ok(Some(worn)) = worn {
                    used.player.update_stack(slot, &StackData { item: held.item.clone(), count: worn.count, damage: worn.damage, max_damage: worn.max_damage, custom_data: worn.custom_data.clone(), custom_model_data: worn.custom_model_data.clone(), enchantments: held.enchantments.clone() });
                }
            }
        }
        ActionResult::Success
    });
}
