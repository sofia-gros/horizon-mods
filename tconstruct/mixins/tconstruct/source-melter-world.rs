// Resolve Melter and Casting recipes from the current world's datapack and
// registry snapshots. Unknown conditions and ambiguous tags fail closed.
use std::collections::{HashMap, HashSet};
use super::tconstruct_source_melting_lookup::Catalog;
use super::tconstruct_source_melting_recipes::{CastingRecipe, FluidFuel, FoundryRecipe, MeltingRecipe};
use super::tconstruct_world::World;

type Tags = HashMap<String, HashSet<String>>;
pub(super) use super::tconstruct_source_conditions::{ModPresence, conditional_conditions, conditional_conditions_with_mods, conditional_recipe};

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

pub(super) fn molding(world: &World, basin: bool, material: &str, pattern: Option<&str>) -> Option<(String, super::tconstruct_source_melting_recipes::MoldingRecipe)> {
    with_catalog(world, |catalog, registries, config| {
        catalog.molding(basin, material, pattern, &|conditions| conditional_conditions(conditions, registries, config, catalog.known_items).unwrap_or(false))
    })?
}

fn ore_rates(world: &World, machine: &str) -> (Option<u32>, Option<u32>) {
    let rates = world.chunks.__horizon_mixins_tconstruct_source_ore_rates().as_ref().and_then(|value| value.as_ref().ok());
    (rates.and_then(|rates| rates.get(&format!("{machine}.nuggetsPerMetal")).copied()),
     rates.and_then(|rates| rates.get(&format!("{machine}.shardsPerGem")).copied()))
}

pub(super) fn melting(world: &World, item: &str) -> Option<(String, MeltingRecipe, String)> {
    with_catalog(world, |catalog, registries, config| {
        catalog.melting(item, &|conditions| conditional_conditions(conditions, registries, config, catalog.known_items).unwrap_or(false))
    })?
}

pub(super) fn melting_stack(world: &World, stack: &horizon_items::items::ItemStack, no_container: Option<bool>) -> Result<Option<(String, MeltingRecipe, String)>, ()> {
    melting_stack_for(world, stack, no_container, "melter")
}
pub(super) fn melting_stack_for(world: &World, stack: &horizon_items::items::ItemStack, no_container: Option<bool>, machine: &str) -> Result<Option<(String, MeltingRecipe, String)>, ()> {
    let rates = ore_rates(world, machine);
    with_catalog(world, |catalog, registries, config| {
        catalog.melting_with_rates(&stack.item, no_container, &|conditions| conditional_conditions(conditions, registries, config, catalog.known_items).unwrap_or(false), rates)
    }).ok_or(())?
}

