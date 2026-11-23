//! EnderportingModifier (SlimeKnights, MIT; see LICENSE): the holder trades places with what the
//! tool touches. A harvested block pulls the holder onto it and a melee hit swaps the attacker and
//! the mob it landed on; the projectile half rides the flight profile (flight_hook's impact pass).
//! Every move the callbacks can test goes through the original's landing test as the ender slime's
//! teleport states it (slime_entities::teleport_lands: the blocks the body would occupy are free of
//! solids and liquids); the melee swap needs no test, as its destinations are the two parties' own
//! standing places. A landed harvest or swap wears the tool by 2, as the original calls
//! ToolDamageUtil.damageTool with 2 there.
//!
//! The harvest half is finishHarvest and the plant harvest in one: an area tool's extra blocks
//! ride the break without events of their own, so the centre block's after-break is the one that
//! is heard, which is where both of the original's hooks put the holder. Its `canHarvest`
//! condition is the break itself — the after-break callback only runs for a block the tool did
//! harvest. Not wired: the launch wear (the shoot telling does not tell a multishot's primary
//! arrow from its extras, and the original charges only the primary), the fuse teleport (no flight
//! action moves the owner), the fishing rod's block-hit form (no fishing telling names the tool),
//! and the monster-held form (a monster's melee hit reaches no weapon-hit telling here; the
//! wearer-hit telling hears the armor's modules, not the mob's weapon).
use crate::{break_speed_hook::with_held_tool, common, slime_entities, tool_stack};
use pluto_lib::{ActionTarget, EntityAction, Identifier, PlayerBlockBreakEvents, ServerLivingEntityEvents, StackSlot, World};

/// The wear actions of a landed enderport: the tool in the main hand takes `amount` damage.
fn wear(held: &pluto_lib::StackData, amount: i32) -> Option<EntityAction> {
    let edits = with_held_tool(held, |_env, stack| tool_stack::damage_edits(&stack.custom_data, amount)).ok()?;
    (!edits.is_empty()).then(|| EntityAction::EditStackData { target: ActionTarget::Attacker, slot: StackSlot::MainHand, item: held.item.clone(), edits })
}

/// Whether the tool in hand carries enderporting.
fn carries(held: &pluto_lib::StackData) -> bool {
    matches!(with_held_tool(held, |env, stack| tool_stack::enderporting(&stack.custom_data, env)), Ok(true))
}

/// Registers the callbacks. Called once from the product's hooks.
pub(crate) fn register() {
    // finishHarvest: a block harvested with a harvest tool that carries enderporting pulls the
    // holder onto the block's centre, and wears the tool by 2.
    PlayerBlockBreakEvents::AFTER.register(|broken| {
        let Some(held) = broken.held.as_ref() else { return; };
        let Some(items) = common::item_snapshot() else { return; };
        let (Ok(item), Ok(harvest)) = (Identifier::parse(&held.item), Identifier::new("tconstruct", "modifiable/harvest")) else { return; };
        if !items.get_tag(&harvest).is_some_and(|members| members.contains(&item)) { return; }
        if !carries(held) { return; }
        let dest = [f64::from(broken.pos[0]) + 0.5, f64::from(broken.pos[1]), f64::from(broken.pos[2]) + 0.5];
        let Some(world) = World::ticking() else { return; };
        if !slime_entities::teleport_lands(&world, dest) { return; }
        let mut actions = vec![EntityAction::Teleport { target: ActionTarget::Attacker, position: dest }];
        actions.extend(wear(held, 2));
        broken.actions.borrow_mut().extend(actions);
    });
    // afterMeleeHit: the attacker and the mob it hit trade places, and the tool wears by 2. An
    // extra attack is not told apart by the weapon-hit telling, so every direct hit swaps.
    ServerLivingEntityEvents::WEAPON_HIT.register(|hit| {
        let Some(held) = hit.weapon.as_ref() else { return; };
        if !carries(held) { return; }
        let (Some(attacker), Some(target)) = (hit.attacker_position, hit.target_position) else { return; };
        let mut actions = vec![
            EntityAction::Teleport { target: ActionTarget::Attacker, position: target },
            EntityAction::Teleport { target: ActionTarget::Victim, position: attacker },
        ];
        actions.extend(wear(held, 2));
        hit.actions.borrow_mut().extend(actions);
    });
}
