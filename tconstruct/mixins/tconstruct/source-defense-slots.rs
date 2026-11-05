// Live defense capacity. Fortified is a material trait, not a purchased upgrade.
use super::tconstruct_world::World;
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
use std::collections::BTreeMap;

const FORTIFIED: &str = "tconstruct:fortified";

fn stock_fortified(bytes: &[u8]) -> bool {
    serde_json::from_slice::<serde_json::Value>(bytes).ok().is_some_and(|value|
        value == serde_json::json!({"level_display":"tconstruct:default", "priority":60,
            "modules":[{"type":"tconstruct:modifier_slot", "each_level":1, "name":"defense"}]}))
}

pub(super) fn supported_fortified(world: &World) -> bool {
    world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()
        .and_then(|raw| raw.as_ref().ok())
        .and_then(|raw| raw.0.get("tconstruct:tinkering/modifiers/fortified"))
        .and_then(|layers| layers.last()).is_some_and(|bytes| stock_fortified(bytes))
}

fn total(definition: &[u8], level: i32) -> Option<i32> {
    if level < 0 { return None; }
    let value: serde_json::Value = serde_json::from_slice(definition).ok()?;
    let mut result = level;
    for module in value.get("modules")?.as_array()? {
        if module.get("type").and_then(serde_json::Value::as_str) != Some("tconstruct:modifier_slots") { continue; }
        let slots = module.get("slots")?.as_object()?;
        if let Some(defense) = slots.get("defense") {
            if module.get("hooks").is_some() { return None; }
            result = result.checked_add(i32::try_from(defense.as_i64()?).ok()?)?;
        }
    }
    (result >= 0).then_some(result)
}

pub(super) fn capacity(world: &World, definition: &[u8], tool: &ItemStack, traits: &BTreeMap<String, i32>) -> Option<i32> {
    // There is no ordinary Fortified upgrade recipe. Do not turn arbitrary
    // saved or command-added levels into additional spendable slots.
    if let Some(upgrades) = tool.data().and_then(|data| data.other("minecraft:custom_data"))
        .and_then(|data| data.get("tic_upgrades")) {
        let Tag::Compound(_) = upgrades else { return None; };
        match upgrades.get(FORTIFIED) { None | Some(Tag::Int(0)) => {}, _ => return None }
    }
    let level = traits.get(FORTIFIED).copied().unwrap_or(0);
    if level != 0 && !supported_fortified(world) { return None; }
    total(definition, level)
}

/// Return free and total slots together, using the same current-world data as
/// acquisition. Nothing is cached into the item, so reload cannot mint slots.
pub(super) fn display(world: &World, tool: &ItemStack) -> Option<[i32; 2]> {
    let (namespace, name) = tool.item.split_once(':')?;
    let materials = world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref()?;
    let definition = materials.get(&format!("{namespace}:tinkering/tool_definitions/{}",
        super::tconstruct_source_station_stats::definition_name(namespace, name)))?.last()?;
    let traits = super::tconstruct_source_material_traits::levels(world, tool)?;
    let total = capacity(world, definition, tool, &traits.levels)?;
    let free = super::tconstruct_source_upgrade_rules::free_defense_slots(tool, total)?;
    Some([free, total])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fortified_adds_defense_only_and_recomputes_without_accumulating() {
        let definition = br#"{"modules":[{"type":"tconstruct:modifier_slots","slots":{"upgrades":3,"abilities":1,"defense":2}},{"type":"tconstruct:modifier_slots","slots":{"defense":1}}]}"#;
        assert_eq!(total(definition, 0), Some(3));
        assert_eq!(total(definition, 2), Some(5));
        assert_eq!(total(definition, 2), Some(5));
        assert_eq!(total(definition, 0), Some(3));
        assert_eq!(total(definition, -1), None);
        assert_eq!(total(definition, i32::MAX), None);
        assert_eq!(total(br#"{"modules":[{"type":"tconstruct:modifier_slots","slots":{"defense":"2"}}]}"#, 1), None);
    }
    #[test]
    fn only_the_supported_modifier_contract_grants_slots() {
        let original = serde_json::json!({"level_display":"tconstruct:default", "priority":60,
            "modules":[{"type":"tconstruct:modifier_slot", "each_level":1, "name":"defense"}]});
        assert!(stock_fortified(&serde_json::to_vec(&original).unwrap()));
        let mut changed = original.clone();
        changed["modules"][0]["each_level"] = 2.into();
        assert!(!stock_fortified(&serde_json::to_vec(&changed).unwrap()));
        changed = original;
        changed["modules"][0]["name"] = "upgrades".into();
        assert!(!stock_fortified(&serde_json::to_vec(&changed).unwrap()));
        assert!(!stock_fortified(b"bad json"));
    }
    #[test]
    fn ancient_hide_selects_fortified_only_for_its_default_stat_path() {
        let traits = br#"{"default":[{"level":1,"name":"tconstruct:fortified"}],"perStat":{"tconstruct:melee_harvest":[{"level":1,"name":"tconstruct:fortune"}]}}"#;
        for (stat, expected) in [("tconstruct:maille", FORTIFIED), ("tconstruct:head", "tconstruct:fortune")] {
            let definition = serde_json::to_vec(&serde_json::json!({"modules":[{
                "type":"tconstruct:material_stats", "stat_types":[stat,stat]}]})).unwrap();
            let levels = super::super::tconstruct_source_material_traits::levels_from_definition(&definition,
                &["tconstruct:ancient_hide".into(), "tconstruct:ancient_hide".into()],
                |id| (id == "tconstruct:tinkering/materials/traits/ancient_hide").then(|| vec![traits.to_vec()])).unwrap();
            assert_eq!(levels.get(expected), Some(&2));
            assert_eq!(levels.len(), 1);
        }
    }
    #[test]
    fn fortified_capacity_is_spendable_and_removal_revalidates_defense() {
        use std::collections::{HashMap, HashSet};
        use super::super::tconstruct_source_upgrade_rules::{plan_with_defense, valid_defense_slots};
        let definition = br#"{"modules":[{"type":"tconstruct:modifier_slots","slots":{"defense":0}}]}"#;
        let recipe = serde_json::json!({"type":"tconstruct:modifier", "allow_crystal":true,
            "inputs":[{"ingredient":{"item":"minecraft:anvil"}}], "level":1,
            "result":"tconstruct:knockback_resistance", "slots":{"defense":1},
            "tools":{"tag":"tconstruct:modifiable/armor"}});
        let recipes = BTreeMap::from([("knockback".into(), serde_json::to_vec(&recipe).unwrap())]);
        let tags = HashMap::from([("tconstruct:modifiable/armor".into(), HashSet::from(["tconstruct:plate_chestplate".into()]))]);
        let mut tool = ItemStack::new("tconstruct:plate_chestplate", 1);
        let inputs = [Some(ItemStack::new("minecraft:anvil", 1))];
        let traits = BTreeMap::from([(FORTIFIED.into(), 1)]);
        assert!(plan_with_defense(&recipes, &tags, &tool, &inputs, 0, 0, total(definition, 0).unwrap(), &traits).is_none());
        assert!(plan_with_defense(&recipes, &tags, &tool, &inputs, 0, 0, total(definition, 1).unwrap(), &traits).is_some());
        tool.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_upgrades",
            Tag::compound([("tconstruct:knockback_resistance", Tag::Int(1))]))])));
        assert!(valid_defense_slots(&tool, total(definition, 1).unwrap()));
        assert!(!valid_defense_slots(&tool, total(definition, 0).unwrap()));
    }

}
