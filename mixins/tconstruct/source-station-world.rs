//! Station recipe selection using the current world's source data.
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
use super::tconstruct_source_station_recipes::{BuildingMatch, BuildingRecipe};

/// Resolve the registered pickaxe against source recipes and material stats.
pub(super) fn building_plan(world: &super::tconstruct_world::World, slots: &[Option<ItemStack>]) -> Option<(BuildingMatch, ItemStack)> {
    let recipes = world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref()?;
    let materials = world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()?;
    let definition = materials.get("tconstruct:tinkering/tool_definitions/pickaxe")?.last()?;
    for bytes in recipes.values() {
        let Some(recipe) = BuildingRecipe::parse(bytes) else { continue; };
        if recipe.result != "tconstruct:pickaxe" { continue; }
        let Some(parts) = recipe.parts(definition) else { continue; };
        let Some(inputs) = slots.get(..3) else { return None; };
        let Some(matched) = recipe.match_inputs(&parts, parts.len(), inputs, |stack| {
            match stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|tag| tag.get("Material")) {
                Some(Tag::String(material)) => Some(material.to_string_lossy().to_string()),
                _ => None,
            }
        }, |part, material| {
            super::tconstruct_source_builder_world::supported_material(world, material)
                .is_ok_and(|(_, supported)| supported.contains(part))
        }, |_, _| false) else { continue; };
        let mut stat_maps: Vec<serde_json::Map<String, serde_json::Value>> = Vec::new();
        for material in &matched.materials {
            let (namespace, id) = material.split_once(':')?;
            let id = id.split('#').next()?;
            let layers = materials.get(&format!("{namespace}:tinkering/materials/stats/{id}"))?;
            let merged = super::tconstruct_tx::source_merge_material_stats(layers).ok()?;
            stat_maps.push(merged.into_iter().map(|(key, fields)| (key, serde_json::Value::Object(fields))).collect());
        }
        let stats = super::tconstruct_source_station_stats::base_stats(definition, &stat_maps)?;
        let durability = stats.durability as i32;
        if durability < 1 || matched.result_count != 1 { return None; }
        let head = stat_maps.first()?.get("tconstruct:head")?.as_object()?;
        let tier = match head.get("mining_tier")?.as_str()?.strip_prefix("minecraft:")? {
            "wood" => "wooden",
            tier @ ("stone" | "iron" | "gold" | "diamond" | "netherite") => tier,
            _ => return None,
        };
        let mut output = ItemStack::new(&matched.result, matched.result_count);
        output.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
            ("tic_materials", Tag::List(matched.materials.iter().map(|m| Tag::String(m.as_str().into())).collect())),
            ("tic_max_damage", Tag::Int(durability)),
            ("tic_attack_damage", Tag::Float(stats.attack_damage.to_bits())),
            ("tic_attack_speed", Tag::Float(stats.attack_speed.to_bits())),
        ])));
        output.data_mut().set_other("minecraft:max_damage", Some(Tag::Int(durability)));
        output.data_mut().set_other("minecraft:tool", Some(Tag::compound([
            ("rules", Tag::List(vec![
                Tag::compound([("blocks", Tag::String(format!("#minecraft:incorrect_for_{tier}_tool").as_str().into())), ("correct_for_drops", Tag::Byte(0))]),
                Tag::compound([("blocks", Tag::String("#minecraft:mineable/pickaxe".into())), ("speed", Tag::Float(stats.mining_speed.to_bits())), ("correct_for_drops", Tag::Byte(1))]),
            ])),
            ("default_mining_speed", Tag::Float(1.0f32.to_bits())),
            ("damage_per_block", Tag::Int(1)),
        ])));
        return Some((matched, output));
    }
    None
}
