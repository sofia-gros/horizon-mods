//! Connects PlaceFireModule's entity side (afterEntityUse) to PlutoLib's
//! use-item-on-entity event: right-clicking a creeper with an unbroken
//! Tinkers tool carrying the firestarter module ignites it, plays the flint
//! and steel sound at the creeper and wears the tool by one, as the original
//! does (SoundEvents.FLINTANDSTEEL_USE at a pitch of random * 0.4 + 0.8,
//! ToolDamageUtil.damageAnimated for 1, and sidedSuccess for the click).
//!
//! The ignite itself is the entity reach's ignite verb: the request is
//! queued and the host sets the creeper's own ignited flag at its next rider
//! synchronisation, so the fuse starts a tick later at most, and a request
//! whose target is gone, or not a creeper, by then is dropped (an accepted
//! request is a request taken, not a guarantee). The click is claimed only
//! when the request is taken; a refused ignite passes the click to the game.
//! The definition's interaction gate (InteractionToolModule.canInteract) is
//! not consulted, as nowhere in this port (see tool_definitions).
//!
//! The block side of the same module lives in tool_action_hook.rs.
use crate::{break_speed_hook::with_held_tool, tool_slot, tool_stack};
use pluto_lib::{ActionResult, EntityReach, ItemSlot, StackData, UseEntityCallback, UseItemOnEntityContext, World};

