// Resolve source material traits for assembled and Creative tools.
use super::tconstruct_source_station_recipes::BuildingRecipe;
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
use std::collections::BTreeMap;

pub(super) struct Traits {
    pub levels: BTreeMap<String, i32>,
    pub durability_multiplier: f32,
}

pub(super) fn levels_from_definition(
    definition: &[u8],
    identities: &[String],
    read: impl Fn(&str) -> Option<Vec<Vec<u8>>>,
) -> Option<BTreeMap<String, i32>> {
    let value: serde_json::Value = serde_json::from_slice(definition).ok()?;
    let part_stats = value.get("modules")?.as_array()?.iter().find(|m| {
        m.get("type").and_then(serde_json::Value::as_str) == Some("tconstruct:part_stats")
    })?;
    let parts = part_stats.get("parts")?.as_array()?;
    if parts.len() != identities.len() {
        return None;
    }
    let mut result = BTreeMap::new();
    for (part, id) in parts.iter().zip(identities) {
        let part = part
            .as_str()
            .or_else(|| part.get("item").and_then(serde_json::Value::as_str))?;
        let (namespace, name) = id.split_once(':')?;
        let path = name.split('#').next()?;
        let Some(layers) = read(&format!("{namespace}:tinkering/materials/traits/{path}")) else {
            continue;
        };
        let raw = super::tconstruct_tx::source_merge_material_traits(&layers).ok()?;
        let kind = super::tconstruct_source_station_stats::part_kind(part)?;
        super::tconstruct_source_material_trait_select::add_traits(
            &mut result,
            raw.default.as_ref(),
            &raw.per_stat,
            kind,
        );
    }
    Some(result)
}

pub(super) fn levels(world: &super::tconstruct_world::World, tool: &ItemStack) -> Option<Traits> {
    let Tag::List(ids) = tool
        .data()?
        .other("minecraft:custom_data")?
        .get("tic_materials")?
    else {
        return None;
    };
    let identities = ids
        .iter()
        .map(|id| match id {
            Tag::String(id) => Some(id.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    let materials = world
        .chunks
        .__horizon_mixins_tconstruct_material_snapshot()
        .as_ref()?;
    let (namespace, name) = tool.item.split_once(':')?;
    let definition = materials
        .get(&format!("{namespace}:tinkering/tool_definitions/{name}"))?
        .last()?;
    let mut recipe = world
        .chunks
        .__horizon_mixins_tconstruct_recipe_snapshot()
        .as_ref()?
        .values()
        .filter_map(|bytes| BuildingRecipe::parse(bytes))
        .find(|recipe| recipe.result == &*tool.item)?;
    recipe.parts_override = None;
    if recipe.parts(definition)?.len() != identities.len() {
        return None;
    }
    let resolved =
        levels_from_definition(definition, &identities, |id| materials.get(id).cloned())?;
    let mut stat_maps = Vec::new();
    for id in &identities {
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
    }
    let stats = super::tconstruct_source_station_stats::base_stats(definition, &stat_maps)?;
    let preserved = resolved.get("tconstruct:preserved").copied().unwrap_or(0);
    let ductile = resolved.get("tconstruct:ductile").copied().unwrap_or(0);
    let boost = super::tconstruct_source_material_effects::durability_boost(preserved, ductile);
    Some(Traits {
        levels: resolved,
        durability_multiplier: stats.durability_multiplier
            + stats.durability_multiplier_unit * boost,
    })
}
