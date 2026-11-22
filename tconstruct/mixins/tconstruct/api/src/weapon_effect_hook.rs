//! Connects the weapon mob effect and hit action modifiers to PlutoLib's weapon-hit event: when a player hits a mob with a Tinkers tool, each of the
//! tool's weapon effect modules that lands puts its status effect on the mob (MobEffectModule.Weapon). Only effects the game itself
//! defines (the `minecraft` ones) can land; the original's own effects (venom, conductive, pierce) have no registration here.
use crate::{break_speed_hook::with_held_tool, common, melee_damage, melee_hook, tool_damage_hook, tool_stack};
use pluto_lib::{ActionTarget, EffectGrant, EntityAction, Identifier, ProjectileEvents, ServerEntityEvents, ServerLivingEntityEvents, StackSlot};

/// The action that puts a tool usage module's effect on the player using the tool. Only the game's own effects can land.
fn self_effect(grant: crate::weapon_effect::Grant) -> Option<EntityAction> {
    if grant.effect.namespace() != "minecraft" { return None; }
    Some(EntityAction::AddEffect { target: ActionTarget::Attacker, effect: grant.effect.to_string(), amplifier: grant.amplifier, duration: grant.duration })
}

/// The effects of the Tinkers tool among `stacks` (the first that is one) for a use of it, as actions on its holder.
pub(crate) fn usage_actions(stacks: &[&Option<pluto_lib::StackData>], usage: crate::weapon_effect::Usage) -> Vec<EntityAction> {
    for stack in stacks.iter().filter_map(|stack| stack.as_ref()) {
        if let Ok(grants) = with_held_tool(stack, |env, slot| tool_stack::usage_effects(&slot.custom_data, env, usage, &mut tool_damage_hook::random)) {
            return grants.into_iter().filter_map(self_effect).collect();
        }
    }
    Vec::new()
}

