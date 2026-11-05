// Standalone harness for the production condition resolver. The legacy
// condition parser is deliberately unavailable here: these tests exercise
// Forge item/mod conditions and conservative unknown handling.
#[allow(dead_code)]
mod tconstruct_tx {
    use std::collections::{HashMap, HashSet};
    pub(super) fn source_conditions_match(
        _: &[serde_json::Value],
        _: &HashMap<String, HashMap<String, HashSet<String>>>,
        _: &HashMap<String, bool>,
    ) -> Option<bool> { None }
}
#[allow(dead_code)]
#[path = "../../../mixins/tconstruct/source-conditions.rs"]
mod tconstruct_source_conditions;

#[allow(dead_code)]
#[path = "../../../mixins/tconstruct/source-tags.rs"]
mod tconstruct_source_tags;
#[allow(dead_code)]
#[path = "../../../mixins/tconstruct/modifier-snapshot.rs"]
mod tconstruct_modifier_snapshot;

#[allow(dead_code)]
#[path = "../../../mixins/tconstruct/pig-iron-piston.rs"]
mod tconstruct_pig_iron_piston;

#[allow(dead_code)]
#[path = "../../../mixins/tconstruct/source-budding-piston.rs"]
mod tconstruct_source_budding_piston;

#[cfg(test)]
mod budding_snapshot_test {
    #[test]
    fn extracted_prepared_snapshot_changes_only_four_push_fields() {
        let sample: serde_json::Value = serde_json::from_str(include_str!(
            "../../fixtures/budding-piston-prepared-snapshot.json")).unwrap();
        // Restore compact rows to their original registry indexes for production validation.
        let compact_facts = sample["facts"]["states"].as_array().unwrap();
        let compact_game = sample["gameplay"]["states"].as_array().unwrap();
        let max_id = compact_game.iter().map(|row| row["id"].as_u64().unwrap() as usize).max().unwrap();
        let mut facts = serde_json::json!({"states": vec![serde_json::Value::Null; max_id + 1]});
        let mut game = serde_json::json!({"states": vec![serde_json::Value::Null; max_id + 1]});
        for (fact, row) in compact_facts.iter().zip(compact_game) {
            let index = row["id"].as_u64().unwrap() as usize;
            facts["states"][index] = fact.clone();
            game["states"][index] = row.clone();
        }
        let original = game.clone();
        super::tconstruct_source_budding_piston::apply(&facts, &mut game).unwrap();
        let old = original["states"].as_array().unwrap();
        let new = game["states"].as_array().unwrap();
        let states = facts["states"].as_array().unwrap();
        let changed: Vec<_> = old.iter().zip(new).enumerate().filter(|(_, (a, b))| a != b).collect();
        assert_eq!(changed.len(), 4);
        for (i, (before, after)) in changed {
            assert!(states[i][0].as_str().unwrap().starts_with("tconstruct:budding_"));
            assert_eq!(before["push"], "PUSH_PULL");
            assert_eq!(after["push"], "POPPED");
            let mut expected = before.clone(); expected["push"] = serde_json::json!("POPPED");
            assert_eq!(*after, expected);
        }
        for (i, state) in states.iter().enumerate() {
            if state[0].as_str().is_some_and(|id| id.starts_with("minecraft:")) { assert_eq!(old[i], new[i]); }
        }
    }

}

#[path = "../../../mixins/tconstruct/source-scute-compat.rs"]
mod tconstruct_source_scute_compat;

#[cfg(test)]
mod scute_compat_tests {
    use super::tconstruct_source_scute_compat as compat;
    use serde_json::json;

    #[test]
    fn preparation_is_bounded_idempotent_and_registry_gated() {
        let dir = std::env::temp_dir().join(format!("tconstruct-scute-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let original = [
            (compat::MATERIAL, json!({"type":"tconstruct:material", "ingredient":{"item":"minecraft:scute"}, "material":"tconstruct:turtle", "needed":1,"value":1})),
            (compat::RECYCLING, json!({"type":"tconstruct:part_builder_recycling", "pattern":{"item":"tconstruct:venom_bottle"}, "tool":{"item":"minecraft:turtle_helmet"}, "results":{"tconstruct:scale":{"item":"minecraft:scute","count":5}}})),
        ];
        for (id, value) in &original {
            let path = dir.join(format!("data/tconstruct/recipes/{id}.json"));
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
        }
        for registry in [json!(null), json!({}), json!({"minecraft:scute":{}}), json!({"minecraft:scute":{},"minecraft:turtle_scute":{}})] {
            compat::prepare(&dir, &registry).unwrap();
            for (id, value) in &original {
                let read: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.join(format!("data/tconstruct/recipes/{id}.json"))).unwrap()).unwrap();
                assert_eq!(&read, value);
            }
        }
        compat::prepare(&dir, &json!({"minecraft:turtle_scute":{}})).unwrap();
        for (id, value) in &original {
            let path = dir.join(format!("data/tconstruct/recipes/{id}.json"));
            let saved = std::fs::read(&path).unwrap();
            let read: serde_json::Value = serde_json::from_slice(&saved).unwrap();
            let mut expected = value.clone();
            assert!(compat::adapt_recipe(id, &mut expected));
            assert_eq!(read, expected);
            compat::prepare(&dir, &json!({"minecraft:turtle_scute":{}})).unwrap();
            assert_eq!(std::fs::read(path).unwrap(), saved);
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn unrelated_or_overridden_recipes_are_not_redirected() {
        for (id, mut value) in [
            ("other", json!({"type":"tconstruct:material","ingredient":{"item":"minecraft:scute"}})),
            (compat::MATERIAL, json!({"type":"other","ingredient":{"item":"minecraft:scute"}})),
            (compat::MATERIAL, json!({"type":"tconstruct:material","ingredient":{"tag":"forge:scutes"}})),
            (compat::MATERIAL, json!({"type":"tconstruct:material","ingredient":{"item":"mod:custom_scute"}})),
        ] {
            let original = value.clone();
            assert!(!compat::adapt_recipe(id, &mut value));
            assert_eq!(value, original);
        }
    }
}
