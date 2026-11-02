//! Held-tool slurping for source mob-effect/add and mob-effect/set definitions.
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
use std::collections::{HashMap, HashSet};

pub(super) struct Plan {
    effects: Vec<(&'static str, i32, i32, u32, ItemStack)>,
}

pub(super) fn plan(game: &super::Game<'_>, stack: &ItemStack) -> Option<Plan> {
    if stack.count != 1 || stack.wire_patch().is_some() { return None; }
    if &*stack.item == "tconstruct:crossbow" { return None; }
    let custom = stack.data()?.other("minecraft:custom_data")?;
    if matches!(custom.get("tic_broken"), Some(Tag::Byte(v)) if *v != 0) { return None; }
    let Tag::Int(level) = custom.get("tic_upgrades")?.get("tconstruct:slurping")? else { return None; };
    if *level <= 0 { return None; }
    let capacity = match custom.get("tic_tank_capacity")? { Tag::Int(n) if *n > 0 => *n as u32, _ => return None };
    let fluid = super::pluto_stack_fluid::read(stack, "tic_persistent", "tconstruct:tank_fluid")??;
    if fluid.amount > capacity || ((game.lan.is_some() || game.lan_acting_uuid.is_some()) && fluid.tag.is_some()) { return None; }
    let chunks = &game.world.chunks;
    let layers = chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()?;
    let item_tags = chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    let (_, fluid_tags) = chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref()?.as_ref().ok()?;
    let config = chunks.__horizon_mixins_tconstruct_source_config_snapshot().as_ref()?.as_ref().ok()?;
    let known = chunks.__horizon_mixins_tconstruct_melter_known_items().as_ref()?;
    let registries: HashMap<String, HashMap<String, HashSet<String>>> = [
        ("minecraft:item".into(), item_tags.clone()), ("minecraft:fluid".into(), fluid_tags.clone()),
    ].into_iter().collect();
    let presence = chunks.__horizon_mixins_tconstruct_recipe_mod_snapshot().as_ref()
        .map(|(states, complete)| super::tconstruct_source_conditions::ModPresence { states: states.clone(), complete: *complete });
    let recipe = super::tconstruct_source_fluid_effects::resolve(layers, &fluid.id, fluid_tags, &|conditions|
        super::tconstruct_source_conditions::conditional_conditions_with_mods(conditions, &registries, config, known, presence.as_ref()))??;
    if recipe.entity.is_empty() { return None; }
    let strength = (*level as f32).min(fluid.amount as f32 / recipe.amount as f32);
    if strength <= 0.0 { return None; }
    let mut effects = Vec::new();
    for effect in recipe.entity {
        let known = horizon_client::brewing::effect(&effect.id)?;
        let infinite = effect.time == -1;
        if infinite && strength < 1.0 { continue; }
        let extra = if infinite { -1 } else { (effect.time as f32 * strength) as i32 };
        if !infinite && extra <= 0 { continue; }
        let (duration, used_scale) = if infinite { (-1, 1.0) } else { match game.potions.effects.get(known.id) {
            Some(existing) if existing.amplifier > effect.amplifier => continue,
            Some(existing) if existing.amplifier == effect.amplifier && existing.duration < 0 => continue,
            Some(existing) if existing.amplifier == effect.amplifier => match effect.action {
                super::tconstruct_source_fluid_effects::TimeAction::Add => (existing.duration.checked_add(extra)?, strength),
                super::tconstruct_source_fluid_effects::TimeAction::Set => {
                    let previous = existing.duration as f32 / effect.time as f32;
                    let effective = (strength + previous).min(*level as f32);
                    if effective < previous { continue; }
                    ((effect.time as f32 * effective) as i32, effective - previous)
                }
            },
            _ => (extra, strength),
        }};
        let used = ((recipe.amount as f32 * used_scale.min(strength)).ceil() as u32).min(fluid.amount);
        if duration == 0 || used == 0 { continue; }
        let mut remainder = fluid.clone(); remainder.amount -= used;
        let drained = super::pluto_stack_fluid::write(stack, "tic_persistent", "tconstruct:tank_fluid", (remainder.amount > 0).then_some(&remainder), capacity)?;
        effects.push((known.id, duration, effect.amplifier, used, drained));
    }
    if effects.is_empty() { return None; }
    Some(Plan { effects })
}

pub(super) fn finish(game: &mut super::Game<'_>, stack: &ItemStack, result: &mut Option<ItemStack>) {
    if game.remote.is_some() || !result.as_ref().is_some_and(|held| held.damage == stack.damage && held.same_item_same_components(stack)) { return; }
    let Some(plan) = plan(game, stack) else { return; };
    let mut successful: Option<(u32, ItemStack)> = None;
    for (id, duration, amplifier, used, drained) in plan.effects {
        if game.add_player_effect(horizon_client::effects::EffectInstance::new(id, duration, amplifier))
            && successful.as_ref().is_none_or(|(prior, _)| used > *prior) {
            successful = Some((used, drained));
        }
    }
    if !game.creative { if let Some((_, drained)) = successful { *result = Some(drained); } }
}
