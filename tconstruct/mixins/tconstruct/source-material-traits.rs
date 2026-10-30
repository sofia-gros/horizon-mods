// Resolve source material traits for an assembled tool using the current world snapshot.
use super::tconstruct_source_station_recipes::BuildingRecipe;
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
use std::collections::BTreeMap;

pub(super) struct Traits {
    pub levels: BTreeMap<String, i32>,
    pub durability_multiplier: f32,
}

pub(super) fn levels(world: &super::tconstruct_world::World, tool: &ItemStack) -> Option<Traits> {
    let Tag::List(ids) = tool
        .data()?
        .other("minecraft:custom_data")?
        .get("tic_materials")?
    else {
        return None;
    };
    let materials = world
        .chunks
        .__horizon_mixins_tconstruct_material_snapshot()
        .as_ref()?;
    let (namespace, name) = tool.item.split_once(':')?;
    let definition = materials
        .get(&format!("{namespace}:tinkering/tool_definitions/{name}"))?
        .last()?;
    let value: serde_json::Value = serde_json::from_slice(definition).ok()?;
    if !value
        .get("modules")?
        .as_array()?
        .iter()
        .any(|m| m.get("type").and_then(serde_json::Value::as_str) == Some("tconstruct:part_stats"))
    {
        return None;
    }
    let mut recipe = world
        .chunks
        .__horizon_mixins_tconstruct_recipe_snapshot()
        .as_ref()?
        .values()
        .filter_map(|bytes| BuildingRecipe::parse(bytes))
        .find(|recipe| recipe.result == &*tool.item)?;
    recipe.parts_override = None;
    let parts = recipe.parts(definition)?;
    if parts.len() != ids.len() {
        return None;
    }
    let mut result = BTreeMap::new();
    let mut stat_maps = Vec::new();
    for (part, id) in parts.iter().zip(ids) {
        let Tag::String(id) = id else {
            return None;
        };
        let id = id.to_string_lossy();
        let (namespace, name) = id.split_once(':')?;
        let path = name.split('#').next()?;
        let stats = materials.get(&format!("{namespace}:tinkering/materials/stats/{path}"))?;
        let stats = super::tconstruct_tx::source_merge_material_stats(stats).ok()?;
        stat_maps.push(
            stats
                .into_iter()
                .map(|(k, v)| (k, serde_json::Value::Object(v)))
                .collect(),
        );
        let Some(layers) = materials.get(&format!("{namespace}:tinkering/materials/traits/{path}"))
        else {
            continue;
        };
        let raw = super::tconstruct_tx::source_merge_material_traits(layers).ok()?;
        let kind = super::tconstruct_source_station_stats::part_kind(part)?;
        super::tconstruct_source_material_trait_select::add_traits(
            &mut result,
            raw.default.as_ref(),
            &raw.per_stat,
            kind,
        );
    }
    let durability_multiplier =
        super::tconstruct_source_station_stats::base_stats(definition, &stat_maps)?
            .durability_multiplier;
    Some(Traits {
        levels: result,
        durability_multiplier,
    })
}
