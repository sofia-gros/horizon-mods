//! Focused predicate fixture. No client, graphics, registry downloads, or game loop.
extern crate self as horizon_world;

pub mod nbt {
    use std::collections::BTreeMap;
    #[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
    pub struct JavaString(pub Vec<u16>);
    impl From<&str> for JavaString {
        fn from(value: &str) -> Self { Self(value.encode_utf16().collect()) }
    }
    impl JavaString {
        pub fn to_string(&self) -> Result<String, std::string::FromUtf16Error> {
            String::from_utf16(&self.0)
        }
    }
    #[derive(Clone, Debug)]
    pub enum Tag { Compound(BTreeMap<JavaString, Tag>), Int(i32), String(JavaString), List(Vec<Tag>) }
    impl Tag {
        pub fn get(&self, key: &str) -> Option<&Tag> {
            match self {
                Self::Compound(fields) => fields.get(&JavaString::from(key)),
                _ => None,
            }
        }
    }
}

#[derive(Default)]
pub struct ItemView { pub id: String, pub count: u32, pub custom_data: Option<nbt::Tag> }
pub trait World { fn in_tag(&self, registry: &str, tag: &str, id: &str) -> bool; }

include!("../../../mixins/tconstruct/source-advancement-tool-predicate.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use nbt::Tag;
    use serde_json::json;
    use std::ops::ControlFlow::{Break, Continue};

    struct CurrentWorld(Vec<&'static str>);
    impl World for CurrentWorld {
        fn in_tag(&self, registry: &str, tag: &str, id: &str) -> bool {
            registry == "item" && tag == "tconstruct:modifiable" && self.0.contains(&id)
        }
    }
    fn data(fields: Vec<(&str, Tag)>) -> Tag {
        Tag::Compound(fields.into_iter().map(|(key, value)| (key.into(), value)).collect())
    }
    fn tool(upgrades: Tag) -> ItemView {
        ItemView { id: "tconstruct:pickaxe".into(), count: 1,
            custom_data: Some(data(vec![("tic_upgrades", upgrades)])) }
    }
    fn criterion() -> serde_json::Value {
        json!({"type": "tconstruct:tool_stack", "predicate": "tconstruct:has_upgrades"})
    }
    fn matches(item: &ItemView) -> bool {
        tconstruct_advancement_tool_predicate(&criterion(), item,
            &CurrentWorld(vec!["tconstruct:pickaxe"])) == Break(true)
    }

    #[test]
    fn installed_and_progressive_upgrade_amounts_match() {
        for (name, amount) in [("tconstruct:emerald", 1), ("tconstruct:haste", 18)] {
            assert!(matches(&tool(data(vec![(name, Tag::Int(amount))]))));
        }
        assert!(matches(&tool(data(vec![("example:custom_upgrade", Tag::Int(1))]))));
        assert!(matches(&tool(data(vec![("tconstruct:emerald", Tag::Int(0)),
            ("tconstruct:haste", Tag::Int(18))]))));
    }

    #[test]
    fn ordinary_items_empty_stacks_and_traits_only_do_not_match() {
        let mut item = tool(data(vec![("tconstruct:emerald", Tag::Int(1))]));
        item.id = "minecraft:diamond".into();
        assert!(!matches(&item));
        item.id = "tconstruct:pickaxe".into();
        item.count = 0;
        assert!(!matches(&item));
        item.count = 1;
        item.custom_data = None;
        assert!(!matches(&item));
        item.custom_data = Some(data(vec![("tic_materials", Tag::String("tconstruct:wood".into())),
            ("tic_modifiers", data(vec![("tconstruct:cultivated", Tag::Int(1))]))]));
        assert!(!matches(&item));
    }

    #[test]
    fn malformed_and_empty_upgrades_fail_closed() {
        for upgrades in [data(vec![]), Tag::Int(1), Tag::String("tconstruct:emerald".into()),
            data(vec![("tconstruct:emerald", Tag::Int(0))]),
            data(vec![("tconstruct:emerald", Tag::Int(-1))]),
            data(vec![("tconstruct:emerald", Tag::String("1".into()))]),
            data(vec![("emerald", Tag::Int(1))]),
            data(vec![("tconstruct:", Tag::Int(1))]),
            data(vec![(":emerald", Tag::Int(1))]),
            data(vec![("TConstruct:emerald", Tag::Int(1))]),
            data(vec![("tconstruct:bad key", Tag::Int(1))]),
            data(vec![("tconstruct:emerald", Tag::Int(1)), ("tconstruct:haste", Tag::Int(-1))]),
            data(vec![("tconstruct:emerald", Tag::Int(1)), ("invalid", Tag::Int(1))])] {
            assert!(!matches(&tool(upgrades)));
        }
        let mut item = tool(data(vec![("tconstruct:emerald", Tag::Int(1))]));
        item.custom_data = Some(Tag::Int(1));
        assert!(!matches(&item));
        let invalid_utf16 = Tag::Compound(std::collections::BTreeMap::from([
            (nbt::JavaString(vec![0xd800]), Tag::Int(1))]));
        assert!(!matches(&tool(invalid_utf16)));
    }

    #[test]
    fn world_tag_membership_is_not_a_fixed_tool_allowlist() {
        let item = tool(data(vec![("tconstruct:emerald", Tag::Int(1))]));
        assert_eq!(tconstruct_advancement_tool_predicate(&criterion(), &item, &CurrentWorld(vec![])), Break(false));
        let mut replacement = item;
        replacement.id = "example:world_tool".into();
        assert_eq!(tconstruct_advancement_tool_predicate(&criterion(), &replacement,
            &CurrentWorld(vec!["example:world_tool"])), Break(true));
    }

    #[test]
    fn unsupported_tool_predicates_do_not_fall_through_to_native_matcher() {
        let item = tool(data(vec![("tconstruct:emerald", Tag::Int(1))]));
        for predicate in [serde_json::Value::Null, json!("tconstruct:has_material"),
            json!({"type": "tconstruct:has_upgrades"}), json!([])] {
            assert_eq!(tconstruct_advancement_tool_predicate(
                &json!({"type": "tconstruct:tool_stack", "predicate": predicate}),
                &item, &CurrentWorld(vec!["tconstruct:pickaxe"])), Break(false));
        }
        assert_eq!(tconstruct_advancement_tool_predicate(&json!({"type": "tconstruct:tool_stack"}),
            &item, &CurrentWorld(vec!["tconstruct:pickaxe"])), Break(false));
    }

    #[test]
    fn unrelated_item_predicates_keep_native_semantics() {
        for predicate in [serde_json::Value::Null, json!({}), json!({"items": "minecraft:redstone"}),
            json!({"type": "example:custom", "predicate": "tconstruct:has_upgrades"})] {
            assert_eq!(tconstruct_advancement_tool_predicate(&predicate, &ItemView::default(),
                &CurrentWorld(vec![])), Continue(()));
        }
    }

    fn materials(values: &[&str]) -> ItemView {
        ItemView { id: "tconstruct:pickaxe".into(), count: 1,
            custom_data: Some(data(vec![("tic_materials", Tag::List(values.iter()
                .map(|id| Tag::String((*id).into())).collect()))])) }
    }
    fn has_material(item: &ItemView, predicate: serde_json::Value) -> bool {
        tconstruct_advancement_tool_predicate(
            &json!({"type": "tconstruct:tool_stack", "predicate": predicate}), item,
            &CurrentWorld(vec!["tconstruct:pickaxe"])) == Break(true)
    }
    fn material(value: &str) -> serde_json::Value {
        json!({"type": "tconstruct:has_material", "material": value})
    }

    #[test]
    fn material_base_ids_accept_variants_but_not_other_bases() {
        let item = materials(&["tconstruct:cobalt", "tconstruct:wood#oak"]);
        assert!(has_material(&item, material("tconstruct:wood")));
        assert!(has_material(&item, material("tconstruct:cobalt")));
        assert!(!has_material(&item, material("tconstruct:manyullyn")));
        assert!(!has_material(&item, material("tconstruct:wood#birch")));
        assert!(has_material(&item, material("tconstruct:wood#oak")));
        assert!(!has_material(&item, material("tconstruct:wood#default")));
        assert!(has_material(&item, material("tconstruct:cobalt#default")));
    }

    #[test]
    fn material_indexes_are_exact_and_minus_one_scans_all_slots() {
        let item = materials(&["tconstruct:cobalt", "tconstruct:wood#oak"]);
        for (index, expected) in [(-1, true), (0, false), (1, true), (2, false)] {
            let mut predicate = material("tconstruct:wood");
            predicate["index"] = json!(index);
            assert_eq!(has_material(&item, predicate), expected);
        }
        for index in [json!(-2), json!(2147483648_i64), json!(1.5), json!("1"), serde_json::Value::Null] {
            let mut predicate = material("tconstruct:wood");
            predicate["index"] = index;
            assert!(!has_material(&item, predicate));
        }
    }

    #[test]
    fn material_match_keeps_item_tag_and_empty_stack_gates() {
        let mut item = materials(&["tconstruct:cobalt"]);
        item.count = 0;
        assert!(!has_material(&item, material("tconstruct:cobalt")));
        item.count = 1;
        item.id = "minecraft:diamond".into();
        assert!(!has_material(&item, material("tconstruct:cobalt")));
        let item = tool(data(vec![("tconstruct:emerald", Tag::Int(1))]));
        assert!(!has_material(&item, material("tconstruct:cobalt")));
        assert!(!has_material(&materials(&[]), material("tconstruct:cobalt")));
    }

    #[test]
    fn malformed_material_lists_and_unsupported_predicates_fail_closed() {
        for invalid in ["wood", "tconstruct:", "tconstruct:wood#", "tconstruct:wood#bad#variant",
            "tconstruct:Wood", "tconstruct:wood#bad variant"] {
            assert!(!has_material(&materials(&["tconstruct:cobalt", invalid]), material("tconstruct:cobalt")));
            assert!(!has_material(&materials(&["tconstruct:cobalt"]), material(invalid)));
        }
        for invalid in [Tag::Int(1), Tag::String(nbt::JavaString(vec![0xd800]))] {
            let item = ItemView { id: "tconstruct:pickaxe".into(), count: 1,
                custom_data: Some(data(vec![("tic_materials", Tag::List(vec![
                    Tag::String("tconstruct:cobalt".into()), invalid]))])) };
            assert!(!has_material(&item, material("tconstruct:cobalt")));
        }
        for value in [serde_json::Value::Null, json!(["tconstruct:cobalt"]),
            json!({"type": "mantle:tag", "tag": "tconstruct:metal"})] {
            let mut predicate = material("tconstruct:cobalt");
            predicate["material"] = value;
            assert!(!has_material(&materials(&["tconstruct:cobalt"]), predicate));
        }
        assert!(!has_material(&materials(&["tconstruct:cobalt"]), json!({"type": "tconstruct:has_material"})));
    }
}
