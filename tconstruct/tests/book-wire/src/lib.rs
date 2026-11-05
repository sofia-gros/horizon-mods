extern crate self as horizon_client;
pub mod text {
    #[derive(Clone, Debug, PartialEq)] pub enum ClickEvent { ChangePage(i32), RunCommand(String) }
    #[derive(Clone, Default)] pub struct Style { pub click: Option<ClickEvent>, pub insertion: Option<String>, pub color: Option<u32>, pub bold: Option<bool>, pub italic: Option<bool>, pub underlined: Option<bool>, pub strikethrough: Option<bool>, pub obfuscated: Option<bool> }
    #[derive(Clone)] pub struct Span { pub text: String, pub style: Style }
    #[derive(Clone)] pub struct Text { pub spans: Vec<Span> }
    impl Text { pub fn styled(s: impl Into<String>, style: Style) -> Self { Self { spans: vec![Span { text: s.into(), style }] } } pub fn literal(s: impl Into<String>) -> Self { Self::styled(s, Style::default()) } pub fn empty() -> Self { Self { spans: Vec::new() } } pub fn push(&mut self, other: Text) { self.spans.extend(other.spans); } }
}
#[path = "../../../mixins/tconstruct/source-materials-book-wire.rs"] mod wire;

#[path = "../../../mixins/tconstruct/source-materials-book-recipe.rs"] mod recipe;

#[cfg(test)]
#[path = "../../../mixins/tconstruct/source-smeltery-structure.rs"] mod smeltery_structure;

#[cfg(test)]
#[path = "../../../mixins/tconstruct/source-tags.rs"] mod tags;

