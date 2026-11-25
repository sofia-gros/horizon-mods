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
//!
//! PlaceFireModule (the firestarter's flint and steel) shares the click as well: an unlit campfire,
//! candle or candle cake is lit, a TNT block is primed and cleared, and otherwise fire is placed at
//! the clicked face where fire can be placed, one durability per success. Clicking fire itself is
//! spent without wear, as the original answers success for the click that targets fire (its offset
//! target behind the fire is what the block-break veto in block_break_hook protects). One of the
//! original's paths stays unimplemented, on the record: the volatile area of effect (only the clicked
//! block is treated, the standing reduction above). The creeper ignite in afterEntityUse lives in
//! place_fire_hook.rs on the entity reach's ignite verb. The TNT prime rides
//! the world summon reach with an empty tag, the host's own fuse defaults: if the summon is refused
//! the TNT is left standing and nothing ignites, rather than the block being cleared with no primed
//! entity behind it. No game event is emitted for a lighting: the world operations name none.
use crate::{break_speed_hook::with_held_tool, tool_actions::{replaceable_above, ToolAction}, tool_slot, tool_stack};
use pluto_lib::{ActionResult, BlockState, Direction, Identifier, ItemSlot, NbtCompound, StackData, StackSlot, UseItemOnBlockCallback, UseItemOnBlockContext, World};

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

/// SoundEvents.FLINTANDSTEEL_USE, played by every successful place fire ignite.
const FLINT_AND_STEEL_SOUND: &str = "minecraft:item.flintandsteel.use";

/// The glow block's place sound: its sound type is wool (TinkerCommons), and PlaceGlowModule plays
/// the placed state's place sound.
const GLOW_PLACE_SOUND: &str = "minecraft:block.wool.place";

/// Whether the state is a lit campfire, the block ExtinguishCampfireModule.transform acts on.
fn lit_campfire(state: &BlockState) -> bool {
    matches!(state.block.path(), "campfire" | "soul_campfire") && state.block.namespace() == "minecraft" && state.property("lit") == Some("true")
}

/// Whether the state is a fire block (BlockTags.FIRE), the block PlaceFireModule targets and
/// protects: a click on it is spent, and the block-break veto keeps it standing.
fn fire_block(state: &BlockState) -> bool {
    state.block.namespace() == "minecraft" && matches!(state.block.path(), "fire" | "soul_fire")
}

/// Whether the state is a campfire, candle or candle cake PlaceFireModule.ignite can light
/// (their canLight: unlit, and a campfire not waterlogged).
fn lightable(state: &BlockState) -> bool {
    if state.block.namespace() != "minecraft" || state.property("lit") != Some("false") { return false; }
    let path = state.block.path();
    if matches!(path, "campfire" | "soul_campfire") { return state.property("waterlogged") != Some("true"); }
    path == "candle" || path == "candle_cake" || path.ends_with("_candle") || path.ends_with("_candle_cake")
}

