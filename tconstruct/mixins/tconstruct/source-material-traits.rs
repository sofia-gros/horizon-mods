// Resolve source material traits for assembled and Creative tools.
use super::tconstruct_source_station_recipes::BuildingRecipe;
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
use std::collections::BTreeMap;

pub(super) struct Traits {
    pub levels: BTreeMap<String, i32>,
    pub order: Vec<String>,
    pub durability_multiplier: f32,
}

/// Combine independently stored recipe upgrades with resolved traits once.
/// Refuse malformed values rather than treating them as an absent upgrade.
pub(super) fn level_with_upgrade(stack: &ItemStack, traits: &Traits, id: &str) -> Option<i32> {
    let saved = stack.data().and_then(|data| data.other("minecraft:custom_data"))
        .and_then(|tag| tag.get("tic_upgrades"));
    let upgrade = match saved {
        None => 0,
        Some(tag @ Tag::Compound(_)) => match tag.get(id) {
            None => 0,
            Some(Tag::Int(level)) if *level >= 0 => *level,
            _ => return None,
        },
        _ => return None,
    };
    merge_modifier_levels(traits.levels.get(id).copied().unwrap_or(0), upgrade)
}

fn merge_modifier_levels(trait_level: i32, upgrade: i32) -> Option<i32> {
    if trait_level < 0 || upgrade < 0 { return None; }
    trait_level.checked_add(upgrade)
}

pub(super) fn levels_from_definition(
    definition: &[u8],
    identities: &[String],
    read: impl Fn(&str) -> Option<Vec<Vec<u8>>>,
) -> Option<BTreeMap<String, i32>> {
    ordered_levels_from_definition(definition, identities, read).map(|(levels, _)| levels)
}

pub fn ordered_levels_from_definition(
    definition: &[u8], identities: &[String], read: impl Fn(&str) -> Option<Vec<Vec<u8>>>,
) -> Option<(BTreeMap<String, i32>, Vec<String>)> {
    let value: serde_json::Value = serde_json::from_slice(definition).ok()?;
    let modules = value.get("modules")?.as_array()?;
    let stats = modules.iter().find(|module| matches!(
        module.get("type").and_then(serde_json::Value::as_str),
        Some("tconstruct:part_stats" | "tconstruct:material_stats")
    ))?;
    let is_parts = stats.get("type")?.as_str()? == "tconstruct:part_stats";
    let entries = stats.get(if is_parts { "parts" } else { "stat_types" })?.as_array()?;
    if entries.len() != identities.len() { return None; }
    let mut result = BTreeMap::new();
    let mut order = Vec::new();
    let add = |result: &mut BTreeMap<String, i32>, order: &mut Vec<String>, id: &str, kind: &str| -> Option<()> {
        let (namespace, name) = id.split_once(':')?;
        let path = name.split('#').next()?;
        let Some(layers) = read(&format!("{namespace}:tinkering/materials/traits/{path}")) else {
            return Some(());
        };
        let raw = super::tconstruct_tx::source_merge_material_traits(&layers).ok()?;
        super::tconstruct_source_material_trait_select::add_traits_ordered(
            result, order, raw.default.as_ref(), &raw.per_stat, kind,
        );
        Some(())
    };
    for module in modules {
        if std::ptr::eq(module, stats) {
            for (entry, id) in entries.iter().zip(identities) {
                let name = entry.as_str().or_else(|| entry.get(if is_parts { "item" } else { "stat" }).and_then(serde_json::Value::as_str))?;
                let kind = if is_parts { super::tconstruct_source_station_stats::part_kind(name)? } else { name.strip_prefix("tconstruct:")? };
                add(&mut result, &mut order, id, kind)?;
            }
        } else if module.get("type").and_then(serde_json::Value::as_str) == Some("tconstruct:material_traits") {
            // An explicit hook override must include tool_traits.
            if let Some(hooks) = module.get("hooks").and_then(serde_json::Value::as_array) {
                if !hooks.iter().any(|hook| hook.as_str() == Some("tconstruct:tool_traits")) { continue; }
            }
            let index = usize::try_from(module.get("material_index")?.as_u64()?).ok()?;
            let entry = entries.get(index)?;
            let stat = module.get("stat_type").and_then(serde_json::Value::as_str)
                .or_else(|| entry.as_str()).or_else(|| entry.get("stat").and_then(serde_json::Value::as_str))?;
            add(&mut result, &mut order, identities.get(index)?, stat.strip_prefix("tconstruct:")?)?;
        } else if module.get("type").and_then(serde_json::Value::as_str) == Some("tconstruct:traits") {
            if let Some(hooks) = module.get("hooks").and_then(serde_json::Value::as_array) {
                if !hooks.iter().any(|hook| hook.as_str() == Some("tconstruct:tool_traits")) { continue; }
            }
            for trait_entry in module.get("traits")?.as_array()? {
                let name = trait_entry.get("name")?.as_str()?;
                let level = match trait_entry.get("level") {
                    Some(level) => i32::try_from(level.as_i64()?).ok()?,
                    None => 1,
                };
                if level <= 0 { return None; }
                if !result.contains_key(name) { order.push(name.to_owned()); }
                *result.entry(name.to_owned()).or_insert(0) += level;
            }
        }
    }
    Some((result, order))
}

