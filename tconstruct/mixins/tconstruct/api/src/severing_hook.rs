//! Connects SeveringModule to PlutoLib's weapon-hit and loot events. ModifierLootModifier runs a tool's loot modules for the mob a player kills
//! with it, taking the tool from the killer's hand; the loot event of a mob's death reports no tool, so the weapon-hit event, heard just before
//! the damage that kills, leaves the tool's severing levels and looting for the loot event of that mob (matched by entity id, and only within a
//! moment of the hit, so a mob that dies later of something else is left alone). The looting left is the total the original's
//! ModifierLootingHandler computes for a kill: the weapon's own looting plus every worn piece's armor looting modules
//! (ArmorLootingModifierHook.getLooting), which is the total SeveringModule.processLoot reads as the context's looting modifier.
//! PlutoLib has no looting level event, so ordinary mob drops keep rolling from the weapon's published enchantments alone;
//! the armor part reaches the loot processing this port runs itself, as above.
use crate::{break_speed_hook::with_held_tool, common, melee_hook, tool_damage_hook, tool_stack};
use pluto_lib::{Identifier, LootTableEvents, ServerLivingEntityEvents};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

/// How long a hit's tool stays waiting for the death it may cause.
const WINDOW: Duration = Duration::from_millis(500);

struct Pending {
    entity: u32,
    levels: Vec<f32>,
    looting: i32,
    at: Instant,
}
static PENDING: Mutex<Vec<Pending>> = Mutex::new(Vec::new());

fn leave(entity: u32, levels: Vec<f32>, looting: i32) {
    let mut pending = PENDING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    pending.retain(|hit| hit.at.elapsed() < WINDOW && hit.entity != entity);
    if !levels.is_empty() {
        pending.push(Pending {
            entity,
            levels,
            looting,
            at: Instant::now(),
        });
    }
}

fn take(entity: u32) -> Option<Pending> {
    let mut pending = PENDING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    pending.retain(|hit| hit.at.elapsed() < WINDOW);
    let at = pending.iter().position(|hit| hit.entity == entity)?;
    Some(pending.remove(at))
}

/// Registers the callbacks. Called once from the product's hooks.
pub(crate) fn register() {
    ServerLivingEntityEvents::WEAPON_HIT.register(|hit| {
        let Ok(entity) = u32::try_from(hit.entity) else {
            return;
        };
        let Some(weapon) = hit.weapon.as_ref() else {
            return;
        };
        // ModifierLootModifier: a tool of the loot capable tag that is not broken (with_held_tool leaves a broken one alone).
        let Some(items) = common::item_snapshot() else {
            return;
        };
        let (Ok(item), Ok(capable)) = (
            Identifier::parse(&weapon.item),
            Identifier::new("tconstruct", "modifiable/loot_capable_tool"),
        ) else {
            return;
        };
        if !items
            .get_tag(&capable)
            .is_some_and(|members| members.contains(&item))
        {
            return;
        }
        if let Ok((levels, looting)) = with_held_tool(weapon, |env, stack| {
            (
                tool_stack::severing_levels(&stack.custom_data, env),
                tool_stack::looting(&stack.custom_data, env),
            )
        }) {
            // ArmorLootingModifierHook.getLooting: every other worn piece's armor looting modules add to the weapon's
            // looting; the hit reports the attacker's worn armor, each piece read as a valid tool like the weapon.
            let mut looting = looting;
            for (slot, piece) in &hit.armor {
                if let Ok(piece_looting) = with_held_tool(piece, |env, stack| tool_stack::armor_looting(&stack.custom_data, env, slot)) {
                    looting += piece_looting;
                }
            }
            leave(entity, levels, looting);
        }
    });
    LootTableEvents::MODIFY_DROPS.register(|drops| {
        let (Some((entity, kind)), None, false) = (drops.entity.as_ref(), drops.block, drops.used)
        else {
            return;
        };
        let Some(hit) = take(*entity) else { return };
        let (Some(severing), Ok(kind)) = (common::severing(), Identifier::parse(kind)) else {
            return;
        };
        let Ok(recipes) = severing.read() else { return };
        // The mob's entity type tags come from the captured entity type registry (the original's rare mob tag defines none, so it doubles nothing).
        let entities = common::entity_type_snapshot();
        let in_tag =
            |tag: &Identifier| melee_hook::target_in_tag(entities.as_deref(), Some(&kind), tag);
        let rare = Identifier::new("tconstruct", "rare_mobs").is_ok_and(|tag| in_tag(&tag));
        let mut loot = drops.drops.borrow_mut();
        for level in hit.levels {
            recipes.process(
                &kind,
                &in_tag,
                rare,
                level,
                hit.looting as f32,
                &mut loot,
                &mut tool_damage_hook::random,
            );
        }
    });
}
