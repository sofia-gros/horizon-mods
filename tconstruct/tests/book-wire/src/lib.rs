// Real PlutoLib graph engine under the native library export name.
#[path = "../../../../pluto-lib/mixins/pluto_native_creative/tag-graph.rs"]
pub mod pluto_tag_graph;
extern crate self as horizon_client;
pub mod text {
    #[derive(Clone, Debug, PartialEq)] pub enum ClickEvent { ChangePage(i32), RunCommand(String) }
    #[derive(Clone, Default)] pub struct Style { pub click: Option<ClickEvent>, pub insertion: Option<String>, pub color: Option<u32>, pub bold: Option<bool>, pub italic: Option<bool>, pub underlined: Option<bool>, pub strikethrough: Option<bool>, pub obfuscated: Option<bool> }
    #[derive(Clone)] pub struct Span { pub text: String, pub style: Style }
    #[derive(Clone)] pub struct Text { pub spans: Vec<Span> }
    impl Text { pub fn styled(s: impl Into<String>, style: Style) -> Self { Self { spans: vec![Span { text: s.into(), style }] } } pub fn literal(s: impl Into<String>) -> Self { Self::styled(s, Style::default()) } pub fn empty() -> Self { Self { spans: Vec::new() } } pub fn push(&mut self, other: Text) { self.spans.extend(other.spans); } }
}
#[path = "../../../mixins/tconstruct/source-materials-book-identity.rs"] mod tconstruct_materials_book_identity;
#[path = "../../../mixins/tconstruct/source-materials-book-wire.rs"] mod wire;

#[path = "../../../mixins/tconstruct/source-materials-book-recipe.rs"] mod recipe;

#[cfg(test)]
#[path = "../../../mixins/tconstruct/source-tags.rs"] mod tags;

#[cfg(test)]
mod showcase_wire_tests {
    use super::{recipe, text::{Text, Style}, wire};
    use std::collections::{HashMap, HashSet};
    #[test]
    fn pinned_seared_item_tags_and_world_replacement_feed_showcase() {
        use std::collections::BTreeMap;
        let input = serde_json::json!({"tag":"tconstruct:seared_blocks"});
        let known: HashSet<_> = ["seared_stone", "seared_cracked_bricks", "seared_cobble", "seared_paver", "seared_bricks", "seared_fancy_bricks", "seared_triangle_bricks"].into_iter().map(|name| format!("tconstruct:{name}")).collect();
        let mut raw = BTreeMap::from([
            ("tconstruct:seared_blocks".into(), vec![include_bytes!("../../../content/reference/tconstruct-tags/data/tconstruct/tags/items/seared_blocks.json").to_vec()]),
            ("tconstruct:seared_bricks".into(), vec![include_bytes!("../../../content/reference/tconstruct-tags/data/tconstruct/tags/items/seared_bricks.json").to_vec()]),
        ]);
        let resolved = super::tags::resolve_registry(&raw, &known).unwrap();
        let spec = recipe::showcase_tag(&input, Some(&resolved), Some(&known)).unwrap();
        assert_eq!(spec["showcase_items"].as_array().unwrap().len(), 7);
        raw.get_mut("tconstruct:seared_blocks").unwrap().push(br#"{"replace":true,"values":["tconstruct:seared_bricks"]}"#.to_vec());
        let replaced = super::tags::resolve_registry(&raw, &known).unwrap();
        assert_eq!(recipe::showcase_tag(&input, Some(&replaced), Some(&known)).unwrap(), serde_json::json!({"showcase_items":["tconstruct:seared_bricks"]}));
        assert_eq!(recipe::showcase_tag(&input, Some(&resolved), Some(&known)), Some(spec));
    }
    #[test]
    fn tag_choices_and_body_survive_server_reply_and_share_tooltip_selection() {
        let input = serde_json::json!({"tag":"tconstruct:seared_blocks"});
        let members = HashSet::from(["tconstruct:seared_bricks".into(), "tconstruct:seared_stone".into()]);
        let tags = HashMap::from([("tconstruct:seared_blocks".into(), members.clone())]);
        let mut spec = recipe::showcase_tag(&input, Some(&tags), Some(&members)).unwrap();
        spec["body_runs"] = serde_json::json!([{"text":"The guide body remains visible."}]);
        let pages = vec![
            Text::styled("Mighty Smelting\n", Style { insertion:Some("tconstruct:materials-book-root".into()), ..Default::default() }),
            Text::styled("Seared Blocks", Style { insertion:Some(format!("tconstruct:book:{spec}")), ..Default::default() }),
        ];
        let bytes = wire::encode_reply(7, &pages).unwrap();
        let result = wire::decode_reply(7, &pages, &bytes).unwrap();
        let insertion = result[1].spans[0].style.insertion.as_ref().unwrap();
        let decoded: serde_json::Value = serde_json::from_str(insertion.strip_prefix("tconstruct:book:").unwrap()).unwrap();
        assert_eq!(decoded, spec);
        for seconds in [0, 2, 3, 5, 6, u64::MAX] {
            assert_eq!(recipe::showcase_display_item(&decoded, seconds), recipe::showcase_display_item(&spec, seconds));
        }
        assert!(wire::decode_reply(8, &pages, &bytes).is_none());
    }
}
