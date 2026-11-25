//! Monster melee modifiers, from ToolEvents.livingHurt and ToolEvents.livingDamage in the original: a mob
//! holding a Tinkers tool applies the tool's melee modifiers to its hits. Both dispatches are gated by the
//! `allowMonsterMeleeModifiers` config, the damage type being in `tconstruct:modifier_whitelist` and the
//! attacker's type not being in `tconstruct:damage_modifier_blacklist` (which holds `minecraft:player`, so
//! player attacks never run here; they have their own dispatch). The weapon branch runs when the mob's main
//! hand holds a `tconstruct:modifiable/melee/weapon` tool; the unarmed branch when the hand is empty and
//! its chest wears a `tconstruct:modifiable/melee/unarmed` tool. PlutoLib's damage events report the mob
//! behind a synchronous attack with its type, its main-hand and chest stacks, its attack attribute and its
//! state, and the victim's health and attack attribute, which is everything the gates and evaluations read.
//!
//! The unarmed branch mirrors the original's quirk: its base damage starts from the tool's attack damage
//! (as wielded) plus the difference between the incoming damage and the *victim's* attack attribute, and
//! the running damage starts from that base rather than from the incoming amount.
//!
//! The melee instrument sound rides the same hit as an `EntityAction::PlaySound` on the attacker, and the unarmed branch runs
//! against a player victim too: PlutoLib's transport remembers a player victim's attack attribute from their tick, so the quirk's
//! base can be worked out for them as for a mob victim. The smelting cook of a monster's melee hit, its melee capacity fills, and
//! the enderporting swap's wear by 2 ride the mob-stack-write surface (`EntityAction::EditStackData` on the attacker, MainHand for a
//! held weapon, Chest for the unarmed chest piece).
use crate::{break_speed_hook::with_held_tool, common, config, melee_damage, melee_hook, protection_hook, tool_damage_hook, tool_stack, tool_stats::Stat, weapon_effect_hook};
use pluto_lib::{ActionTarget, DamageModification, EntityAction, EntityState, Identifier, ServerLivingEntityEvents, StackData, StackSlot};

/// Which tool of the mob's the hit is made with.
enum Attack<'a> { Weapon(&'a StackData), Unarmed(&'a StackData) }

impl<'a> Attack<'a> {
    fn stack(&self) -> &'a StackData { match self { Attack::Weapon(stack) | Attack::Unarmed(stack) => stack } }

    /// The stack the hit's edits land on: the held weapon, or the chest piece for the unarmed form.
    fn slot(&self) -> StackSlot { match self { Attack::Weapon(_) => StackSlot::MainHand, Attack::Unarmed(_) => StackSlot::Chest } }
}

/// Whether `stack`'s item is in the item tag `tag`, from the captured item registry.
fn in_item_tag(stack: &StackData, tag: &str) -> bool {
    let (Ok(item), Ok(tag)) = (Identifier::parse(&stack.item), Identifier::parse(tag)) else { return false; };
    let items = common::item_snapshot();
    items.as_deref().and_then(|items| items.get_tag(&tag)).is_some_and(|members| members.contains(&item))
}

/// The monster melee attack a hit is, when every gate of the original holds and the mob's tool is known.
/// A mob the game did not name as the hit's attacker (no attack snapshot) is no attack: its stacks and
/// type are unknown, so neither the blacklist nor the weapon checks could be answered.
fn attack_of(hit: &DamageModification) -> Option<Attack<'_>> {
    if !config::allow_monster_melee_modifiers() { return None; }
    if hit.attacker_kind.is_empty() || hit.attacker_is_player == Some(true) { return None; }
    let damage_type = Identifier::parse(&hit.damage.damage_type).ok()?;
    let whitelist = Identifier::parse("tconstruct:modifier_whitelist").ok()?;
    if !protection_hook::tags().contains(&whitelist, &damage_type) { return None; }
    let entities = common::entity_type_snapshot();
    let attacker = Identifier::parse(&hit.attacker_kind).ok()?;
    let blacklist = Identifier::parse("tconstruct:damage_modifier_blacklist").ok()?;
    if melee_hook::target_in_tag(entities.as_deref(), Some(&attacker), &blacklist) { return None; }
    if let Some(held) = &hit.attacker_main_hand {
        // A held item that is no melee weapon ends the matter: the unarmed branch needs the hand empty.
        if in_item_tag(held, "tconstruct:modifiable/melee/weapon") { return Some(Attack::Weapon(held)); }
        return None;
    }
    let chest = hit.attacker_chest.as_ref()?;
    if in_item_tag(chest, "tconstruct:modifiable/melee/unarmed") { return Some(Attack::Unarmed(chest)); }
    None
}

