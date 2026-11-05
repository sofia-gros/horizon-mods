//! Execute the exact injected helper against small current-world item snapshots.
use std::collections::HashMap;
#[derive(Default)]
pub struct Items {
    pub items: HashMap<String, ()>,
    pub item_tag_order: HashMap<String, Vec<String>>,
}
#[derive(Debug, PartialEq)]
pub struct Ingredient(pub Vec<String>);
impl Ingredient { pub fn of(items: Vec<String>) -> Self { Self(items) } }
include!("../../../mixins/tconstruct/source-stonecutting-ingredient.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::ops::ControlFlow::{Break, Continue};

    fn registry() -> Items {
        let seared = ["seared_bricks", "seared_fancy_bricks", "seared_triangle_bricks"];
        let scorched = ["scorched_stone", "polished_scorched_stone", "scorched_bricks", "scorched_road", "chiseled_scorched_bricks"];
        let ids = |names: &[&str]| names.iter().map(|name| format!("tconstruct:{name}")).collect::<Vec<_>>();
        let mut items = Items::default();
        items.items = ids(&seared).into_iter().chain(ids(&scorched)).chain(ids(&["seared_stone"]))
            .chain(["example:world_block".into()]).map(|id| (id, ())).collect();
        items.item_tag_order.insert("tconstruct:seared_bricks".into(), ids(&seared));
        items.item_tag_order.insert("tconstruct:scorched_blocks".into(), ids(&scorched));
        items
    }
    fn wrapped(value: Value) -> Value { json!({"type": "tconstruct:stonecutting_ingredient", "value": value}) }
    fn resolve(value: &Value, items: &Items) -> Vec<String> {
        match tconstruct_stonecutting_ingredient(value, items) {
            Break(Ingredient(ids)) => ids,
            Continue(()) => panic!("custom predicate fell through"),
        }
    }
    fn difference(tag: &str, excluded: &str) -> Value {
        json!({"type": "forge:difference", "base": {"tag": tag}, "subtracted": {"item": excluded}})
    }

    #[test]
    fn all_six_native_recipes_keep_source_subtraction_and_union() {
        let recipes = [
            include_str!("../../../content/data/tconstruct/recipe/chiseled_scorched_bricks_stonecutting.json"),
            include_str!("../../../content/data/tconstruct/recipe/polished_scorched_stone_stonecutting.json"),
            include_str!("../../../content/data/tconstruct/recipe/scorched_bricks_stonecutting.json"),
            include_str!("../../../content/data/tconstruct/recipe/seared_bricks_stonecutting.json"),
            include_str!("../../../content/data/tconstruct/recipe/seared_fancy_bricks_stonecutting.json"),
            include_str!("../../../content/data/tconstruct/recipe/seared_triangle_bricks_stonecutting.json"),
        ];
        let items = registry();
        for source in recipes {
            let recipe: Value = serde_json::from_str(source).unwrap();
            let result = recipe["result"]["id"].as_str().unwrap();
            let seared = result.starts_with("tconstruct:seared_");
            let tag = if seared { "tconstruct:seared_bricks" } else { "tconstruct:scorched_blocks" };
            let mut expected = items.item_tag_order[tag].clone();
            expected.retain(|id| id != result);
            if seared { expected.push("tconstruct:seared_stone".into()); }
            let actual = resolve(&recipe["ingredient"], &items);
            assert_eq!(actual, expected);
            assert!(!actual.iter().any(|id| id == result));
            assert_eq!(recipe["result"]["count"], 1);
        }
    }

    #[test]
    fn world_membership_replacement_order_duplicates_and_missing_tags_are_live() {
        let predicate = wrapped(difference("tconstruct:seared_bricks", "tconstruct:seared_bricks"));
        let mut items = registry();
        assert_eq!(resolve(&predicate, &items), vec!["tconstruct:seared_fancy_bricks", "tconstruct:seared_triangle_bricks"]);
        items.item_tag_order.insert("tconstruct:seared_bricks".into(), vec!["example:world_block".into(),
            "tconstruct:seared_bricks".into(), "example:world_block".into()]);
        assert_eq!(resolve(&predicate, &items), vec!["example:world_block"]);
        items.item_tag_order.remove("tconstruct:seared_bricks");
        assert!(resolve(&predicate, &items).is_empty());
        let union = wrapped(json!([difference("tconstruct:seared_bricks", "tconstruct:seared_bricks"),
            {"item": "tconstruct:seared_stone"}]));
        assert_eq!(resolve(&union, &items), vec!["tconstruct:seared_stone"]);
        // A different world has its own members, without cached substitutions.
        assert_eq!(resolve(&predicate, &registry()).len(), 2);
    }

    #[test]
    fn malformed_or_unsupported_children_close_the_entire_ingredient() {
        let items = registry();
        for value in [Value::Null, json!([]), json!({}), json!({"tag": 1}),
            json!({"item": "example:missing"}), json!({"tag": "invalid id"}),
            json!({"item": "tconstruct:seared_stone", "nbt": "{}"}),
            json!({"type": "forge:difference", "base": {"tag": "tconstruct:seared_bricks"}}),
            json!({"type": "forge:intersection", "children": []}),
            json!([{"item": "tconstruct:seared_stone"}, {"type": "example:unknown"}]),
            json!({"type": "forge:difference", "base": {"tag": "tconstruct:seared_bricks"},
                "subtracted": {"item": "tconstruct:seared_bricks"}, "extra": true})] {
            assert!(resolve(&wrapped(value), &items).is_empty());
        }
        let mut extra = wrapped(json!({"item": "tconstruct:seared_stone"})); extra["extra"] = json!(true);
        assert!(resolve(&extra, &items).is_empty());
        assert!(resolve(&json!({"type": "tconstruct:stonecutting_ingredient"}), &items).is_empty());
        let mut invalid_tag = registry();
        invalid_tag.item_tag_order.insert("tconstruct:seared_bricks".into(), vec!["example:missing".into()]);
        assert!(resolve(&wrapped(json!({"tag": "tconstruct:seared_bricks"})), &invalid_tag).is_empty());
    }

    #[test]
    fn composition_is_bounded_and_ordinary_ingredients_are_untouched() {
        let items = registry();
        let mut value = json!({"item": "tconstruct:seared_stone"});
        for _ in 0..9 { value = json!([value]); }
        assert!(resolve(&wrapped(value), &items).is_empty());
        assert!(resolve(&wrapped(json!(vec![json!({"item": "tconstruct:seared_stone"}); 17])), &items).is_empty());
        for value in [Value::Null, json!("tconstruct:seared_stone"), json!("#tconstruct:seared_bricks"),
            json!(["tconstruct:seared_stone"]), json!({"item": "tconstruct:seared_stone"}),
            difference("tconstruct:seared_bricks", "tconstruct:seared_bricks"),
            json!({"type": "example:other", "value": {"item": "tconstruct:seared_stone"}})] {
            assert_eq!(tconstruct_stonecutting_ingredient(&value, &items), Continue(()));
        }
    }
}
