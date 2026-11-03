// Resolve Melter and Casting recipes from the current world's datapack and
// registry snapshots. Unknown conditions and ambiguous tags fail closed.
use std::collections::{HashMap, HashSet};
use super::tconstruct_source_melting_lookup::Catalog;
use super::tconstruct_source_melting_recipes::{CastingRecipe, FluidFuel, FoundryRecipe, MeltingRecipe};
use super::tconstruct_world::World;

type Tags = HashMap<String, HashSet<String>>;

// Resolve conditional recipes in source order. An unknown condition stops
// selection: falling through could select a different recipe than Forge.
fn conditional_test(value: &serde_json::Value, registries: &HashMap<String, Tags>, config: &HashMap<String, bool>, items: &HashSet<String>) -> Option<bool> {
    match value.get("type")?.as_str()? {
        "forge:true" => Some(true),
        "forge:false" => Some(false),
        "forge:item_exists" => Some(items.contains(value.get("item")?.as_str()?)),
        "forge:not" => Some(!conditional_test(value.get("value")?, registries, config, items)?),
        kind @ ("forge:and" | "forge:or") => {
            let values = value.get("values")?.as_array()?;
            if values.is_empty() { return None; }
            let mut unknown = false;
            for child in values {
                match conditional_test(child, registries, config, items) {
                    Some(result) if (kind == "forge:and" && !result) || (kind == "forge:or" && result) => return Some(result),
                    Some(_) => {}, None => unknown = true,
                }
            }
            if unknown { None } else { Some(kind == "forge:and") }
        }
        _ => super::tconstruct_tx::source_conditions_match(std::slice::from_ref(value), registries, config),
    }
}

pub(super) fn conditional_conditions(values: &[serde_json::Value], registries: &HashMap<String, Tags>, config: &HashMap<String, bool>, items: &HashSet<String>) -> Option<bool> {
    for value in values { if !conditional_test(value, registries, config, items)? { return Some(false); } }
    Some(true)
}

pub(super) fn conditional_recipe(bytes: &[u8], registries: &HashMap<String, Tags>, config: &HashMap<String, bool>, items: &HashSet<String>) -> Option<Vec<u8>> {
    // Most entries are ordinary recipes; keep their exact bytes untouched.
    if !bytes.windows(b"forge:conditional".len()).any(|part| part == b"forge:conditional") { return Some(bytes.to_vec()); }
    let mut value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    for _ in 0..16 {
        if value.get("type").and_then(serde_json::Value::as_str) != Some("forge:conditional") {
            return serde_json::to_vec(&value).ok();
        }
        match value.get("conditions") {
            None => {},
            Some(serde_json::Value::Array(conditions)) if conditional_conditions(conditions, registries, config, items)? => {},
            _ => return None,
        }
        let branches = value.get("recipes")?.as_array()?;
        let mut selected = None;
        for branch in branches {
            let conditions = branch.get("conditions")?.as_array()?;
            let mut matches = true;
            for condition in conditions {
                if !conditional_test(condition, registries, config, items)? { matches = false; break; }
            }
            if matches { selected = Some(branch.get("recipe")?.clone()); break; }
        }
        value = selected?;
    }
    None
}

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
        catalog.melting(item, &|conditions| conditional_conditions(conditions, registries, config, catalog.known_items).unwrap_or(false))
    })?
}

pub(super) fn foundry_melting(world: &World, item: &str) -> Option<(String, FoundryRecipe, String, Vec<(String, u32)>)> {
    with_catalog(world, |catalog, registries, config| {
        catalog.foundry_melting(item, &|conditions| conditional_conditions(conditions, registries, config, catalog.known_items).unwrap_or(false))
    })?
}

pub(super) fn material_melting(world: &World, stack: &horizon_items::items::ItemStack)
    -> Option<(String, FoundryRecipe, String, Vec<(String, u32)>)> {
    let tags = world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    if !tags.get("tconstruct:parts")?.contains(stack.item.as_ref()) { return None; }
    let material = super::tconstruct_source_builder_world::lookup_material(world, stack).ok()??;
    let cost = u32::try_from(material.value).ok()?;
    super::tconstruct_source_builder_world::supported_material(world, &material.material).ok()?;
    with_catalog(world, |catalog, registries, config| {
        catalog.material_melting(&stack.item, &material.material, cost, &|conditions| conditional_conditions(conditions, registries, config, catalog.known_items).unwrap_or(false))
    })?
}