/// The state of the hit's target as the evaluations read it: its type, and the health the hit found (the
/// weapon-hit report that fills `target_state` only runs for a player's attack).
fn target_state_of(hit: &DamageModification) -> EntityState {
    EntityState { kind: hit.damage.kind.clone(), health: hit.target_health, ..EntityState::default() }
}

/// Registers the callbacks. Called once from the product's hooks.
pub(crate) fn register() {
    // The damage hook (the original's livingHurt, before armor): the tool's melee damage modules change
    // the amount the mob's hit goes on with.
    ServerLivingEntityEvents::MODIFY_HURT.register(|hit| {
        let Some(attack) = attack_of(hit) else { return; };
        let amount = hit.amount.get();
        let entities = common::entity_type_snapshot();
        let target = Identifier::parse(&hit.damage.kind).ok();
        let target_state = target_state_of(hit);
        let in_tag = |tag: &Identifier| {
            if tag.namespace() == melee_damage::STATE_NAMESPACE { return melee_damage::state_in_tag(tag, hit.attacker_state.as_ref(), Some(&target_state)); }
            melee_hook::target_in_tag(entities.as_deref(), target.as_ref(), tag)
        };
        let facts = crate::entity_facts::Entities { attacker: hit.attacker_state.as_ref(), target: Some(&target_state) };
        let changed = match attack {
            Attack::Weapon(held) => with_held_tool(held, |env, tool| {
                // applyAttributes: the attacker's attack attribute with the tool in hand; the port's player
                // path builds the same base as the attacker's base plus the tool's attack damage stat.
                let base = hit.attacker_attack_damage.unwrap_or(0.0) + tool_stack::stat(&tool.custom_data, Stat::AttackDamage);
                tool_stack::melee_damage(&tool.custom_data, env, base, amount, &in_tag, &facts)
            }),
            Attack::Unarmed(chest) => {
                // The quirk's base needs the victim's attack attribute; without it the hit is left alone.
                let Some(victim_attack) = hit.target_attack_damage else { return; };
                with_held_tool(chest, |env, tool| {
                    let tool_attack = tool_stack::stat(&tool.custom_data, Stat::AttackDamage);
                    let base = hit.attacker_attack_damage.unwrap_or(0.0) + tool_attack + amount - victim_attack;
                    tool_stack::melee_damage(&tool.custom_data, env, base, base, &in_tag, &facts)
                })
            }
        };
        if let Ok(damage) = changed {
            if damage.is_finite() && damage >= 0.0 && damage != amount { hit.amount.set(damage); }
        }
    });
    // The hit hook (the original's livingDamage, after armor): the tool's hit modules act on the parties,
    // by the damage actually dealt, which the original first cuts to the health the victim had.
    ServerLivingEntityEvents::MODIFY_DAMAGE.register(|hit| {
        let Some(attack) = attack_of(hit) else { return; };
        let amount = hit.amount.get();
        let dealt = hit.target_health.map_or(amount, |health| amount.min(health));
        if dealt <= 0.0 { return; }
        let stack = attack.stack();
        // EnderportingModifier's monster-held form: a monster's melee hit with an enderporting
        // weapon trades the attacker and the victim (the wear by 2 rides the mob-stack-write
        // surface with the swap — see enderporting_hook). Only a held weapon swaps; the unarmed
        // form's chest piece is no weapon.
        if matches!(&attack, Attack::Weapon(_)) {
            crate::enderporting_hook::monster_swap(hit);
        }
        let entities = common::entity_type_snapshot();
        let target = Identifier::parse(&hit.damage.kind).ok();
        let attacker_kind = Identifier::parse(&hit.attacker_kind).ok();
        let target_state = target_state_of(hit);
        let target_tag = |tag: &Identifier| {
            if tag.namespace() == melee_damage::STATE_NAMESPACE { return melee_damage::state_in_tag(tag, hit.attacker_state.as_ref(), Some(&target_state)); }
            melee_hook::target_in_tag(entities.as_deref(), target.as_ref(), tag)
        };
        let attacker_tag = |tag: &Identifier| {
            if tag.namespace() == melee_damage::STATE_NAMESPACE { return melee_damage::state_in_tag(tag, hit.attacker_state.as_ref(), Some(&target_state)); }
            melee_hook::target_in_tag(entities.as_deref(), attacker_kind.as_ref(), tag)
        };
        // Effects the tool puts on its own holder as it swings (MobEffectModule.ToolUsage, whose modules
        // bind the monster melee hit as a usage in this port's data).
        let usage_stack = Some(stack.clone());
        hit.actions.borrow_mut().extend(weapon_effect_hook::usage_actions(&[&usage_stack], crate::weapon_effect::Usage::MeleeHit));
        let Ok(done) = with_held_tool(stack, |env, tool| {
            let mut actions = Vec::new();
            // Fire and freezing on the entity hit.
            actions.extend(tool_stack::attack_actions(&tool.custom_data, env));
            // The mob effects the tool puts on its target; the ones applying before the hit and after it
            // both land here, as on the player's weapon-hit path.
            for (_, grant) in tool_stack::weapon_effects(&tool.custom_data, env, &target_tag, &mut tool_damage_hook::random) {
                // The original puts every granted effect on the target, whatever namespace it is in;
                // an effect the game does not know is skipped by the host, as on the player's path.
                actions.push(EntityAction::AddEffect { target: ActionTarget::Victim, effect: grant.effect.to_string(), amplifier: grant.amplifier, duration: grant.duration });
            }
            // Lifesteal: the wielder is healed by a share of the damage dealt (the monster half of the
            // module, which has no critical gate; the player's half adds one).
            let blacklisted = Identifier::new("tconstruct", "necrotic_blacklist").is_ok_and(|tag| target_tag(&tag));
            let heal = tool_stack::lifesteal_heal(&tool.custom_data, env, dealt, blacklisted, &attacker_tag, &target_tag);
            if heal > 0.0 { actions.push(EntityAction::Heal { target: ActionTarget::Attacker, amount: heal }); }
            // Melee instrument: the hit plays the tool's instrument (MeleeInstrumentModule, on the monster melee hit hook the
            // original hears in livingDamage), at the attacker, in the records category the original plays it in.
            for play in tool_stack::melee_instrument_sound(&tool.custom_data, env, &attacker_tag, &target_tag, tool_damage_hook::random()) {
                actions.push(EntityAction::PlaySound { target: ActionTarget::Attacker, sound: play.event, volume: play.volume, pitch: 1.0, source: 2 });
            }
            // Smelting: the hit cooks the attacker's tool by the damage dealt, as the player's weapon-hit path does. The edits land
            // on the attacker's own stack through the mob-stack-write surface (the held weapon, or the chest piece for the unarmed
            // form), not on the victim.
            let mut edits = tool_stack::smelting_edits(&tool.custom_data, env, dealt);
            // Melee capacity: the hit fills (or drains) the tool's capacity bars, as the player's weapon-hit path does (MeleeCapacityModule
            // afterMeleeHit, heard here on the damage dispatch); the grant is flat per hit, gated by the target and attacker conditions,
            // and rides the same EditStackData push as the cooking, on the same stack.
            edits.extend(tool_stack::melee_capacity_edits(&tool.custom_data, env, &target_tag, &attacker_tag));
            if !edits.is_empty() {
                actions.push(EntityAction::EditStackData { target: ActionTarget::Attacker, slot: attack.slot(), item: stack.item.clone(), edits });
            }
            actions
        }) else { return; };
        hit.actions.borrow_mut().extend(done);
    });
}
