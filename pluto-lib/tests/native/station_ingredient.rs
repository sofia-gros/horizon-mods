//! Real Items catalog + exact official Ingredient parser and station wrapper.
//! The HEAD dispatch and unrelated native ingredient-provider endpoints are modeled.
#[path = "station-callback.rs"]
mod consumer;
pub mod crafting;
pub mod recipe_book;
#[path = "station-reference.rs"]
mod reference;
pub mod stations;
#[cfg(test)]
mod tests {
    use super::*;
    use horizon_items::items::Items;
    use horizon_world::loot::PackOverlay;
    use pluto_inventory_api::crafting::ingredients::{self, Decision, Registration};
    use serde_json::json;
    fn catalog(name: &str, tag: &[&str]) -> Items {
        let path = std::env::temp_dir().join(format!(
            "pluto-station-parser-{}-{name}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(
            path.join("items.json"),
            br#"{"example:a":{},"example:b":{},"example:c":{}}"#,
        )
        .unwrap();
        let mut items = Items::catalog(&path, &PackOverlay::NONE).unwrap();
        std::fs::remove_dir_all(path).unwrap();
        items.item_tag_order.insert(
            "example:ordered".into(),
            tag.iter().map(|v| (*v).into()).collect(),
        );
        items.item_tag_order.insert("example:empty".into(), vec![]);
        items
    }
    fn marked(value: serde_json::Value) -> serde_json::Value {
        json!({"type":"tconstruct:stonecutting_ingredient","value":value})
    }
    #[test]
    fn exact_station_path_preserves_native_and_bounded_consumer_resolution() {
        let items = catalog("first", &["example:b", "example:a"]);
        let other = catalog("second", &["example:c"]);
        assert!(!ingredients::has_parsers());
        for v in [
            json!("example:a"),
            json!(["example:b", "example:a"]),
            json!("#example:ordered"),
            json!("not-in-registry"),
            json!({"neoforge:ingredient_type":"neoforge:difference","base":["example:b","example:a"],"subtracted":"example:a"}),
        ] {
            let actual = stations::ingredient(&v, &items);
            let original = stations::native_ingredient(&v, &items);
            assert_eq!(actual.order, original.order);
            assert_eq!(actual.items, original.items);
            assert_eq!(actual.children.is_some(), original.children.is_some());
        }
        ingredients::register(Registration {
            id: "unrelated",
            callback: |_| Decision::Unsupported,
        })
        .unwrap();
        assert_eq!(
            stations::ingredient(&json!("not-in-registry"), &items).order,
            vec!["not-in-registry"]
        );
        ingredients::register(Registration {
            id: "tconstruct:source-test",
            callback: consumer::parse,
        })
        .unwrap();
        let cases = [
            marked(json!({"item":"example:a"})),
            marked(json!({"tag":"example:ordered"})),
            marked(
                json!([{ "item":"example:c" }, {"tag":"example:ordered"}, {"item":"example:b"}]),
            ),
            marked(
                json!({"type":"forge:difference","base":{"tag":"example:ordered"},"subtracted":{"item":"example:b"}}),
            ),
            marked(json!({"tag":"example:missing"})),
            marked(json!({"tag":"example:empty"})),
            marked(json!({"item":"example:missing"})),
            marked(json!({"item":"Bad:ID"})),
            marked(
                json!({"type":"forge:difference","base":{"tag":"example:ordered"},"subtracted":{"item":"example:a"},"unexpected":true}),
            ),
            marked(json!([])),
            marked(serde_json::Value::Array(vec![
                json!({"item":"example:a"});
                17
            ])),
            json!({"type":"tconstruct:stonecutting_ingredient","value":{"item":"example:a"},"unexpected":true}),
        ];
        for v in cases {
            let expected = reference::parse(&v, &items).unwrap();
            let actual = stations::ingredient(&v, &items);
            assert_eq!(actual.order, expected.order, "{v}");
            assert_eq!(actual.items, expected.items, "{v}");
            assert!(actual.custom.is_none() && actual.children.is_none());
        }
        let mut nested = json!({"item":"example:a"});
        for _ in 0..10 {
            nested = json!([nested]);
        }
        let v = marked(nested);
        assert!(reference::parse(&v, &items).unwrap().order.is_empty());
        assert!(stations::ingredient(&v, &items).order.is_empty());
        let v = marked(json!({"tag":"example:ordered"}));
        assert_eq!(
            stations::ingredient(&v, &items).order,
            vec!["example:b", "example:a"]
        );
        assert_eq!(stations::ingredient(&v, &other).order, vec!["example:c"]);
        // Native fallback preserves nested structure, not just its flat ID list.
        let native = json!({"neoforge:ingredient_type":"neoforge:intersection","children":[["example:b","example:a"],["example:a"]]});
        assert!(stations::ingredient(&native, &items).children.is_some());
        // Missing and known-empty tags are separately visible to the callback.
        ingredients::register(Registration {
            id: "lookup-proof",
            callback: |r| {
                if r.value != &json!("lookup") {
                    return Decision::Unsupported;
                }
                assert!(r.registry.ordered_tag("example:missing").is_none());
                assert_eq!(r.registry.ordered_tag("example:empty"), Some([].as_slice()));
                assert!(!r.registry.contains_item("example:missing"));
                Decision::Handled(vec![])
            },
        })
        .unwrap();
        assert!(
            stations::ingredient(&json!("lookup"), &items)
                .order
                .is_empty()
        );
        for id in ["conflict-a", "conflict-b"] {
            ingredients::register(Registration {
                id,
                callback: |r| {
                    if r.value == &json!("example:a") {
                        Decision::Handled(vec!["example:a".into()])
                    } else {
                        Decision::Unsupported
                    }
                },
            })
            .unwrap();
        }
        assert!(
            stations::ingredient(&json!("example:a"), &items)
                .order
                .is_empty()
        );
        ingredients::register(Registration {
            id: "invalid-proof",
            callback: |r| {
                if r.value == &json!("example:b") {
                    Decision::Handled(vec!["example:missing".into()])
                } else {
                    Decision::Unsupported
                }
            },
        })
        .unwrap();
        assert!(
            stations::ingredient(&json!("example:b"), &items)
                .order
                .is_empty()
        );
    }
}
