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

/// ModifierEvents: an effect starting on an entity lasts longer or shorter by the entity's good or
/// bad effect duration multiplier, worked out from the pieces it wears and the effects it carries.
/// A grant whose effect has no known category is left as it is.
fn scaled(grant: EffectGrant, pieces: &[(String, pluto_lib::StackData)], effects: &[(String, i32)]) -> EffectGrant {
    let Some(category) = pluto_lib::effect_category(&grant.effect) else { return grant; };
    let multiplier = crate::custom_attributes::effect_duration_multiplier(pieces, effects, category == pluto_lib::EffectCategory::Beneficial);
    EffectGrant { duration: crate::custom_attributes::scaled_duration(grant.duration, multiplier), ..grant }
}

/// [`scaled`] against an entity state the game reported, or the grant as it is when it reported none.
fn scaled_on(grant: EffectGrant, state: Option<&pluto_lib::EntityState>) -> EffectGrant {
    let Some(state) = state else { return grant; };
    scaled(grant, &state.armor_stacks, &state.effect_levels)
}

/// Whether a thorns hurt's damage type may reach the game as counter damage: the game's own
/// types and the port's own registered ones (the `tconstruct` damage types shipped as content
/// data, `tconstruct:shock` among them). A type from any other namespace is not one the game
/// knows here, so it stays dropped. There is no Rust-side registry of the port's damage types
/// to check against — they live as content data files — so the namespace is the boundary.
fn counter_damage_type_allowed(damage_type: &Identifier) -> bool {
    matches!(damage_type.namespace(), "minecraft" | "tconstruct")
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
        // The hit's damage, for the counter modules that condition on its source. The transport resolves the damage type at the
        // damage event that follows the hurt report, so the type is the real one (a mob's melee hit, an arrow, ...) and its tags
        // answer; an unparsed type still stands as the generic one, which no tag names. The hit is indirect when something other
        // than the attacker dealt it (an arrow's shooter, for example).
        let tags = crate::protection_hook::tags();
        let damage_type = Identifier::parse(&hit.damage_type).unwrap_or_else(|_| Identifier::parse("minecraft:generic").unwrap());
        let indirect = hit.direct >= 0 && hit.direct != hit.attacker;
        let source = crate::protection::DamageSource { damage_type: &damage_type, indirect, tags: &tags, attacker_in_tag: &in_tag, entities: crate::entity_facts::Entities { attacker: Some(&hit.attacker_state), target: None } };
        let mut actions = Vec::new();
        for (_, piece) in &hit.armor {
            // Fire, freezing and knockback against the attacker.
            if let Ok(done) = with_held_tool(piece, |env, stack| tool_stack::counter_actions(&stack.custom_data, env, &in_tag, &mut tool_damage_hook::random)) { actions.extend(done); }
            // Thorns: the damage goes to the attacker as the damage type names it (the game's own types and the port's own registered ones).
            if let Ok(hurts) = with_held_tool(piece, |env, stack| tool_stack::thorns_hurts(&stack.custom_data, env, &in_tag, &mut tool_damage_hook::random)) {
                for hurt in hurts {
                    if !counter_damage_type_allowed(&hurt.damage_type) { continue; }
                    hit.counter_damage.borrow_mut().push(pluto_lib::CounterDamage { damage_type: hurt.damage_type.to_string(), amount: hurt.amount });
                }
            }
            let Ok(grants) = with_held_tool(piece, |env, stack| tool_stack::counter_effects(&stack.custom_data, env, &source, &in_tag, &mut tool_damage_hook::random)) else { continue; };
            for (whom, grant) in grants {
                if grant.effect.namespace() != "minecraft" { continue; }
                let grant = EffectGrant { effect: grant.effect.to_string(), amplifier: grant.amplifier, duration: grant.duration };
                // The wearer's own grants are scaled by the wearer's worn pieces (their active
                // effects are not reported here); the mob's by what the hit reports of it.
                if whom == crate::weapon_effect::Whom::Wearer { hit.self_effects.borrow_mut().push(scaled(grant, &hit.armor, &[])); } else { hit.effects.borrow_mut().push(scaled(grant, &hit.attacker_state.armor_stacks, &hit.attacker_state.effect_levels)); }
            }
            // Lacerating: a worn piece may put the bleeding effect on the mob that hit its wearer.
            if let Ok(Some(grant)) = with_held_tool(piece, |env, stack| tool_stack::lacerating_counter_grant(&stack.custom_data, env, &hit.armor, &mut tool_damage_hook::random)) {
                hit.effects.borrow_mut().push(scaled(EffectGrant { effect: grant.effect.to_string(), amplifier: grant.amplifier, duration: grant.duration }, &hit.attacker_state.armor_stacks, &hit.attacker_state.effect_levels));
            }
            // Enderclearance: a hit on the wearer may teleport the mob that dealt it to a random
            // place within the module's diameter of where it stands (the chance doubles on a
            // blocking wearer in the original; blocking is not reported, so the base chance runs).
            if let (Ok(pairs), Some(origin)) = (with_held_tool(piece, |env, stack| tool_stack::enderclearance(&stack.custom_data, env)), hit.attacker_position) {
                for (chance, diameter) in pairs {
                    if crate::tool_damage_hook::random() < chance {
                        let angle = crate::tool_damage_hook::random() * std::f32::consts::TAU;
                        let radius = crate::tool_damage_hook::random() * diameter / 2.0;
                        let to = [origin[0] + f64::from(radius * angle.cos()), origin[1], origin[2] + f64::from(radius * angle.sin())];
                        hit.actions.borrow_mut().push(EntityAction::Teleport { target: ActionTarget::Attacker, position: to });
                    }
                }
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
                    hit.effects.borrow_mut().push(scaled_on(EffectGrant { effect: grant.effect.to_string(), amplifier: grant.amplifier, duration: grant.duration }, hit.target_state.as_ref()));
                }
            }
        }
        let Some(held) = hit.weapon.as_ref() else { return; };
        // Effects the tool puts on its own holder as it swings (MobEffectModule.ToolUsage).
        hit.actions.borrow_mut().extend(usage_actions(&[&hit.weapon], crate::weapon_effect::Usage::MeleeHit));
        // Insatiable: a melee hit feeds the holder's insatiable (melee) effect a level (the grant is
        // the original's own effect, landed like the kinetic feed lands its armor effect).
        if let Ok(Some(grant)) = with_held_tool(held, |env, stack| tool_stack::insatiable_hit_grant(&stack.custom_data, env, hit.attacker_state.as_ref())) {
            hit.actions.borrow_mut().push(EntityAction::AddEffect { target: ActionTarget::Attacker, effect: grant.effect.to_string(), amplifier: grant.amplifier, duration: grant.duration });
        }
        // Lacerating: a melee hit puts the bleeding effect on the mob, at the hit's looting.
        if let Ok(Some(grant)) = with_held_tool(held, |env, stack| tool_stack::lacerating_hit_grant(&stack.custom_data, env, &hit.armor)) {
            hit.effects.borrow_mut().push(scaled_on(EffectGrant { effect: grant.effect.to_string(), amplifier: grant.amplifier, duration: grant.duration }, hit.target_state.as_ref()));
        }
        // Enderclearance: a hit may teleport the mob it landed on to a random place within the
        // module's diameter of where it stands. The original tries up to its teleport chances for
        // a spot the mob fits; the port's teleport names one destination, so one random spot in
        // the diameter is named.
        if let (Ok(pairs), Some(origin)) = (with_held_tool(held, |env, stack| tool_stack::enderclearance(&stack.custom_data, env)), hit.target_position) {
            for (chance, diameter) in pairs {
                if crate::tool_damage_hook::random() < chance {
                    let angle = crate::tool_damage_hook::random() * std::f32::consts::TAU;
                    let radius = crate::tool_damage_hook::random() * diameter / 2.0;
                    let to = [origin[0] + f64::from(radius * angle.cos()), origin[1], origin[2] + f64::from(radius * angle.sin())];
                    hit.actions.borrow_mut().push(EntityAction::Teleport { target: ActionTarget::Victim, position: to });
                }
            }
        }
        // Channeling (melee form): a hit may call lightning down on the mob, at the chance for the
        // weather over the attacker (rain or thunder count only under open sky, which the reported
        // rain exposure stands for). The shipped module forbids melee, so this runs only for data
        // that allows it; the launcher half is the projectile side's.
        if let Ok(chances) = with_held_tool(held, |env, stack| tool_stack::channeling_melee(&stack.custom_data, env)) {
            for (clear, rain, thunder) in chances {
                let chance = match hit.attacker_state.as_ref() {
                    Some(state) if state.rain_exposed && state.thundering => thunder,
                    Some(state) if state.rain_exposed => rain,
                    _ => clear,
                };
                if crate::tool_damage_hook::random() < chance {
                    hit.actions.borrow_mut().push(EntityAction::Summon { target: ActionTarget::Victim, kind: "minecraft:lightning_bolt".into() });
                }
            }
        }
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
            hit.effects.borrow_mut().push(scaled_on(EffectGrant { effect: grant.effect.to_string(), amplifier: grant.amplifier, duration: grant.duration }, hit.target_state.as_ref()));
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn counter_damage_accepts_the_game_and_the_port_s_own_damage_types() {
        assert!(counter_damage_type_allowed(&Identifier::parse("minecraft:thorns").unwrap()));
        assert!(counter_damage_type_allowed(&Identifier::parse("tconstruct:shock").unwrap()));
        assert!(counter_damage_type_allowed(&Identifier::parse("tconstruct:smeltery_heat").unwrap()));
        assert!(!counter_damage_type_allowed(&Identifier::parse("other:shock").unwrap()), "a type no side registered stays dropped");
    }

    struct Always;
    impl crate::materials::Conditions for Always {
        type Parsed = ();
        fn decode(&self, _: &serde_json::Value) -> Result<(), String> { Ok(()) }
        fn test(&self, _: &()) -> Result<bool, String> { Ok(true) }
    }

    #[test]
    fn shock_thorns_hurt_a_rain_exposed_attacker_through_the_worn_piece() {
        // The shipped shock modifier's second thorns, without its tool tag: the consumer
        // already iterates worn armor pieces, and a tool-tag predicate needs an item the
        // test stack does not carry, so it would never match here.
        let mut registry = crate::modifiers::Registry::default();
        registry.reload(&[(Identifier::parse("test:shock").unwrap(), json!({"modules": [{"type": "tconstruct:thorns", "attacker": "mantle:raining_at", "chance": {"each_level": 0.1}, "constant_damage": {"flat": 2.0}, "damage_type": "tconstruct:shock", "random_damage": {"flat": 3.0}}]}))], &Always);
        let mut materials = crate::materials::Definitions::default();
        materials.server_loaded = true;
        let data = crate::tool_definitions::Data::default();
        let env = crate::tool_stack::Environment { definition: Some(&data), has_materials: false, materials: &materials, modifiers: &registry, item_tags_loaded: true, item: None, item_tags: None };
        let mut nbt = pluto_lib::NbtCompound::new();
        nbt.insert(crate::tool_stack::TAG_MODIFIERS.into(), crate::modifier_entries::List::default().with_modifier(Identifier::parse("test:shock").unwrap(), 1).unwrap().to_nbt());
        let rain = pluto_lib::EntityState { rain_exposed: true, ..Default::default() };
        let in_rain = |tag: &Identifier| crate::melee_damage::state_in_tag(tag, Some(&rain), None);
        assert_eq!(crate::tool_stack::thorns_hurts(&nbt, &env, &in_rain, &mut || 0.0), vec![crate::thorns::Hurt { damage_type: Identifier::parse("tconstruct:shock").unwrap(), amount: 2.0 }]);
        let dry = pluto_lib::EntityState::default();
        let in_dry = |tag: &Identifier| crate::melee_damage::state_in_tag(tag, Some(&dry), None);
        assert!(crate::tool_stack::thorns_hurts(&nbt, &env, &in_dry, &mut || 0.0).is_empty(), "no rain, no shock");
    }
}