/// Finds the player's hotbar or inventory slot holding exactly the stack that was used.
fn find_slot(player: &pluto_lib::ServerPlayer, held: &StackData) -> Option<i32> {
    (0..36).find(|slot| player.stack(*slot).is_some_and(|stack| stack.item == held.item && stack.custom_data == held.custom_data && stack.damage == held.damage))
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

/// SoundEvents.FLINTANDSTEEL_USE, played by every successful place fire ignite.
const FLINT_AND_STEEL_SOUND: &str = "minecraft:item.flintandsteel.use";

/// PlaceFireModule.afterEntityUse on a creeper, with the module already
/// resolved (`fires`): the reach ignites it, the sound plays at the creeper,
/// and the tool wears by one (creative mode keeps it whole), written back to
/// the slot that held it; without a player to write back to, the wear is
/// skipped but the ignite still stands. Anything else passes the click on.
fn ignite(used: &UseItemOnEntityContext, held: &StackData, fires: bool) -> ActionResult {
    if used.kind != "minecraft:creeper" || !fires {
        return ActionResult::Pass;
    }
    let Ok(id) = u64::try_from(used.entity) else { return ActionResult::Pass };
    if !EntityReach::ignite_creeper(id) {
        return ActionResult::Pass;
    }
    if let Some(at) = EntityReach::position_of(id) {
        World::new(&used.dimension).play_sound(FLINT_AND_STEEL_SOUND, at, 1.0, 0.8 + random() * 0.4);
    }
    // ToolDamageUtil.damageAnimated for 1, through the same wear path the
    // block side uses; a tool whose slot cannot be found is left unworn
    // rather than wearing a stack the player does not hold.
    if !used.creative {
        if let Some(player) = used.player.as_ref() {
            if let Some(slot) = find_slot(player, held) {
                let worn = with_held_tool(held, |env, stack| {
                    let mut worn: ItemSlot = stack.clone();
                    let applied = tool_slot::damage_slot(&mut worn, env, 1, true, true, None, &mut random).is_ok();
                    applied.then_some(worn)
                });
                if let Ok(Some(worn)) = worn {
                    player.update_stack(slot, &StackData { item: held.item.clone(), count: worn.count, damage: worn.damage, max_damage: worn.max_damage, custom_data: worn.custom_data.clone(), custom_model_data: worn.custom_model_data.clone(), can_place_on: held.can_place_on.clone(), enchantments: held.enchantments.clone() });
                }
            }
        }
    }
    ActionResult::Success
}

static LOGGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Registers the callback. Called once from the product's hooks.
pub(crate) fn register() {
    UseEntityCallback::EVENT.register(|used| {
        if used.kind != "minecraft:creeper" {
            return ActionResult::Pass;
        }
        let Some(held) = used.held.as_ref() else { return ActionResult::Pass; };
        let (fires, broken) = match with_held_tool(held, |env, stack| (tool_stack::place_fire(&stack.custom_data, env), tool_stack::is_broken(&stack.custom_data))) {
            Ok(found) => found,
            Err(reason) => {
                if !LOGGED.swap(true, std::sync::atomic::Ordering::Relaxed) { eprintln!("Tinkers: creeper ignite click on {} left alone: {reason}", held.item); }
                return ActionResult::Pass;
            }
        };
        ignite(used, held, fires && !broken)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::{Hand, Identifier, LivingFacts, MemoryEntityOps, MemoryWorldOps, WorldEntity, with_entities, with_world};
    use std::cell::RefCell;

    fn creeper(id: u64, at: [f64; 3]) -> ([f64; 3], WorldEntity) {
        (at, WorldEntity {
            id,
            kind: Identifier::parse("minecraft:creeper").unwrap(),
            item: None,
            living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
        })
    }

    fn click(kind: &str, creative: bool) -> UseItemOnEntityContext {
        UseItemOnEntityContext {
            entity: 5, kind: kind.into(), hand: Hand::Main,
            held: Some(StackData { item: "tconstruct:pickaxe".into(), count: 1, ..Default::default() }),
            player: None, dimension: "minecraft:overworld".into(), player_dimension: "minecraft:overworld".into(), creative,
            game_mode: if creative { 1 } else { 0 },
            actions: RefCell::new(Vec::new()),
        }
    }

    #[test]
    fn a_firestarter_ignites_a_creeper_with_its_sound() {
        let mut entities = MemoryEntityOps::default();
        entities.entities.push(creeper(5, [1.0, 64.0, 2.0]));
        let mut world = MemoryWorldOps::default();
        let used = click("minecraft:creeper", false);
        let held = used.held.clone().unwrap();
        let result = with_entities(&mut entities, || with_world(&mut world, || ignite(&used, &held, true)));
        assert_eq!(result, ActionResult::Success);
        assert_eq!(entities.ignited_creepers, vec![5]);
        assert_eq!(world.sounds, vec![("minecraft:item.flintandsteel.use".to_owned(), [1.0, 64.0, 2.0])]);
    }

    #[test]
    fn other_targets_and_a_tool_without_place_fire_pass() {
        let mut entities = MemoryEntityOps::default();
        entities.entities.push(creeper(5, [1.0, 64.0, 2.0]));
        let mut world = MemoryWorldOps::default();
        let (sheep, cold) = with_entities(&mut entities, || with_world(&mut world, || {
            let sheep = click("minecraft:sheep", false);
            let sheep_held = sheep.held.clone().unwrap();
            let sheep_result = ignite(&sheep, &sheep_held, true);
            let creeper = click("minecraft:creeper", false);
            let creeper_held = creeper.held.clone().unwrap();
            let cold_result = ignite(&creeper, &creeper_held, false);
            (sheep_result, cold_result)
        }));
        assert_eq!((sheep, cold), (ActionResult::Pass, ActionResult::Pass));
        assert!(entities.ignited_creepers.is_empty(), "nothing ignited");
        assert!(world.sounds.is_empty(), "no sound without an ignite");
    }

    #[test]
    fn an_ignite_the_reach_refuses_passes_without_a_sound() {
        let mut world = MemoryWorldOps::default();
        let used = click("minecraft:creeper", false);
        let held = used.held.clone().unwrap();
        let result = with_world(&mut world, || ignite(&used, &held, true));
        assert_eq!(result, ActionResult::Pass);
        assert!(world.sounds.is_empty());
    }
}
