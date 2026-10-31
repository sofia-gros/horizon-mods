// Resolve Melter and Casting recipes from the current world's datapack and
// registry snapshots. Unknown conditions and ambiguous tags fail closed.
use std::collections::{HashMap, HashSet};
use super::tconstruct_source_melting_lookup::Catalog;
use super::tconstruct_source_melting_recipes::{CastingRecipe, FluidFuel, MeltingRecipe};
use super::tconstruct_world::World;

type Tags = HashMap<String, HashSet<String>>;

fn with_catalog<R>(world: &World, f: impl FnOnce(&Catalog<'_>, &HashMap<String, Tags>, &HashMap<String, bool>) -> R) -> Option<R> {
    let chunks = &world.chunks;
    let recipes = chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref()?;
    let item_tags = chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    let (known_fluids, fluid_tags) = chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref()?.as_ref().ok()?;
    let known_items = chunks.__horizon_mixins_tconstruct_melter_known_items().as_ref()?;
    let config = chunks.__horizon_mixins_tconstruct_source_config_snapshot().as_ref()?.as_ref().ok()?;
    let registries = [("minecraft:item".to_owned(), item_tags.clone()), ("minecraft:fluid".to_owned(), fluid_tags.clone())].into_iter().collect();
    let catalog = Catalog { recipes, item_tags, fluid_tags, known_items, known_fluids };
    Some(f(&catalog, &registries, config))
}

pub(super) fn melting(world: &World, item: &str) -> Option<(String, MeltingRecipe, String)> {
    with_catalog(world, |catalog, registries, config| {
        catalog.melting(item, &|conditions| super::tconstruct_tx::source_conditions_match(conditions, registries, config).unwrap_or(false))
    })?
}

pub(super) fn alloy(world: &World, fluids: &[(String, u32)], capacity: u32, temperature: u32)
    -> Option<(String, Vec<(String, u32)>, String, u32)> {
    with_catalog(world, |catalog, registries, config| {
        catalog.alloy(fluids, capacity, temperature, &|conditions| super::tconstruct_tx::source_conditions_match(conditions, registries, config).unwrap_or(false))
    })?
}

pub(super) fn fluid_fuel(world: &World, fluid: &str, available: u32) -> Option<FluidFuel> {
    with_catalog(world, |catalog, registries, config| {
        catalog.fluid_fuel(fluid, available, &|conditions| super::tconstruct_tx::source_conditions_match(conditions, registries, config).unwrap_or(false))
    })?
}

pub(super) fn casting(world: &World, basin: bool, input: Option<&str>, fluid: &str) -> Option<(String, CastingRecipe, String)> {
    with_catalog(world, |catalog, registries, config| {
        catalog.casting(basin, input, fluid, &|conditions| super::tconstruct_tx::source_conditions_match(conditions, registries, config).unwrap_or(false))
    })?
}
