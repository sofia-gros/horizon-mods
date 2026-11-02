// Resolve Melter and Casting recipes from the current world's datapack and
// registry snapshots. Unknown conditions and ambiguous tags fail closed.
use std::collections::{HashMap, HashSet};
use super::tconstruct_source_melting_lookup::Catalog;
use super::tconstruct_source_melting_recipes::{CastingRecipe, FluidFuel, FoundryRecipe, MeltingRecipe};
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

pub(super) fn molding(world: &World, material: &str, pattern: Option<&str>) -> Option<(String, super::tconstruct_source_melting_recipes::MoldingRecipe)> {
    with_catalog(world, |catalog, registries, config| {
        catalog.molding(material, pattern, &|conditions| super::tconstruct_tx::source_conditions_match(conditions, registries, config).unwrap_or(false))
    })?
}

pub(super) fn melting(world: &World, item: &str) -> Option<(String, MeltingRecipe, String)> {
    with_catalog(world, |catalog, registries, config| {
        catalog.melting(item, &|conditions| super::tconstruct_tx::source_conditions_match(conditions, registries, config).unwrap_or(false))
    })?
}

pub(super) fn foundry_melting(world: &World, item: &str) -> Option<(String, FoundryRecipe, String, Vec<(String, u32)>)> {
    with_catalog(world, |catalog, registries, config| {
        catalog.foundry_melting(item, &|conditions| super::tconstruct_tx::source_conditions_match(conditions, registries, config).unwrap_or(false))
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

/// Resolve the completed stack before reserving fluid. Both normal item
/// casting and material casting then use the same persistent transaction.
pub(super) fn casting_stack(world: &World, basin: bool, input: Option<&horizon_items::items::ItemStack>, fluid: &str) -> Option<(String, CastingRecipe, horizon_items::items::ItemStack)> {
    let input_id = input.map(|stack| &*stack.item);
    if let Some((id, recipe, output)) = casting(world, basin, input_id, fluid) {
        return Some((id, recipe, horizon_items::items::ItemStack::new(&output, 1)));
    }
    let stack = input?;
    let lantern = matches!(&*stack.item, "tconstruct:seared_lantern" | "tconstruct:scorched_lantern");
    if (basin || lantern) && stack.count == 1 && stack.damage == 0 {
        if let Some(capacity) = super::tconstruct_source_smeltery_fuel::item_capacity(&stack.item) {
            use horizon_world::nbt::{Compound, Tag};
            let custom = stack.data().and_then(|data| data.other("minecraft:custom_data"));
            let mut data = match custom { None => Compound::new(), Some(Tag::Compound(data)) => data.clone(), _ => return None };
            let mut tank = match data.get("tconstruct:tank") {
                Some(tag) => super::tconstruct_source_melter_state::Tank::from_nbt(tag, capacity)?,
                None => super::tconstruct_source_melter_state::Tank::new(capacity),
            };
            if tank.fluid.as_deref().is_some_and(|existing| existing != fluid) { return None; }
            let free = capacity.saturating_sub(tank.amount);
            if let Some((id, recipe)) = with_catalog(world, |catalog, registries, config| {
                catalog.portable_tank_filling(basin, &stack.item, fluid, free, &|conditions| super::tconstruct_tx::source_conditions_match(conditions, registries, config).unwrap_or(false))
            })? {
                if !tank.insert(fluid, recipe.fluid.amount) { return None; }
                data.insert("tconstruct:tank".into(), tank.to_nbt());
                let mut output = stack.clone();
                output.data_mut().set_other("minecraft:custom_data", Some(Tag::Compound(data)));
                return Some((id, recipe, output));
            }
        }
    }
    if !basin {
        // Forge FluidBucketWrapper replaces the stack with a new filled bucket.
        if stack.item.as_ref() == "minecraft:bucket" && stack.damage == 0 && stack.count == 1 {
            if let Some((id, recipe, output)) = with_catalog(world, |catalog, registries, config| {
                catalog.bucket_filling(fluid, &|conditions| super::tconstruct_tx::source_conditions_match(conditions, registries, config).unwrap_or(false))
            })? {
                return Some((id, recipe, horizon_items::items::ItemStack::new(&output, 1)));
            }
        }
        if let Some((id, recipe)) = with_catalog(world, |catalog, registries, config| {
            catalog.duplication(&stack.item, fluid, &|conditions| super::tconstruct_tx::source_conditions_match(conditions, registries, config).unwrap_or(false))
        })? {
            let mut output = stack.clone();
            output.count = 1;
            return Some((id, recipe, output));
        }
    }
    let material_tags = &world.chunks.__horizon_mixins_tconstruct_resolved_material_definitions().as_ref()?.as_ref().ok()?.2;
    let supports_checked = |material: &str, output: &str| super::tconstruct_source_builder_world::supported_material(world, material)
        .ok().map(|(_, parts)| parts.contains(output));
    let supports = |material: &str, output: &str| supports_checked(material, output).unwrap_or(false);
    let material = stack.data().and_then(|data| data.other("minecraft:custom_data"))
        .and_then(|tag| tag.get("Material"))
        .and_then(|tag| match tag { horizon_world::nbt::Tag::String(id) => Some(id.to_string_lossy().to_string()), _ => None });
    let resolved = with_catalog(world, |catalog, registries, config| {
        let conditions = |values: &[serde_json::Value]| super::tconstruct_tx::source_conditions_match(values, registries, config).unwrap_or(false);
        let allowed = |value: Option<&serde_json::Value>, material: &str| {
            let extra = |kind: &str, value: &serde_json::Value, material: &str| -> Option<bool> {
                let base = material.split_once('#').map_or(material, |(base, _)| base);
                let defs = &world.chunks.__horizon_mixins_tconstruct_resolved_material_definitions().as_ref()?.as_ref().ok()?.0;
                let stats = world.chunks.__horizon_mixins_tconstruct_resolved_material_stats().as_ref()?.as_ref().ok()?;
                match kind {
                    "tconstruct:registered" => Some(defs.contains_key(base)),
                    "tconstruct:craftable" => Some(defs.get(base)?.0),
                    "tconstruct:not_hidden" => Some(!defs.get(base)?.1),
                    "tconstruct:stat_type" => Some(stats.get(base)?.contains_key(value.get("stat_type")?.as_str()?)),
                    "tconstruct:tier" => {
                        let tier = defs.get(base)?.2 as i64; let range = value.get("tier")?;
                        let (min,max) = if let Some(n) = range.as_i64() { (n,n) } else {
                            let map = range.as_object()?;
                            (match map.get("min") { Some(v) => v.as_i64()?, None => 0 },
                             match map.get("max") { Some(v) => v.as_i64()?, None => i32::MAX as i64 })
                        };
                        if min < 0 || max > i32::MAX as i64 || min > max { return None; }
                        Some((min..=max).contains(&tier))
                    },
                    "tconstruct:castable" | "tconstruct:composite" => {
                        let matches = |output: &str| {
                            let (wanted_base, wanted_variant) = material.split_once('#').unwrap_or((material, ""));
                            let (output_base, output_variant) = output.split_once('#').unwrap_or((output, ""));
                            wanted_base == output_base && (wanted_variant.is_empty() || wanted_variant == output_variant || wanted_variant == "default" && output_variant.is_empty())
                        };
                        Some(catalog.recipes.values().any(|bytes| {
                            if kind == "tconstruct:castable" {
                                super::tconstruct_source_melting_recipes::MaterialFluidRecipe::parse(bytes, &conditions).is_some_and(|r| matches(&r.material))
                            } else {
                                super::tconstruct_source_melting_recipes::CompositeFluidRecipe::parse(bytes, &conditions).is_some_and(|r| matches(&r.material))
                            }
                        }))
                    },
                    _ => None,
                }
            };
            super::tconstruct_source_melting_lookup::material_predicate(value, material,
                &|tag, id| material_tags.get(tag).is_some_and(|values| values.contains(id)), &supports_checked, &extra).unwrap_or(false)
        };
        let composite = material.as_deref().and_then(|material| catalog.composite_casting(basin, &stack.item, material, fluid,
            &supports, &allowed,
            &|material, stat| {
                let stats = world.chunks.__horizon_mixins_tconstruct_resolved_material_stats().as_ref()?.as_ref().ok()?;
                let base = material.split_once('#').map_or(material, |(base, _)| base);
                Some(stats.get(base)?.contains_key(stat))
            }, &conditions));
        composite.or_else(|| if basin { None } else { catalog.material_casting(&stack.item, fluid, &supports, &allowed, &conditions) })
    })??;
    let (id, recipe, output, material) = resolved;
    let mut output_stack = horizon_items::items::ItemStack::new(&output, 1);
    output_stack.data_mut().set_other("minecraft:custom_data", Some(horizon_world::nbt::Tag::compound([
        ("Material", horizon_world::nbt::Tag::String(material.as_str().into()))
    ])));
    Some((id, recipe, output_stack))
}
