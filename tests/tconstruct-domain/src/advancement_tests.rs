//! The shipped advancement data through the production consumers: every criterion of every
//! shipped file is classified by what its grant needs, and every `tconstruct:tool_stack`
//! predicate decodes through the production decoder. The classification is the coverage
//! contract of the advancement port: `Host` criteria the game evaluates on its own,
//! `ToolConsumer` criteria the registered item predicate evaluates, `EventConsumer` criteria
//! the block_container_opened submission feeds, and the three classes whose grant waits on
//! another lane's content (slime island structures, the piggy backpack item, the book item
//! the starting book's reward table gives).
use crate::advancements::ToolPredicate;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn content() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tconstruct/content/data/tconstruct")
}

fn advancements() -> Vec<(String, Value)> {
    let root = content().join("advancement");
    let mut found = Vec::new();
    for group in ["foundry", "internal", "smeltery", "tools", "world"] {
        for entry in std::fs::read_dir(root.join(group)).expect("advancement group folder") {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let name = format!("{group}/{}", path.file_stem().unwrap().to_str().unwrap());
            let value = serde_json::from_slice(&std::fs::read(&path).unwrap()).expect("valid JSON");
            found.push((name, value));
        }
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Class {
    Host,
    ToolConsumer,
    EventConsumer,
    Structures,
    PiggyItem,
    BookReward,
}

fn classify(data: &Value) -> Class {
    let mut tool = false;
    let mut event = false;
    let mut structure = false;
    let mut piggy = false;
    fn item_flags(predicate: &Value, tool: &mut bool, piggy: &mut bool) {
        if predicate.get("type").and_then(Value::as_str) == Some("tconstruct:tool_stack") {
            *tool = true;
        }
        if predicate
            .get("items")
            .and_then(Value::as_array)
            .is_some_and(|items| items.iter().any(|item| item.as_str() == Some("tconstruct:piggy_backpack")))
        {
            *piggy = true;
        }
    }
    fn entity_flags(node: &Value, tool: &mut bool, structure: &mut bool) {
        for entry in node.as_array().into_iter().flatten() {
            let predicate = &entry["predicate"];
            if predicate["location"].get("structure").is_some() {
                *structure = true;
            }
            for slot in predicate["equipment"].as_object().into_iter().flat_map(|slots| slots.values()) {
                if slot.get("type").and_then(Value::as_str) == Some("tconstruct:tool_stack") {
                    *tool = true;
                }
            }
        }
    }
    for criterion in data["criteria"].as_object().into_iter().flat_map(|criteria| criteria.values()) {
        let conditions = &criterion["conditions"];
        if criterion["trigger"].as_str() == Some("tconstruct:block_container_opened") {
            event = true;
        }
        for item in conditions["items"].as_array().into_iter().flatten() {
            item_flags(item, &mut tool, &mut piggy);
        }
        if conditions.get("item").is_some() {
            item_flags(&conditions["item"], &mut tool, &mut piggy);
        }
        entity_flags(&conditions["player"], &mut tool, &mut structure);
        entity_flags(&conditions["entity"], &mut tool, &mut structure);
    }
    if structure {
        Class::Structures
    } else if piggy {
        Class::PiggyItem
    } else if data["rewards"]["loot"].is_array() {
        Class::BookReward
    } else if event {
        Class::EventConsumer
    } else if tool {
        Class::ToolConsumer
    } else {
        Class::Host
    }
}

fn expected() -> BTreeMap<&'static str, Class> {
    use Class::*;
    [
        ("foundry/alloyer", Host),
        ("foundry/blaze", Host),
        ("foundry/encyclopedia", Host),
        ("foundry/fantastic_foundry", Host),
        ("foundry/manyullyn_lanterns", Host),
        ("foundry/plate_armor", Host),
        ("foundry/structure", EventConsumer),
        ("internal/crafting_station", Host),
        ("internal/diamond_armor", ToolConsumer),
        ("internal/iron_armor", ToolConsumer),
        ("internal/iron_pickaxe", ToolConsumer),
        ("internal/netherite_armor", ToolConsumer),
        ("internal/netherite_hoe", ToolConsumer),
        ("internal/starting_book", BookReward),
        ("internal/stone_pickaxe", ToolConsumer),
        ("internal/walk_on_powder_snow", ToolConsumer),
        ("smeltery/abilities", ToolConsumer),
        ("smeltery/cast_collector", Host),
        ("smeltery/gold_casting", Host),
        ("smeltery/melter", Host),
        ("smeltery/mighty_smelting", Host),
        ("smeltery/puny_smelting", Host),
        ("smeltery/sand_casting", Host),
        ("smeltery/structure", EventConsumer),
        ("smeltery/tinkers_anvil", Host),
        ("smeltery/tool_forge", Host),
        ("tools/make_part", Host),
        ("tools/material_master", ToolConsumer),
        ("tools/materials_and_you", Host),
        ("tools/modified", ToolConsumer),
        ("tools/netherite_tier", ToolConsumer),
        ("tools/one_shot", ToolConsumer),
        ("tools/part_builder", Host),
        ("tools/perfect_aim", ToolConsumer),
        ("tools/tinker_station", Host),
        ("tools/tinker_tool", Host),
        ("tools/tool_smith", Host),
        ("tools/travelers_gear", Host),
        ("tools/upgrade_slots", ToolConsumer),
        ("world/ancient_tools", Host),
        ("world/blood_island", Structures),
        ("world/clay_island", Structures),
        ("world/earth_island", Structures),
        ("world/ender_island", Structures),
        ("world/piggybackpack", PiggyItem),
        ("world/sky_island", Structures),
        ("world/slime_collector", Host),
        ("world/slimeskull", ToolConsumer),
        ("world/slimesuit", Host),
        ("world/tinkers_gadgetry", Host),
    ]
    .into_iter()
    .collect()
}

#[test]
fn all_fifty_gameplay_advancements_ship_and_classify_as_reviewed() {
    let shipped = advancements();
    assert_eq!(shipped.len(), 50);
    let classified: BTreeMap<String, Class> = shipped.iter().map(|(name, data)| (name.clone(), classify(data))).collect();
    let expected: BTreeMap<String, Class> = expected().into_iter().map(|(name, class)| (name.to_owned(), class)).collect();
    assert_eq!(classified, expected);
    let count = |class: Class| classified.values().filter(|held| **held == class).count();
    assert_eq!(
        (count(Class::Host), count(Class::ToolConsumer), count(Class::EventConsumer), count(Class::Structures), count(Class::PiggyItem), count(Class::BookReward)),
        (26, 15, 2, 5, 1, 1)
    );
    // Every parent inside the tconstruct namespace names another shipped advancement.
    let ids: Vec<&str> = shipped.iter().map(|(name, _)| name.as_str()).collect();
    for (_, data) in &shipped {
        if let Some(parent) = data["parent"].as_str() {
            let parent = parent.strip_prefix("tconstruct:").expect("parents are tconstruct advancements");
            assert!(ids.contains(&parent), "parent {parent} ships");
        }
    }
}

#[test]
fn every_shipped_tool_predicate_decodes_through_the_production_decoder() {
    let mut forms: BTreeMap<&'static str, usize> = BTreeMap::new();
    fn count(predicate: &ToolPredicate, forms: &mut BTreeMap<&'static str, usize>) {
        let name = match predicate {
            ToolPredicate::HasUpgrades => "has_upgrades",
            ToolPredicate::HasModifier { .. } => "has_modifier",
            ToolPredicate::HasMaterial { .. } => "has_material",
            ToolPredicate::HasVolatileKey(_) => "has_volatile_key",
            ToolPredicate::StatInRange { .. } => "stat_in_range",
            ToolPredicate::StatInSet { .. } => "stat_in_set",
            ToolPredicate::ToolAction(_) => "tool_action",
            ToolPredicate::Set(_) => "mantle:set",
            ToolPredicate::Tag(_) => "mantle:tag",
            ToolPredicate::And(_) => "mantle:and",
            ToolPredicate::Or(_) => "mantle:or",
            ToolPredicate::Not(_) => "mantle:inverted",
        };
        *forms.entry(name).or_default() += 1;
        match predicate {
            ToolPredicate::And(all) | ToolPredicate::Or(all) => all.iter().for_each(|sub| count(sub, forms)),
            ToolPredicate::Not(inner) => count(inner, forms),
            _ => {}
        }
    }
    fn visit(item: &Value, forms: &mut BTreeMap<&'static str, usize>) {
        if item.get("type").and_then(Value::as_str) == Some("tconstruct:tool_stack") {
            let decoded = ToolPredicate::decode(&item["predicate"]).unwrap_or_else(|error| panic!("shipped tool predicate decodes: {error}"));
            count(&decoded, forms);
        }
    }
    for (_, data) in advancements() {
        for criterion in data["criteria"].as_object().unwrap().values() {
            let conditions = &criterion["conditions"];
            for item in conditions["items"].as_array().into_iter().flatten() {
                visit(item, &mut forms);
            }
            if conditions.get("item").is_some() {
                visit(&conditions["item"], &mut forms);
            }
            for key in ["player", "entity"] {
                for entry in conditions[key].as_array().into_iter().flatten() {
                    for slot in entry["predicate"]["equipment"].as_object().into_iter().flat_map(|slots| slots.values()) {
                        visit(slot, &mut forms);
                    }
                }
            }
        }
    }
    assert_eq!(
        forms,
        BTreeMap::from([
            ("has_material", 54),
            ("has_modifier", 62),
            ("has_upgrades", 1),
            ("has_volatile_key", 8),
            ("mantle:and", 28),
            ("mantle:set", 16),
            ("mantle:tag", 11),
            ("stat_in_range", 2),
            ("stat_in_set", 3),
            ("tool_action", 3),
        ])
    );
}

#[test]
fn the_internal_advancements_reward_the_shipped_grant_functions() {
    let mut rewarded = 0;
    for (_, data) in advancements() {
        let Some(function) = data["rewards"]["function"].as_str() else { continue };
        rewarded += 1;
        let name = function.strip_prefix("tconstruct:grant_advancement/").expect("internal rewards are grant functions");
        let path = content().join(format!("function/grant_advancement/{name}.mcfunction"));
        let text = std::fs::read_to_string(&path).expect("grant function ships");
        assert!(text.starts_with("advancement grant @s only minecraft:"), "{name}: {text}");
    }
    assert_eq!(rewarded, 8);
    // The starting book's reward table ships and gives the book item.
    let table: Value = serde_json::from_slice(&std::fs::read(content().join("loot_table/gameplay/starting_book.json")).unwrap()).unwrap();
    assert_eq!(table["type"], "minecraft:advancement_reward");
    assert_eq!(table["pools"][0]["entries"][0]["name"], "tconstruct:materials_and_you");
}