/// PlaceFireModule.ignite at one clicked block: a lightable campfire or candle is lit, a TNT
/// block is primed through the summon reach and cleared, and otherwise fire is placed in the
/// cell on the clicked face when fire can be placed there — the cell replaceable, over a sturdy
/// face (the fire walker's read; the flammable-neighbour path of BaseFireBlock.canBePlacedAt is
/// not modelled, as there), soul fire over a soul base block. Returns whether anything ignited.
fn ignite(world: &World, pos: [i32; 3], face: [i32; 3]) -> bool {
    let Some(state) = world.get_block_state(pos) else { return false; };
    let centre = [f64::from(pos[0]) + 0.5, f64::from(pos[1]) + 0.5, f64::from(pos[2]) + 0.5];
    if lightable(&state) {
        if !world.set_block_state(pos, &state.with("lit", "true")) { return false; }
        world.play_sound(FLINT_AND_STEEL_SOUND, centre, 1.0, 0.8 + random() * 0.4);
        return true;
    }
    if state.block.to_string() == "minecraft:tnt" {
        // TntBlock.onCaughtFire primes the block where it stands; the summon reach is the port's
        // primed-TNT path, an empty tag taking the host's own fuse defaults. A refused summon
        // leaves the TNT standing and ignites nothing (see the head note).
        if world.summon("minecraft:tnt", centre, &NbtCompound::new()).is_none() { return false; }
        if !world.set_block_state(pos, &BlockState::new(Identifier::new("minecraft", "air").expect("valid id"))) { return false; }
        world.play_sound(FLINT_AND_STEEL_SOUND, centre, 1.0, 0.8 + random() * 0.4);
        return true;
    }
    let target = Direction::from_offset(face).unwrap_or(Direction::Up).relative(pos);
    let Some(cell) = world.get_block_state(target) else { return false; };
    if !replaceable_above(&cell.block) { return false; }
    let below = [target[0], target[1] - 1, target[2]];
    let Some(ground) = world.get_block_state(below) else { return false; };
    if pluto_lib::world_api_v1::face_sturdy(&ground.block.to_string(), 1) != Some(true) { return false; }
    // BaseFireBlock.getState: soul fire over a soul base block, plain fire otherwise.
    let fire = if matches!(ground.block.to_string().as_str(), "minecraft:soul_sand" | "minecraft:soul_soil") { "soul_fire" } else { "fire" };
    if !world.set_block_state(target, &BlockState::new(Identifier::new("minecraft", fire).expect("valid id"))) { return false; }
    world.play_sound(FLINT_AND_STEEL_SOUND, [f64::from(target[0]) + 0.5, f64::from(target[1]) + 0.5, f64::from(target[2]) + 0.5], 1.0, 0.8 + random() * 0.4);
    true
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
        let (actions, extinguish, glows, fires, broken) = match with_held_tool(held, |env, stack| (tool_stack::tool_actions(&stack.custom_data, env), tool_stack::extinguishes_campfires(&stack.custom_data, env), tool_stack::place_glow(&stack.custom_data, env), tool_stack::place_fire(&stack.custom_data, env), tool_stack::is_broken(&stack.custom_data))) {
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
        // PlaceFireModule.afterBlockUse: a broken tool passes, as the original's does. A click on
        // fire itself is spent without wear (the original offsets its target behind the fire and
        // answers success for targeting it); anything else goes through ignite, which wears the
        // tool by one when it lights something.
        let mut fire_worn = false;
        let mut fire_spent = false;
        if fires && !broken {
            let world = World::new(&used.dimension);
            if world.get_block_state(used.pos).is_some_and(|state| fire_block(&state)) { fire_spent = true; }
            else { fire_worn = ignite(&world, used.pos, used.face); }
        }
        if performed.is_none() && !fire_worn && !fire_spent && (broken || glows.is_empty()) { return ActionResult::Pass; }
        // The tool wears by one for a transform, by one for an ignite and by the module's damage
        // for a placed glow (creative mode keeps it whole), written back to the slot that held it.
        if !used.creative && (performed.is_some() || glow_wear.is_some() || fire_worn) {
            if let Some(slot) = find_slot(used, held) {
                let worn = with_held_tool(held, |env, stack| {
                    let mut worn: ItemSlot = stack.clone();
                    let mut applied = false;
                    let mut broke = false;
                    if performed.is_some() { let outcome = tool_slot::damage_slot(&mut worn, env, 1, true, true, None, &mut random); applied |= outcome.is_ok(); if let Ok(outcome) = outcome { broke |= outcome.broke; } }
                    if fire_worn { let outcome = tool_slot::damage_slot(&mut worn, env, 1, true, true, None, &mut random); applied |= outcome.is_ok(); if let Ok(outcome) = outcome { broke |= outcome.broke; } }
                    if let Some((amount, cause)) = &glow_wear { let outcome = tool_slot::damage_slot(&mut worn, env, *amount, true, true, Some(cause), &mut random); applied |= outcome.is_ok(); if let Ok(outcome) = outcome { broke |= outcome.broke; } }
                    applied.then_some((worn, broke))
                });
                if let Ok(Some((worn, broke))) = worn {
                    let back = StackData { item: held.item.clone(), count: worn.count, damage: worn.damage, max_damage: worn.max_damage, custom_data: worn.custom_data.clone(), custom_model_data: worn.custom_model_data.clone(), enchantments: held.enchantments.clone() };
                    // Each of the three wears announces a break it causes in the original (the
                    // transform, ignite and glow modules broadcast by hand); at most one can break
                    // the tool — the ones after it meet a broken tool and do nothing — so one
                    // announcement covers the click. The use names no hand, so the break is
                    // announced only when the found slot is the selected one, the main hand's place.
                    if used.player.update_stack(slot, &back) && broke && slot == used.player.selected_slot().unwrap_or(-1) {
                        used.player.broadcast_item_break(StackSlot::MainHand, &back);
                    }
                }
            }
        }
        ActionResult::Success
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::{with_world, MemoryWorldOps};

    fn state(text: &str) -> BlockState { BlockState::parse(text).unwrap() }

    #[test]
    fn lightable_and_fire_blocks_name_the_states_place_fire_reads() {
        assert!(lightable(&state("minecraft:campfire[lit=false]")));
        assert!(lightable(&state("minecraft:soul_campfire[lit=false,waterlogged=false]")));
        assert!(!lightable(&state("minecraft:campfire[lit=true]")), "a lit campfire is the extinguish module's block");
        assert!(!lightable(&state("minecraft:campfire[lit=false,waterlogged=true]")), "a waterlogged campfire cannot light");
        assert!(lightable(&state("minecraft:candle[lit=false]")));
        assert!(lightable(&state("minecraft:white_candle_cake[lit=false]")));
        assert!(!lightable(&state("minecraft:candle[lit=true]")));
        assert!(!lightable(&state("minecraft:stone")));
        assert!(fire_block(&state("minecraft:fire")));
        assert!(fire_block(&state("minecraft:soul_fire")));
        assert!(!fire_block(&state("minecraft:campfire[lit=true]")));
    }

    #[test]
    fn ignite_lights_an_unlit_campfire_and_primes_tnt() {
        let mut ops = MemoryWorldOps::default();
        let world = World::new("minecraft:overworld");
        let (lit, primed) = with_world(&mut ops, || {
            world.set_block_state([0, 64, 0], &state("minecraft:campfire[lit=false]"));
            let lit = ignite(&world, [0, 64, 0], [0, 1, 0]);
            assert_eq!(world.get_block_state([0, 64, 0]).unwrap().property("lit"), Some("true"), "the campfire state was written lit");
            world.set_block_state([5, 64, 0], &state("minecraft:tnt"));
            let primed = ignite(&world, [5, 64, 0], [0, 1, 0]);
            assert_eq!(world.get_block_state([5, 64, 0]).unwrap().block.to_string(), "minecraft:air", "the primed block was cleared");
            (lit, primed)
        });
        assert!(lit && primed);
        assert_eq!(ops.summons.len(), 1, "the TNT was primed through the summon reach");
        assert_eq!(ops.summons[0].kind, "minecraft:tnt");
    }

    #[test]
    fn ignite_places_fire_over_a_sturdy_face_and_soul_fire_over_soul_soil() {
        // The sturdy read is a host-published table this test stands in for: stone and soul soil
        // answer sturdy, everything else is unanswered, the uninstalled default for other blocks.
        pluto_lib::world_api_v1::set_sturdy_lookup(|block, _| (block == "minecraft:stone" || block == "minecraft:soul_soil").then_some(true));
        let mut ops = MemoryWorldOps::default();
        let world = World::new("minecraft:overworld");
        let placed = with_world(&mut ops, || {
            world.set_block_state([0, 64, 0], &state("minecraft:stone"));
            world.set_block_state([0, 65, 0], &state("minecraft:air"));
            let plain = ignite(&world, [0, 64, 0], [0, 1, 0]);
            let plain_block = world.get_block_state([0, 65, 0]).map(|placed| placed.block.to_string());
            world.set_block_state([9, 64, 0], &state("minecraft:soul_soil"));
            world.set_block_state([9, 65, 0], &state("minecraft:air"));
            let soul = ignite(&world, [9, 64, 0], [0, 1, 0]);
            let soul_block = world.get_block_state([9, 65, 0]).map(|placed| placed.block.to_string());
            (plain, plain_block, soul, soul_block)
        });
        assert_eq!(placed, (true, Some("minecraft:fire".to_owned()), true, Some("minecraft:soul_fire".to_owned())));
    }
}
