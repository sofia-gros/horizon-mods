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

#[cfg(test)]
mod modifier_identity_fixtures {
    #[test]
    fn source_derived_modifier_tag_operations() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../fixtures/modifier-tag-identities.json")).unwrap();
        let registry = fixture["registry"].as_object().unwrap().iter()
            .map(|(id, target)| (id.clone(), target.as_str().unwrap().to_owned())).collect();
        for case in fixture["cases"].as_array().unwrap() {
            let mut raw = super::tconstruct_modifier_snapshot::Raw::default();
            for (tag, layers) in case["tags"].as_object().unwrap() {
                raw.1.insert(format!("x:tinkering/tags/modifiers/{tag}"),
                    layers.as_array().unwrap().iter().map(|layer| serde_json::to_vec(layer).unwrap()).collect());
            }
            let result = super::tconstruct_modifier_snapshot::resolve_canonical_tag(&raw, "x:root", Some(&registry));
            if let Some(error) = case["error"].as_str() {
                assert_eq!(result.unwrap_err(), error, "{}", case["name"]);
            } else {
                let expected: Vec<_> = case["expected"].as_array().unwrap().iter().map(|id| id.as_str().unwrap()).collect();
                assert_eq!(result.unwrap(), expected, "{}", case["name"]);
            }
        }
    }
}