pub(super) fn foundry_melting_stack(world: &World, stack: &horizon_items::items::ItemStack, no_container: Option<bool>) -> Result<Option<(String, FoundryRecipe, String, Vec<(String, u32)>)>, ()> {
    with_catalog(world, |catalog, registries, config| {
        catalog.foundry_melting_with_rates(&stack.item, no_container, &|conditions| conditional_conditions(conditions, registries, config, catalog.known_items).unwrap_or(false), ore_rates(world, "foundry"), ore_rates(world, "foundry_byproduct"))
    }).ok_or(())?
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

pub(super) fn alloy(world: &World, fluids: &[(String, u32)], tagged: &[String], capacity: u32, temperature: u32)
    -> Option<(String, Vec<(String, u32)>, String, u32)> {
    with_catalog(world, |catalog, registries, config| {
        catalog.alloy_except(fluids, tagged, capacity, temperature, &|conditions| conditional_conditions(conditions, registries, config, catalog.known_items).unwrap_or(false))
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

/// Resolve base potion and, on the local path, its custom color. Unsupported
/// potion metadata is rejected before reserving the fluid.
pub(super) fn potion_casting_stack(world: &World, basin: bool, input: Option<&horizon_items::items::ItemStack>, fluid: &str, tag: &horizon_world::nbt::Tag, allow_color: bool)
    -> Option<(String, CastingRecipe, horizon_items::items::ItemStack)> {
    let (potion, color, effects) = super::tconstruct_source_fluid_container::potion_tag(tag, allow_color)?;
    let input = input?;
    let (id, recipe, output) = with_catalog(world, |catalog, registries, config| {
        catalog.potion_casting(basin, &input.item, fluid, &|conditions| conditional_conditions(conditions, registries, config, catalog.known_items).unwrap_or(false))
    })??;
    if !matches!(output.as_str(), "minecraft:potion" | "minecraft:splash_potion" | "minecraft:lingering_potion" | "minecraft:tipped_arrow") { return None; }
    Some((id, recipe, super::tconstruct_source_fluid_container::potion_stack(&output, &potion, color, effects)))
}

/// Resolve the completed stack before reserving fluid. Both normal item
/// casting and material casting then use the same persistent transaction.
pub(super) fn casting_stack(world: &World, basin: bool, input: Option<&horizon_items::items::ItemStack>, fluid: &str) -> Option<(String, CastingRecipe, horizon_items::items::ItemStack)> {
    let input_id = input.map(|stack| &*stack.item);
    if let Some((id, recipe, output)) = casting(world, basin, input_id, fluid) {
        return Some((id, recipe, horizon_items::items::ItemStack::new(&output, 1)));
    }
    if let Some(stack) = input {
    let lantern = matches!(&*stack.item, "tconstruct:seared_lantern" | "tconstruct:scorched_lantern");
    if (basin || lantern) && stack.count == 1 && stack.damage == 0 {
        if let Some(capacity) = super::tconstruct_source_smeltery_fuel::item_capacity(&stack.item) {
            use horizon_world::nbt::{Compound, Tag};
            let custom = stack.data().and_then(|data| data.other("minecraft:custom_data"));
            let mut data = match custom { None => Compound::new(), Some(Tag::Compound(data)) => data.clone(), _ => return None };
            let mut tank = match data.get(&"tconstruct:tank".into()) {
                Some(tag) => super::tconstruct_source_melter_state::Tank::from_nbt(tag, capacity)?,
                None => super::tconstruct_source_melter_state::Tank::new(capacity),
            };
            if tank.fluid.as_deref().is_some_and(|existing| existing != fluid) { return None; }
            let free = capacity.saturating_sub(tank.amount);
            if let Some((id, recipe)) = with_catalog(world, |catalog, registries, config| {
                catalog.portable_tank_filling(basin, &stack.item, fluid, free, &|conditions| conditional_conditions(conditions, registries, config, catalog.known_items).unwrap_or(false))
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
                catalog.bucket_filling(fluid, &|conditions| conditional_conditions(conditions, registries, config, catalog.known_items).unwrap_or(false))
            })? {
                return Some((id, recipe, horizon_items::items::ItemStack::new(&output, 1)));
            }
        }
        if stack.count == 1 && super::tconstruct_source_fluid_container::can_fluid(stack) == Some(None) {
            if let Some((id, recipe)) = with_catalog(world, |catalog, registries, config| {
                catalog.can_filling(fluid, &|conditions| conditional_conditions(conditions, registries, config, catalog.known_items).unwrap_or(false))
            })? {
                let output = super::tconstruct_source_fluid_container::can_set_fluid(stack, Some(fluid))?;
                return Some((id, recipe, output));
            }
        }
    }
    if let Some((id, recipe)) = with_catalog(world, |catalog, registries, config| {
        catalog.duplication(basin, &stack.item, fluid, &|conditions| conditional_conditions(conditions, registries, config, catalog.known_items).unwrap_or(false))
    })? {
        let mut output = stack.clone();
        output.count = 1;
        return Some((id, recipe, output));
    }
    // The source rebuilds and validates a tool when metal replaces one of its
    // materials. Ordinary casts may repair; composite casts do not. Supported
    // upgrades and material traits are rebuilt through the Station path.
    if stack.count == 1 && stack.item.starts_with("tconstruct:") {
        let candidate = (|| {
            use horizon_world::nbt::Tag;
            let custom = stack.data()?.other("minecraft:custom_data")?;
            match custom.get("tic_upgrades") {
                None => {},
                Some(Tag::Compound(values)) if values.iter().all(|(id, value)| match id.to_string_lossy().as_str() {
                    "tconstruct:diamond" | "tconstruct:emerald" => matches!(value, Tag::Int(1)),
                    "tconstruct:haste" | "tconstruct:sharpness" | "tconstruct:swiftstrike"
                        | "tconstruct:quick_charge" | "tconstruct:trueshot" => matches!(value, Tag::Int(v) if *v > 0),
                    _ => false,
                }) => {},
                _ => return None,
            }
            match custom.get("tic_broken") { None | Some(Tag::Byte(0)) => {}, _ => return None }
            let old_max = match custom.get("tic_max_damage") { Some(Tag::Int(value)) if *value > 0 => *value as u32, _ => return None };
            if stack.damage >= old_max { return None; }
            let materials = world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()?;
            let (namespace, name) = stack.item.split_once(':')?;
            let definition = materials.get(&format!("{namespace}:tinkering/tool_definitions/{name}"))?.last()?;
            let building = world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref()?.values()
                .filter_map(|bytes| super::tconstruct_source_station_recipes::BuildingRecipe::parse(bytes))
                .find(|recipe| recipe.result == &*stack.item)?;
            let parts = building.parts(definition)?;
            let Tag::List(values) = custom.get("tic_materials")? else { return None; };
            let identities: Vec<String> = values.iter().map(|value| match value { Tag::String(id) => Some(id.to_string_lossy().to_string()), _ => None }).collect::<Option<Vec<String>>>()?;
            if identities.len() != parts.len() { return None; }
            let ((id, recipe, material, index), composite) = with_catalog(world, |catalog, registries, config| {
                let supports = |index: usize, material: &str| {
                    let Some(part) = parts.get(index) else { return false; };
                    let Some(old) = identities.get(index) else { return false; };
                    let canonical = |id: &str| id.strip_suffix("#default").unwrap_or(id).to_owned();
                    canonical(old.as_str()) != canonical(material) && super::tconstruct_source_builder_world::supported_material(world, material)
                        .is_ok_and(|(_, supported)| supported.contains(part))
                };
                let conditions = |values: &[serde_json::Value]| conditional_conditions(values, registries, config, catalog.known_items).unwrap_or(false);
                catalog.part_swap_casting(basin, &stack.item, fluid, &supports, &conditions).map(|value| (value, false))
                    .or_else(|| catalog.composite_part_swap(basin, &stack.item, fluid, &identities, &supports, &conditions).map(|value| (value, true)))
            })??;
            let mut next = identities; *next.get_mut(index)? = material;
            let mut output = super::tconstruct_source_station_world::rebuild_material_swap(world, definition, stack, &next, 0, true, &mut None)?;
            if !composite && stack.damage != 0 {
                // Only use hooks whose complete repair semantics are known.
                // Do not infer effects for unknown traits or definition traits.
                let definition_json: serde_json::Value = serde_json::from_slice(definition).ok()?;
                if definition_json.get("modules")?.as_array()?.iter().any(|module|
                    module.get("type").and_then(serde_json::Value::as_str) == Some("tconstruct:traits")) { return None; }
                let traits = super::tconstruct_source_material_traits::levels(world, &output)?.levels;
                if traits.keys().any(|id| !matches!(id.as_str(), "tconstruct:preserved" | "tconstruct:ductile" | "tconstruct:cultivated" | "tconstruct:dense")) { return None; }
                let level = |id: &str| traits.get(id).copied().unwrap_or(0);
                let emerald = matches!(custom.get("tic_upgrades").and_then(|v| v.get("tconstruct:emerald")), Some(Tag::Int(1)));
                // One active factor avoids changing the upstream hook order.
                if [level("tconstruct:preserved") > 0, level("tconstruct:cultivated") > 0,
                    level("tconstruct:dense") > 0, emerald].into_iter().filter(|v| *v).count() > 1 { return None; }
                let kind = super::tconstruct_source_station_stats::part_kind(parts.get(index)?)?;
                let durability = if matches!(kind, "head" | "limb" | "slime" | "skull") || kind.starts_with("plating_") {
                    let id = next.get(index)?;
                    let (namespace, name) = id.split_once(':')?;
                    let layers = materials.get(&format!("{namespace}:tinkering/materials/stats/{}", name.split('#').next()?))?;
                    let stats = super::tconstruct_tx::source_merge_material_stats(layers).ok()?;
                    stats.get(&format!("tconstruct:{kind}"))?.get("durability")?.as_i64()?.try_into().ok()?
                } else { 0i32 };
                let recipes = world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref()?;
                let swap = super::tconstruct_source_melting_recipes::PartSwapCastingRecipe::parse(recipes.get(&id)?, &|_| true)?;
                let factor = super::tconstruct_source_material_effects::repair_factor(level("tconstruct:cultivated"), level("tconstruct:preserved"))
                    * super::tconstruct_source_material_effects::dense_repair_factor(level("tconstruct:dense"))
                    * if emerald { 1.5 } else { 1.0 };
                let repair = (durability as f32 * (swap.item_cost as f32 / 3.0) * factor).max(0.0) as u32;
                super::tconstruct_tx::fixed_set_damage(&mut output, stack.damage.saturating_sub(repair));
            }
            let max_damage = match output.data()?.other("minecraft:custom_data")?.get("tic_max_damage")? {
                Tag::Int(value) if *value > 0 => *value as u32, _ => return None,
            };
            if output.damage >= max_damage { return None; }
            Some((id, recipe, output))
        })();
        if candidate.is_some() { return candidate; }
    }
    }
    let material_tags = &world.chunks.__horizon_mixins_tconstruct_resolved_material_definitions().as_ref()?.as_ref().ok()?.2;
    let supports_checked = |material: &str, output: &str| super::tconstruct_source_builder_world::supported_material(world, material)
        .ok().map(|(_, parts)| parts.contains(output));
    let supports = |material: &str, output: &str| supports_checked(material, output).unwrap_or(false);
    let material = input.and_then(|stack| stack.data()).and_then(|data| data.other("minecraft:custom_data"))
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
        let composite = input.zip(material.as_deref()).and_then(|(stack, material)| catalog.composite_casting(basin, &stack.item, material, fluid,
            &supports, &allowed,
            &|material, stat| {
                let stats = world.chunks.__horizon_mixins_tconstruct_resolved_material_stats().as_ref()?.as_ref().ok()?;
                let base = material.split_once('#').map_or(material, |(base, _)| base);
                Some(stats.get(base)?.contains_key(stat))
            }, &conditions));
        composite.or_else(|| catalog.material_casting(basin, input_id, fluid, &supports, &allowed, &conditions))
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
        let selected = conditional_recipe(bytes, &tags, &config, &items, None).unwrap();
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&selected).unwrap()["choice"], 1);
        config.insert("cheaper_netherite_alloy".into(), false);
        let selected = conditional_recipe(bytes, &tags, &config, &items, None).unwrap();
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&selected).unwrap()["choice"], 2);
        config.clear();
        assert!(conditional_recipe(bytes, &tags, &config, &items, None).is_none());
        let outer = br#"{"type":"forge:conditional","conditions":[{"type":"tconstruct:config","prop":"cheaper_netherite_alloy"}],"recipes":[{"conditions":[{"type":"forge:true"}],"recipe":{"type":"tconstruct:alloy"}}]}"#;
        assert!(conditional_recipe(outer, &tags, &config, &items, None).is_none());
        config.insert("cheaper_netherite_alloy".into(), false);
        assert!(conditional_recipe(outer, &tags, &config, &items, None).is_none());
        config.insert("cheaper_netherite_alloy".into(), true);
        assert!(conditional_recipe(outer, &tags, &config, &items, None).is_some());
    }
}