#[cfg(test)]
mod showcase_wire_tests {
    use super::{recipe, text::{Text, Style}, wire};
    use std::collections::{HashMap, HashSet};
    #[test]
    fn composite_input_material_and_all_fluid_choices_survive_book_reply() {
        let entry = recipe::material_composite_options(&["test:head".into(), "test:binding".into()],
            &["Fluid B".into(), "Fluid A".into()], "test:input", "Input", "Composite", "Cast %s on %s parts").unwrap();
        let spec = serde_json::json!({"material_panel":{"material":"test:output","sidebar":[entry.clone()],"groups":[]}});
        let pages = vec![
            Text::styled("Materials & You\n", Style { insertion:Some("tconstruct:materials-book-root".into()), ..Default::default() }),
            Text::styled("Output", Style { insertion:Some(format!("tconstruct:book:{spec}")), ..Default::default() }),
        ];
        let reply = wire::encode_reply(16, &pages).unwrap();
        let decoded = wire::decode_reply(16, &pages, &reply).unwrap();
        let spec: serde_json::Value = serde_json::from_str(decoded[1].spans[0].style.insertion.as_ref().unwrap().strip_prefix("tconstruct:book:").unwrap()).unwrap();
        assert_eq!(spec["material_panel"]["material"], "test:output");
        let paired = &spec["material_panel"]["sidebar"][0];
        for seconds in [0, 2, 3, 5, 6, 8, 9, 11, 12, u64::MAX] {
            let selected = recipe::material_sidebar_entry(paired, seconds).unwrap();
            assert_eq!(selected, recipe::material_sidebar_entry(&entry, seconds).unwrap());
            assert_eq!(selected["material"], "test:input");
        }
    }
    #[test]
    fn material_fluid_sidebar_pairs_survive_book_reply() {
        let entry = recipe::material_fluid_options(&[("test:b".into(), "Fluid B".into()), ("test:a".into(), "Fluid A".into())], "Castable", "Cast from %s").unwrap();
        let spec = serde_json::json!({"material_panel":{"material":"test:material","sidebar":[entry.clone()],"groups":[]}});
        let pages = vec![
            Text::styled("Materials & You\n", Style { insertion:Some("tconstruct:materials-book-root".into()), ..Default::default() }),
            Text::styled("Material", Style { insertion:Some(format!("tconstruct:book:{spec}")), ..Default::default() }),
        ];
        let reply = wire::encode_reply(15, &pages).unwrap();
        let decoded = wire::decode_reply(15, &pages, &reply).unwrap();
        let spec: serde_json::Value = serde_json::from_str(decoded[1].spans[0].style.insertion.as_ref().unwrap().strip_prefix("tconstruct:book:").unwrap()).unwrap();
        let paired = &spec["material_panel"]["sidebar"][0];
        for seconds in [0, 2, 3, 5, 6, u64::MAX] {
            assert_eq!(recipe::material_sidebar_entry(paired, seconds), recipe::material_sidebar_entry(&entry, seconds));
        }
    }
    #[test]
    fn drawn_small_smeltery_layers_match_the_production_detector() {
        use super::smeltery_structure::{self, Part};
        let layers = recipe::small_smeltery_layers();
        assert_eq!(layers.iter().flatten().filter(|cell| cell.is_some_and(|id| id != "minecraft:air")).count(), 5);
        let accepts = |p: [i32; 3], part: Part| {
            if !(0..3).contains(&p[0]) || !(0..2).contains(&p[1]) || !(0..3).contains(&p[2]) { return false; }
            let cell = layers[p[1] as usize][(p[2] * 3 + p[0]) as usize];
            match part {
                Part::Floor => p[1] == 0 && cell == Some("tconstruct:seared_bricks"),
                Part::Interior => cell == Some("minecraft:air"),
                Part::Wall => p[1] == 1 && cell.is_some_and(|id| id != "minecraft:air"),
            }
        };
        assert_eq!(smeltery_structure::discover([1,1,0], [0,-1], accepts), Some(([0,0,0], [2,1,2], 1080)));
        assert!(smeltery_structure::discover([1,1,0], [0,1], accepts).is_none());
        for missing in [[1,0,1], [0,1,1], [2,1,1], [1,1,2], [1,1,1]] {
            assert!(smeltery_structure::discover([1,1,0], [0,-1], |p, part| p != missing && accepts(p, part)).is_none());
        }
        let metadata = serde_json::json!({"small_smeltery":true,"body_runs":[{"text":"Static top views; north is up."}]});
        let pages = vec![
            Text::styled("Mighty Smelting\n", Style { insertion:Some("tconstruct:materials-book-root".into()), ..Default::default() }),
            Text::styled("Small Smeltery Plan", Style { insertion:Some(format!("tconstruct:book:{metadata}")), ..Default::default() }),
        ];
        let result = wire::decode_reply(14, &pages, &wire::encode_reply(14, &pages).unwrap()).unwrap();
        assert_eq!(result[1].spans[0].style.insertion, pages[1].spans[0].style.insertion);
    }
    #[test]
    fn incremental_points_and_return_type_survive_book_reply() {
        let mut spec = recipe::modifier_progress_spec(&["test:b".into(), "test:a".into()], 9, 20, Some("test:point")).unwrap();
        spec["body_runs"] = serde_json::json!([{"text":"From zero: 3 inputs, 7 returned points."}]);
        let pages = vec![
            Text::styled("Puny Smelting\n", Style { insertion:Some("tconstruct:materials-book-root".into()), ..Default::default() }),
            Text::styled("Modifier: Progress", Style { insertion:Some(format!("tconstruct:book:{spec}")), ..Default::default() }),
        ];
        let reply = wire::encode_reply(13, &pages).unwrap();
        let result = wire::decode_reply(13, &pages, &reply).unwrap();
        let decoded: serde_json::Value = serde_json::from_str(result[1].spans[0].style.insertion.as_ref().unwrap().strip_prefix("tconstruct:book:").unwrap()).unwrap();
        assert_eq!(decoded, spec);
        for seconds in [0, 2, 3, 5, 6, u64::MAX] {
            assert_eq!(recipe::modifier_progress_view(&decoded, seconds), recipe::modifier_progress_view(&spec, seconds));
        }
        assert!(wire::decode_reply(14, &pages, &reply).is_none());
    }
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
