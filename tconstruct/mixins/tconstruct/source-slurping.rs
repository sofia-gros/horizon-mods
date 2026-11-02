// Held-tool slurping for source mob-effect/add and mob-effect/set definitions.
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
use std::collections::{HashMap, HashSet};

pub(super) fn passive_item(stack: &ItemStack) -> bool {
    matches!(&*stack.item, "tconstruct:longbow" | "tconstruct:crossbow" | "tconstruct:battlesign" | "tconstruct:travelers_shield" | "tconstruct:plate_shield")
}

enum PlayerAction {
    Mob(&'static str, i32, i32),
    Remove(&'static str),
    Hunger(i32, f32),
}
pub(super) struct Plan {
    effects: Vec<(PlayerAction, u32, ItemStack)>,
}

pub(super) fn raw_recipe(game: &super::Game<'_>, fluid_id: &str) -> Option<serde_json::Value> {
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
    super::tconstruct_source_fluid_effects::select(layers, fluid_id, fluid_tags, &|conditions|
        super::tconstruct_source_conditions::conditional_conditions_with_mods(conditions, &registries, config, known, presence.as_ref()))?

}

pub(super) fn recipe(game: &super::Game<'_>, fluid_id: &str) -> Option<super::tconstruct_source_fluid_effects::Effects> {
    super::tconstruct_source_fluid_effects::parse_entity(&raw_recipe(game, fluid_id)?)
}

pub(super) fn tagged_recipe(game: &super::Game<'_>, fluid_id: &str, tag: Option<&Tag>) -> Option<super::tconstruct_source_fluid_effects::Effects> {
    super::tconstruct_source_fluid_effects::parse_tagged_entity(&raw_recipe(game, fluid_id)?, tag)
}

pub(super) fn plan(game: &super::Game<'_>, stack: &ItemStack) -> Option<Plan> {
    plan_for(game, stack, "tconstruct:slurping", None)
}

/// Resolve the same player effect against another held-fluid ability.
pub(super) fn plan_for(game: &super::Game<'_>, stack: &ItemStack, modifier: &str, override_level: Option<i32>) -> Option<Plan> {
    if stack.count != 1 || stack.wire_patch().is_some() { return None; }
    let custom = stack.data()?.other("minecraft:custom_data")?;
    if matches!(custom.get("tic_broken"), Some(Tag::Byte(v)) if *v != 0) { return None; }
    let Tag::Int(installed) = custom.get("tic_upgrades")?.get(modifier)? else { return None; };
    if *installed <= 0 { return None; }
    let level = override_level.unwrap_or(*installed);
    if level <= 0 { return None; }
    let capacity = match custom.get("tic_tank_capacity")? { Tag::Int(n) if *n > 0 => *n as u32, _ => return None };
    let fluid = super::pluto_stack_fluid::read(stack, "tic_persistent", "tconstruct:tank_fluid")??;
    if fluid.amount > capacity || ((game.lan.is_some() || game.lan_acting_uuid.is_some()) && fluid.tag.is_some()) { return None; }
    let recipe = tagged_recipe(game, &fluid.id, fluid.tag.as_ref())?;
    if recipe.entity.is_empty() { return None; }
    let strength = (level as f32).min(fluid.amount as f32 / recipe.amount as f32);
    if strength <= 0.0 { return None; }
    let mut effects = Vec::new();
    let mut targeted = HashSet::new();
    let mut hunger_seen = false;
    for effect in recipe.entity {
        let (action, used_scale) = match effect {
            super::tconstruct_source_fluid_effects::EntityEffect::Mob(effect) => {
                let known = horizon_client::brewing::effect(&effect.id)?;
                if !targeted.insert(known.id) { return None; }
                let existing = game.potions.effects.get(known.id).map(|current| (current.duration, current.amplifier));
                let Some((duration, used_scale)) = super::tconstruct_source_fluid_effects::mob_duration(&effect, strength, level as f32, existing) else { continue; };
                (PlayerAction::Mob(known.id, duration, effect.amplifier), used_scale)
            }
            super::tconstruct_source_fluid_effects::EntityEffect::Remove(id) => {
                let known = horizon_client::brewing::effect(&id)?;
                if !targeted.insert(known.id) { return None; }
                if strength < 1.0 || game.potions.effects.get(known.id).is_none() { continue; }
                (PlayerAction::Remove(known.id), 1.0)
            }
            super::tconstruct_source_fluid_effects::EntityEffect::Push { .. } => return None,
            super::tconstruct_source_fluid_effects::EntityEffect::CureMilk | super::tconstruct_source_fluid_effects::EntityEffect::Calcified => return None,
            super::tconstruct_source_fluid_effects::EntityEffect::Hunger { hunger, saturation, always } => {
                if hunger_seen { return None; } hunger_seen = true;
                let player = game.player.as_ref()?;
                if !always && !game.creative && player.health.food.food >= 20 { continue; }
                (PlayerAction::Hunger((hunger as f32 * strength) as i32, saturation), strength)
            }
        };
        let used = ((recipe.amount as f32 * used_scale.min(strength)).ceil() as u32).min(fluid.amount);
        if used == 0 { continue; }
        let mut remainder = fluid.clone(); remainder.amount -= used;
        let drained = super::pluto_stack_fluid::write(stack, "tic_persistent", "tconstruct:tank_fluid", (remainder.amount > 0).then_some(&remainder), capacity)?;
        effects.push((action, used, drained));
    }
    if effects.is_empty() { return None; }
    Some(Plan { effects })
}

pub(super) fn finish(game: &mut super::Game<'_>, stack: &ItemStack, result: &mut Option<ItemStack>) {
    let consume = !game.creative;
    let _ = finish_for(game, stack, result, "tconstruct:slurping", None, consume);
}

pub(super) fn finish_for(game: &mut super::Game<'_>, stack: &ItemStack, result: &mut Option<ItemStack>, modifier: &str, override_level: Option<i32>, consume: bool) -> bool {
    if game.remote.is_some() || !result.as_ref().is_some_and(|held| held.damage == stack.damage && held.same_item_same_components(stack)) { return false; }
    let Some(plan) = plan_for(game, stack, modifier, override_level) else { return false; };
    let mut successful: Option<(u32, ItemStack)> = None;
    for (action, used, drained) in plan.effects {
        let applied = match action {
            PlayerAction::Mob(id, duration, amplifier) => game.add_player_effect(horizon_client::effects::EffectInstance::new(id, duration, amplifier)),
            PlayerAction::Remove(id) => {
                let before = game.potions.effects.list.len();
                game.potions.effects.list.retain(|effect| effect.id != id);
                let changed = game.potions.effects.list.len() != before;
                if changed { game.apply_effect_status(); }
                changed
            }
            PlayerAction::Hunger(hunger, saturation) => {
                game.apply_effect_action(horizon_client::effects::EffectAction::Eat(hunger, saturation)); true
            }
        };
        if applied && successful.as_ref().is_none_or(|(prior, _)| used > *prior) {
            successful = Some((used, drained));
        }
    }
    let applied = successful.is_some();
    if consume { if let Some((_, drained)) = successful { *result = Some(drained); } }
    applied
}
