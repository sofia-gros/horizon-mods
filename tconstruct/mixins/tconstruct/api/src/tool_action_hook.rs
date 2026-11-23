//! Connects the tool action transform modifiers (stripping, pathing, tilling) to PlutoLib's use-item-on-block event:
//! right-clicking a block with a Tinkers tool whose modifier names an action the block takes turns the block into its
//! transformed state, plays the sound and wears the tool by one, as ToolActionTransformModule does in the original.
//! The campfire extinguish module shares the click: a lit campfire the tool is used on goes out, with the extinguish
//! sound (ExtinguishCampfireModule.transform; its dowse particles have no world operation here). Like the transforms,
//! only the clicked block is treated, not the tool's AOE neighbours the original's BlockTransformModule also runs.
//!
//! PlaceGlowModule shares the click too: an unbroken tool carrying it places a glow block on the clicked face
//! (GlowBlock.addGlow, through the same placement the glow ball uses, so a glow that cannot survive where it was
//! put removes itself a tick later) and the click is consumed whether or not a glow could be placed
//! (afterBlockUse returns sidedSuccess); the tool wears by the module's damage only when a glow was placed, and
//! hears the placed state's place sound, the glow block's wool sound. The definition's interaction gate
//! (InteractionToolModule.canInteract) is not consulted, as nowhere in this port (see tool_definitions).
use crate::{break_speed_hook::with_held_tool, tool_actions::{replaceable_above, ToolAction}, tool_slot, tool_stack};
use pluto_lib::{ActionResult, BlockState, Direction, Identifier, ItemSlot, StackData, UseItemOnBlockCallback, UseItemOnBlockContext, World};

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

/// The sound of a campfire going out (SoundEvents.GENERIC_EXTINGUISH_FIRE).
const EXTINGUISH_SOUND: &str = "minecraft:entity.generic.extinguish_fire";

/// The glow block's place sound: its sound type is wool (TinkerCommons), and PlaceGlowModule plays
/// the placed state's place sound.
const GLOW_PLACE_SOUND: &str = "minecraft:block.wool.place";

/// Whether the state is a lit campfire, the block ExtinguishCampfireModule.transform acts on.
fn lit_campfire(state: &BlockState) -> bool {
    matches!(state.block.path(), "campfire" | "soul_campfire") && state.block.namespace() == "minecraft" && state.property("lit") == Some("true")
}

/// The action the click performs: transforms the block and returns the sound, or None when nothing applies.
fn perform(used: &UseItemOnBlockContext, actions: &[(ToolAction, String, bool)], extinguish: bool) -> Option<String> {
    let world = World::new(&used.dimension);
    let state = world.get_block_state(used.pos)?;
    if extinguish && lit_campfire(&state) {
        if world.set_block_state(used.pos, &state.clone().with("lit", "false")) {
            world.play_sound(EXTINGUISH_SOUND, [f64::from(used.pos[0]) + 0.5, f64::from(used.pos[1]) + 0.5, f64::from(used.pos[2]) + 0.5], 1.0, 1.0);
            return Some(EXTINGUISH_SOUND.to_owned());
        }
    }
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
        let (actions, extinguish, glows, broken) = match with_held_tool(held, |env, stack| (tool_stack::tool_actions(&stack.custom_data, env), tool_stack::extinguishes_campfires(&stack.custom_data, env), tool_stack::place_glow(&stack.custom_data, env), tool_stack::is_broken(&stack.custom_data))) {
            Ok(found) => found,
            Err(reason) => {
                if !LOGGED.swap(true, std::sync::atomic::Ordering::Relaxed) { eprintln!("Tinkers: tool action click on {} left alone: {reason}", held.item); }
                return ActionResult::Pass;
            }
        };
        let performed = if actions.is_empty() && !extinguish { None } else { perform(used, &actions, extinguish) };
        if let Some(sound) = &performed { eprintln!("Tinkers: tool action {sound} at {:?}", used.pos); }
        else if !actions.is_empty() { eprintln!("Tinkers: tool action found no block to transform at {:?} ({} actions)", used.pos, actions.len()); }
        // PlaceGlowModule.afterBlockUse: the glow goes in the cell on the clicked face's side, facing
        // back toward the clicked block (the glow ball's block-hit target), and the click is consumed
        // whether or not one could be placed; only a placement wears the tool or makes a sound.
        let mut glow_wear: Option<(i32, Identifier)> = None;
        if !broken && !glows.is_empty() {
            let world = World::new(&used.dimension);
            let face = Direction::from_offset(used.face).unwrap_or(Direction::Up);
            let target = face.relative(used.pos);
            let is_glow = |world: &World| world.get_block_state(target).is_some_and(|state| state.block.to_string() == "tconstruct:glow");
            if !is_glow(&world) {
                crate::throwables::place_glow(&world, target, face.opposite());
                if is_glow(&world) {
                    world.play_sound(GLOW_PLACE_SOUND, [f64::from(target[0]) + 0.5, f64::from(target[1]) + 0.5, f64::from(target[2]) + 0.5], 1.0, 1.0);
                    let (modifier, damage) = &glows[0];
                    if *damage > 0 { glow_wear = Some((*damage, modifier.clone())); }
                }
            }
        }
        if performed.is_none() && (broken || glows.is_empty()) { return ActionResult::Pass; }
        // The tool wears by one for a transform and by the module's damage for a placed glow
        // (creative mode keeps it whole), written back to the slot that held it.
        if !used.creative && (performed.is_some() || glow_wear.is_some()) {
            if let Some(slot) = find_slot(used, held) {
                let worn = with_held_tool(held, |env, stack| {
                    let mut worn: ItemSlot = stack.clone();
                    let mut applied = false;
                    if performed.is_some() { applied |= tool_slot::damage_slot(&mut worn, env, 1, true, true, None, &mut random).is_ok(); }
                    if let Some((amount, cause)) = &glow_wear { applied |= tool_slot::damage_slot(&mut worn, env, *amount, true, true, Some(cause), &mut random).is_ok(); }
                    applied.then_some(worn)
                });
                if let Ok(Some(worn)) = worn {
                    used.player.update_stack(slot, &StackData { item: held.item.clone(), count: worn.count, damage: worn.damage, max_damage: worn.max_damage, custom_data: worn.custom_data.clone(), custom_model_data: worn.custom_model_data.clone(), enchantments: held.enchantments.clone() });
                }
            }
        }
        ActionResult::Success
    });
}