pub(super) fn damaged_melting(world: &World, item: &str, damage: u32, maximum: u32)
    -> Option<(String, FoundryRecipe, String, Vec<(String, u32)>)> {
    with_catalog(world, |catalog, registries, config| {
        catalog.damaged_melting(item, damage, maximum, &|conditions| conditional_conditions(conditions, registries, config, catalog.known_items).unwrap_or(false))
    })?
}

pub(super) fn alloy(world: &World, fluids: &[(String, u32)], capacity: u32, temperature: u32)
    -> Option<(String, Vec<(String, u32)>, String, u32)> {
    with_catalog(world, |catalog, registries, config| {
        catalog.alloy(fluids, capacity, temperature, &|conditions| conditional_conditions(conditions, registries, config, catalog.known_items).unwrap_or(false))
    })?
}

pub(super) fn fluid_fuel(world: &World, fluid: &str, available: u32) -> Option<FluidFuel> {
    with_catalog(world, |catalog, registries, config| {
        catalog.fluid_fuel(fluid, available, &|conditions| conditional_conditions(conditions, registries, config, catalog.known_items).unwrap_or(false))
    })?
}

pub(super) fn casting(world: &World, basin: bool, input: Option<&str>, fluid: &str) -> Option<(String, CastingRecipe, String)> {
    with_catalog(world, |catalog, registries, config| {
        catalog.casting(basin, input, fluid, &|conditions| conditional_conditions(conditions, registries, config, catalog.known_items).unwrap_or(false))
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
    let material_tags = &world.chunks.__horizon_mixins_tconstruct_resolved_material_definitions().as_ref()?.as_ref().ok()?.2;
    let supports_checked = |material: &str, output: &str| super::tconstruct_source_builder_world::supported_material(world, material)
        .ok().map(|(_, parts)| parts.contains(output));
    let supports = |material: &str, output: &str| supports_checked(material, output).unwrap_or(false);
    let material = stack.data().and_then(|data| data.other("minecraft:custom_data"))
        .and_then(|tag| tag.get("Material"))
        .and_then(|tag| match tag { horizon_world::nbt::Tag::String(id) => Some(id.to_string_lossy().to_string()), _ => None });
    let resolved = with_catalog(world, |catalog, registries, config| {
        let conditions = |values: &[serde_json::Value]| conditional_conditions(values, registries, config, catalog.known_items).unwrap_or(false);
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

#[cfg(test)]
mod conditional_tests {
    use super::*;
    #[test]
    fn source_order_and_unknown_do_not_choose_wrong_fallback() {
        let tags = HashMap::new();
        let mut config = HashMap::new();
        config.insert("cheaper_netherite_alloy".into(), true);
        let items = HashSet::new();
        let bytes = br#"{"type":"forge:conditional","recipes":[{"conditions":[{"type":"tconstruct:config","prop":"cheaper_netherite_alloy"}],"recipe":{"type":"tconstruct:alloy","choice":1}},{"conditions":[{"type":"forge:true"}],"recipe":{"type":"tconstruct:alloy","choice":2}}]}"#;
        let selected = conditional_recipe(bytes, &tags, &config, &items).unwrap();
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&selected).unwrap()["choice"], 1);
        config.insert("cheaper_netherite_alloy".into(), false);
        let selected = conditional_recipe(bytes, &tags, &config, &items).unwrap();
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&selected).unwrap()["choice"], 2);
        config.clear();
        assert!(conditional_recipe(bytes, &tags, &config, &items).is_none());
        let outer = br#"{"type":"forge:conditional","conditions":[{"type":"tconstruct:config","prop":"cheaper_netherite_alloy"}],"recipes":[{"conditions":[{"type":"forge:true"}],"recipe":{"type":"tconstruct:alloy"}}]}"#;
        assert!(conditional_recipe(outer, &tags, &config, &items).is_none());
        config.insert("cheaper_netherite_alloy".into(), false);
        assert!(conditional_recipe(outer, &tags, &config, &items).is_none());
        config.insert("cheaper_netherite_alloy".into(), true);
        assert!(conditional_recipe(outer, &tags, &config, &items).is_some());
    }
}
