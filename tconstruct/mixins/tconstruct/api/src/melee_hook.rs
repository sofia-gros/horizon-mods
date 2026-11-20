//! Connects the conditional melee damage modifiers to PlutoLib's damage event: when a player hits with a
//! Tinkers tool, its modifiers change the damage as MeleeDamageModifierHook does in the original. The event
//! reports the damage after the game worked it out (before armor), not the attack attribute alone, so the
//! `base_damage` variable is the player's base attack of 1 plus the tool's attack damage stat.
use crate::{break_speed_hook::with_held_tool, common, tool_stack, tool_stats::Stat};
use pluto_lib::{Identifier, ServerLivingEntityEvents};
use std::sync::atomic::{AtomicUsize, Ordering};

const PLAYER_ATTACK: &str = "minecraft:player_attack";

/// What vanilla's entity type tags hold, for the mob types the original's target conditions name. Used when the game
/// reports no entity type tags of its own (the captured registry is absent or lacks the tag).
fn vanilla_tag_members(tag: &str) -> &'static [&'static str] {
    match tag {
        "minecraft:undead" => &["bogged", "drowned", "husk", "phantom", "skeleton", "skeleton_horse", "stray", "wither", "wither_skeleton", "zoglin", "zombie", "zombie_horse", "zombie_villager", "zombified_piglin"],
        "minecraft:arthropod" => &["bee", "cave_spider", "endermite", "silverfish", "spider"],
        "minecraft:illager" => &["evoker", "illusioner", "pillager", "vindicator"],
        "minecraft:aquatic" => &["axolotl", "cod", "dolphin", "elder_guardian", "glow_squid", "guardian", "pufferfish", "salmon", "squid", "tadpole", "tropical_fish", "turtle"],
        _ => &[],
    }
}

/// Whether `target` is in the entity type tag: the captured registry's answer when it knows the tag, else vanilla's contents.
pub(crate) fn target_in_tag(entities: Option<&pluto_lib::RegistryLookup>, target: Option<&Identifier>, tag: &Identifier) -> bool {
    let Some(target) = target else { return false; };
    if let Some(members) = entities.and_then(|entities| entities.get_tag(tag)) { return members.contains(target); }
    target.namespace() == "minecraft" && vanilla_tag_members(&tag.to_string()).contains(&target.path())
}

/// Registers the callback. Called once from the product's hooks.
pub(crate) fn register() {
    // The first few hits are logged with what the hook saw, so a hit it leaves alone is visible in the game log.
    static LOGGED: AtomicUsize = AtomicUsize::new(0);
    ServerLivingEntityEvents::MODIFY_DAMAGE.register(|hit| {
        let Some(held) = hit.attacker_stack.as_ref() else { return; };
        if hit.damage.damage_type != PLAYER_ATTACK { return; }
        let log = LOGGED.fetch_add(1, Ordering::Relaxed) < 8;
        let amount = hit.amount.get();
        // The target's entity type tags come from the entity type registry captured at the last reload.
        let entities = common::entity_type_snapshot();
        let target = Identifier::parse(&hit.damage.kind).ok();
        let in_tag = |tag: &Identifier| {
            if tag.namespace() == crate::melee_damage::STATE_NAMESPACE { return crate::melee_damage::state_in_tag(tag, hit.attacker_state.as_ref(), hit.target_state.as_ref()); }
            target_in_tag(entities.as_deref(), target.as_ref(), tag)
        };
        let changed = with_held_tool(held, |env, stack| {
            let base = 1.0 + tool_stack::stat(&stack.custom_data, Stat::AttackDamage);
            tool_stack::melee_damage(&stack.custom_data, env, base, amount, &in_tag, &crate::entity_facts::Entities { attacker: hit.attacker_state.as_ref(), target: hit.target_state.as_ref() })
        });
        if log {
            eprintln!("Tinkers: melee hit on {} with {}: {} -> {:?} (entity type tags captured: {}; attacker sprinting: {:?}; target fire ticks: {:?})",
                hit.damage.kind, held.item, amount, changed, entities.is_some(),
                hit.attacker_state.as_ref().map(|state| state.sprinting), hit.target_state.as_ref().map(|state| state.fire_ticks));
        }
        if let Ok(damage) = changed {
            if damage.is_finite() && damage >= 0.0 && damage != amount { hit.amount.set(damage); }
        }
    });
    // Knockback: the modules of the tool in the attacker's hand add to the strength the game worked out.
    static KNOCKS: AtomicUsize = AtomicUsize::new(0);
    ServerLivingEntityEvents::MODIFY_KNOCKBACK.register(|push| {
        let Some(held) = push.attacker_main.as_ref() else { return; };
        if push.cancel.get() { return; }
        let strength = push.strength.get();
        // The strength the game reports for a hit with a Tinkers tool (the tool's attack knockback attribute is part of it).
        if held.custom_data.contains_key(crate::tool_stack::TAG_MATERIALS) && KNOCKS.fetch_add(1, Ordering::Relaxed) < 12 {
            eprintln!("Tinkers: knockback strength {strength:.3} on {} with {}", push.kind, held.item);
        }
        let entities = common::entity_type_snapshot();
        let target = Identifier::parse(&push.kind).ok();
        let in_tag = |tag: &Identifier| target_in_tag(entities.as_deref(), target.as_ref(), tag);
        let Ok(changed) = with_held_tool(held, |env, stack| tool_stack::melee_knockback(&stack.custom_data, env, strength as f32, &in_tag)) else { return; };
        let changed = f64::from(changed);
        if changed.is_finite() && changed >= 0.0 && (changed - strength).abs() > 1e-6 { push.strength.set(changed); }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }

    #[test]
    fn without_a_captured_tag_vanillas_mob_type_tags_answer() {
        let husk = id("minecraft:husk");
        assert!(target_in_tag(None, Some(&husk), &id("minecraft:undead")));
        assert!(!target_in_tag(None, Some(&husk), &id("minecraft:arthropod")));
        assert!(target_in_tag(None, Some(&id("minecraft:spider")), &id("minecraft:arthropod")));
        assert!(!target_in_tag(None, Some(&id("modded:husk")), &id("minecraft:undead")), "only vanilla's own entity types");
        assert!(!target_in_tag(None, None, &id("minecraft:undead")));
        assert!(!target_in_tag(None, Some(&husk), &id("tconstruct:killagers")), "an unknown tag matches nothing");
    }
}
