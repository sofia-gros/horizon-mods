//! StrongBonesModifier (bone): finishing a milk bucket with a strong-bones helmet grants
//! damage resistance for 600 ticks plus 600 a level, and the wearer's calcifiable armor
//! levels (the ArmorLevelModule key StrongBonesModifier.CALCIFIABLE, which FrosttouchModifier
//! and WitheredModifier add) grant the calcified effect on the same count. The resistance is
//! cured when the helmet comes off (CureOnRemovalModule.HELMET), over the player tick below.
//! SlimeKnights (MIT; see LICENSE).
//!
//! What is not carried over, with the reasons: the granted instance's curative items — the
//! original replaces them with the helmet itself, so a later milk drink does not cure the
//! resistance; an effect here carries no curative items, so in this port a later drink cures
//! it as it cures everything. The removal cure therefore tracks the grants this hook made and
//! takes the resistance off when the helmet leaves, which is the half a surface here reaches.
//! The spilling form (StrongBonesModifier.FLUID_EFFECT: milk spilled on a target grants the
//! same at 200 ticks a unit of scale) is not wired, and neither is a mob drinker: the finish
//! event here reports players only, and the calcifiable total counts worn pieces, not a held
//! one (the original's HELD_ARMOR filter also counts a piece in hand).
use crate::{break_speed_hook::with_held_tool, tool_stack};
use pluto_lib::{ServerLivingEntityEvents, UseItemFinishCallback};
use std::collections::BTreeSet;
use std::sync::Mutex;

/// The players whose current damage resistance this hook granted, by uuid: the removal cure
/// takes only what the modifier granted.
static GRANTED: Mutex<BTreeSet<u128>> = Mutex::new(BTreeSet::new());

/// The strong-bones level of the player's helmet, from the pieces the player tick last reported.
fn helmet_level(player: u128) -> i32 {
    let Some((_, head)) = pluto_lib::entity_events_v1::equipment_of(player).into_iter().find(|(slot, _)| slot == "head") else { return 0; };
    with_held_tool(&head, |env, tool| tool_stack::modifier_level(&tool.custom_data, env, "tconstruct:strong_bones")).unwrap_or(0)
}

/// ArmorLevelModule.getLevel over the worn pieces for StrongBonesModifier.CALCIFIABLE
/// (`tconstruct:calcifable`, the key's own spelling in the original).
fn calcifiable_level(player: u128) -> i32 {
    let key = pluto_lib::Identifier::parse("tconstruct:calcifable").expect("fixed key");
    let mut total = 0;
    for (slot, piece) in pluto_lib::entity_events_v1::equipment_of(player) {
        if slot == "offhand" { continue; }
        let Ok(levels) = with_held_tool(&piece, |env, tool| tool_stack::armor_levels(&tool.custom_data, env, &slot)) else { continue; };
        total += levels.iter().filter(|(name, _)| *name == key).map(|(_, level)| level).sum::<i32>();
    }
    total
}

/// Registers the milk finish and the removal cure. Called once from the product's hooks.
pub(crate) fn register() {
    // StrongBonesModifier.onItemFinishUse: milk's finish with the helmet on.
    UseItemFinishCallback::EVENT.register(|done| {
        if done.item != "minecraft:milk_bucket" { return; }
        let level = helmet_level(done.player.uuid);
        if level > 0 && done.player.add_effect(&pluto_lib::player_api::EffectInstance::new("minecraft:resistance", 600 + 600 * level)) {
            let _ = GRANTED.lock().map(|mut granted| granted.insert(done.player.uuid));
        }
        let calcified = calcifiable_level(done.player.uuid);
        if calcified > 0 {
            done.player.add_effect(&pluto_lib::player_api::EffectInstance::new("tconstruct:calcified", 600 + 600 * calcified));
        }
    });
    // CureOnRemovalModule.HELMET: the granted resistance ends when the helmet comes off.
    ServerLivingEntityEvents::PLAYER_TICK.register(|tick| {
        if !GRANTED.lock().map(|granted| granted.contains(&tick.player)).unwrap_or(false) { return; }
        let keeps = tick.armor.iter().find(|(slot, _)| slot == "head")
            .is_some_and(|(_, head)| with_held_tool(head, |env, tool| tool_stack::modifier_level(&tool.custom_data, env, "tconstruct:strong_bones") > 0).unwrap_or(false));
        if !keeps {
            tick.remove_effects.borrow_mut().push("minecraft:resistance".to_owned());
            let _ = GRANTED.lock().map(|mut granted| granted.remove(&tick.player));
        }
    });
}