// Reverted grants the hidden Pocket trait at fixed level 1 on slime leggings.
// This is independent of Reverted's own level and is recomputed on rebuild.
fn apply_reverted_trait(item: &str, levels: &mut BTreeMap<String, i32>, order: &mut Vec<String>) -> Option<()> {
  if item == "tconstruct:slime_leggings" && levels.get("tconstruct:reverted").copied().unwrap_or(0) > 0 {
    let pocket = levels.entry("tconstruct:pocket".into()).or_insert_with(|| {
      order.push("tconstruct:pocket".into());
      0
    });
    *pocket = pocket.checked_add(1)?;
  }
  Some(())
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
        .get(&format!("{namespace}:tinkering/tool_definitions/{}", super::tconstruct_source_station_stats::definition_name(namespace, name)))?
        .last()?;
    let definition_json: serde_json::Value = serde_json::from_slice(definition).ok()?;
    let has_part_stats = definition_json.get("modules")?.as_array()?.iter().any(|module| {
        module.get("type").and_then(serde_json::Value::as_str) == Some("tconstruct:part_stats")
    });
    if has_part_stats {
        let mut recipe = world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().as_ref()?
            .values().filter_map(|bytes| BuildingRecipe::parse(bytes))
            .find(|recipe| recipe.result == &*tool.item)?;
        recipe.parts_override = None;
        if recipe.parts(definition)?.len() != identities.len() { return None; }
    }
    let (mut resolved, mut order) =
        ordered_levels_from_definition(definition, &identities, |id| materials.get(id).cloned())?;
    apply_reverted_trait(&tool.item, &mut resolved, &mut order)?;
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
    // Original stat_multiplier is the product of multiply_all operations;
    // material percentages and multiply_base traits do not change it.
    Some(Traits {
        levels: resolved,
        order,
        durability_multiplier: stats.durability_multiplier_unit,
    })
}

#[cfg(test)]
mod merged_level_tests {
    use super::merge_modifier_levels;
    #[test]
    fn traits_and_saved_upgrades_are_counted_once() {
        assert_eq!(merge_modifier_levels(0, 0), Some(0));
        assert_eq!(merge_modifier_levels(2, 0), Some(2));
        assert_eq!(merge_modifier_levels(0, 3), Some(3));
        assert_eq!(merge_modifier_levels(2, 3), Some(5));
        assert_eq!(merge_modifier_levels(-1, 1), None);
        assert_eq!(merge_modifier_levels(1, -1), None);
        assert_eq!(merge_modifier_levels(i32::MAX, 1), None);
    }
}

#[cfg(test)]
mod reverted_tests {
  use super::*;
  #[test]
  fn pocket_is_fixed_and_only_on_reverted_leggings() {
    for (item, reverted, expected) in [("tconstruct:slime_leggings", 0, 0), ("tconstruct:slime_leggings", 2, 1), ("tconstruct:slime_wings", 1, 0)] {
      let mut levels = BTreeMap::from([("tconstruct:reverted".into(), reverted)]);
      let mut order = vec!["tconstruct:reverted".into()];
      apply_reverted_trait(item, &mut levels, &mut order).unwrap();
      assert_eq!(levels.get("tconstruct:pocket").copied().unwrap_or(0), expected);
    }
  }
}