/// Registers the callbacks. Called once from the product's hooks.
pub(crate) fn register() {
    // Armor that counters: each worn piece's counter modules may put an effect on the mob that hit the wearer.
    ServerLivingEntityEvents::WEARER_HIT.register(|hit| {
        let entities = common::entity_type_snapshot();
        let attacker = Identifier::parse(&hit.kind).ok();
        let in_tag = |tag: &Identifier| {
            if tag.namespace() == melee_damage::STATE_NAMESPACE { return melee_damage::state_in_tag(tag, Some(&hit.attacker_state), None); }
            melee_hook::target_in_tag(entities.as_deref(), attacker.as_ref(), tag)
        };
        let mut actions = Vec::new();
        for (_, piece) in &hit.armor {
            // Fire, freezing and knockback against the attacker.
            if let Ok(done) = with_held_tool(piece, |env, stack| tool_stack::counter_actions(&stack.custom_data, env, &in_tag, &mut tool_damage_hook::random)) { actions.extend(done); }
            // Thorns: the damage goes to the attacker as the damage type names it (the game's own types only).
            if let Ok(hurts) = with_held_tool(piece, |env, stack| tool_stack::thorns_hurts(&stack.custom_data, env, &in_tag, &mut tool_damage_hook::random)) {
                for hurt in hurts {
                    if hurt.damage_type.namespace() != "minecraft" { continue; }
                    hit.counter_damage.borrow_mut().push(pluto_lib::CounterDamage { damage_type: hurt.damage_type.to_string(), amount: hurt.amount });
                }
            }
            let Ok(grants) = with_held_tool(piece, |env, stack| tool_stack::counter_effects(&stack.custom_data, env, &in_tag, &mut tool_damage_hook::random)) else { continue; };
            for (whom, grant) in grants {
                if grant.effect.namespace() != "minecraft" { continue; }
                let grant = EffectGrant { effect: grant.effect.to_string(), amplifier: grant.amplifier, duration: grant.duration };
                if whom == crate::weapon_effect::Whom::Wearer { hit.self_effects.borrow_mut().push(grant); } else { hit.effects.borrow_mut().push(grant); }
            }
        }
        hit.actions.borrow_mut().extend(crate::hit_actions::strongest_knockback(actions));
    });
    ServerLivingEntityEvents::WEAPON_HIT.register(|hit| {
        let entities = common::entity_type_snapshot();
        let target = Identifier::parse(&hit.kind).ok();
        let in_tag = |tag: &Identifier| {
            if tag.namespace() == melee_damage::STATE_NAMESPACE { return melee_damage::state_in_tag(tag, hit.attacker_state.as_ref(), hit.target_state.as_ref()); }
            melee_hook::target_in_tag(entities.as_deref(), target.as_ref(), tag)
        };
        // Armor that hits back: each worn piece's armor attack modules may put an effect on the mob the wearer hit (MobEffectModule.ArmorAttack).
        // A weapon hit is the wearer's direct melee damage of the player attack type.
        if let Ok(damage_type) = Identifier::parse("minecraft:player_attack") {
            let tags = crate::protection_hook::tags();
            let no_attacker = |_: &Identifier| false;
            let source = crate::protection::DamageSource { damage_type: &damage_type, indirect: false, tags: &tags, attacker_in_tag: &no_attacker, entities: Default::default() };
            for (_, piece) in &hit.armor {
                let Ok(grants) = with_held_tool(piece, |env, stack| tool_stack::armor_attack_effects(&stack.custom_data, env, &source, &in_tag, &mut tool_damage_hook::random)) else { continue; };
                for grant in grants {
                    if grant.effect.namespace() != "minecraft" { continue; }
                    hit.effects.borrow_mut().push(EffectGrant { effect: grant.effect.to_string(), amplifier: grant.amplifier, duration: grant.duration });
                }
            }
        }
        let Some(held) = hit.weapon.as_ref() else { return; };
        // Effects the tool puts on its own holder as it swings (MobEffectModule.ToolUsage).
        hit.actions.borrow_mut().extend(usage_actions(&[&hit.weapon], crate::weapon_effect::Usage::MeleeHit));
        // Fire and freezing on the mob hit.
        if let Ok(done) = with_held_tool(held, |env, stack| tool_stack::attack_actions(&stack.custom_data, env)) { hit.actions.borrow_mut().extend(done); }
        // Melee capacity: the tool's modules fill (or drain) their bars as it hits; the edit lands on the held stack before the hit's damage.
        let player = Identifier::parse("minecraft:player").ok();
        let attacker_in_tag = |tag: &Identifier| {
            if tag.namespace() == melee_damage::STATE_NAMESPACE { return melee_damage::state_in_tag(tag, hit.attacker_state.as_ref(), hit.target_state.as_ref()); }
            melee_hook::target_in_tag(entities.as_deref(), player.as_ref(), tag)
        };
        if let Ok(edits) = with_held_tool(held, |env, stack| tool_stack::melee_capacity_edits(&stack.custom_data, env, &in_tag, &attacker_in_tag)) {
            if !edits.is_empty() { hit.actions.borrow_mut().push(EntityAction::EditStackData { target: ActionTarget::Attacker, slot: StackSlot::MainHand, item: held.item.clone(), edits }); }
        }
        let Ok(grants) = with_held_tool(held, |env, stack| tool_stack::weapon_effects(&stack.custom_data, env, &in_tag, &mut tool_damage_hook::random)) else { return; };
        for (_, grant) in grants {
            // Effects that apply before the hit and after it both land here, before the damage; the game has no later mod call.
            if grant.effect.namespace() != "minecraft" { continue; }
            hit.effects.borrow_mut().push(EffectGrant { effect: grant.effect.to_string(), amplifier: grant.amplifier, duration: grant.duration });
        }
    });
    // A bow or crossbow shot by the player: the tool's usage effects for the holder (MobEffectModule.ToolUsage's launch half, once for each
    // arrow reported: a multishot's extra arrows are not told apart, so each rolls on its own).
    ProjectileEvents::SHOOT_BOW.register(|shot| {
        shot.actions.borrow_mut().extend(usage_actions(&[&shot.main_hand, &shot.off_hand], crate::weapon_effect::Usage::Launch { primary: true }));
    });
    // Launch capacity (LaunchCapacityModule): the bar of the launcher in either hand (the first Tinkers tool, as the usage effects pick it) is
    // filled or drained once for each arrow. The host reads the hands anew for each arrow and applies the edit at once, so a multishot's
    // arrows add up.
    ProjectileEvents::SHOOT_BOW.register(|shot| {
        for (slot, stack) in [(StackSlot::MainHand, &shot.main_hand), (StackSlot::OffHand, &shot.off_hand)] {
            let Some(held) = stack.as_ref() else { continue; };
            let Ok(edits) = with_held_tool(held, |env, tool| tool_stack::launch_capacity_edits(&tool.custom_data, env)) else { continue; };
            if !edits.is_empty() { shot.actions.borrow_mut().push(EntityAction::EditStackData { target: ActionTarget::Attacker, slot, item: held.item.clone(), edits }); }
            break;
        }
    });
    // Shearing a mob with a tool (MobEffectModule.ToolUsage's shear half): heard before the shears act.
    ServerEntityEvents::SHEAR.register(|attempt| {
        attempt.actions.borrow_mut().extend(usage_actions(&[&attempt.tool], crate::weapon_effect::Usage::Shear { target: true }));
    });
}
