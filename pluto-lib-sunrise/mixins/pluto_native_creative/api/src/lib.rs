//! Host-independent Creative, item-use and experimental world-fluid APIs for native mods.
pub mod creative;
pub mod item;
pub mod resources;
pub mod fluid;
pub mod effects;

#[cfg(test)]
mod public_api_tests {
    use super::creative::{register_tabs, Entry, Tab};
    use super::item::{ItemBehavior, ItemData, UseOutcome};
    use serde_json::json;

    struct Example;
    impl ItemBehavior for Example {
        fn on_use(&self, data: &mut ItemData) -> UseOutcome {
            data.set("uses", 1);
            UseOutcome::Handled
        }
    }

    #[test]
    fn creative_and_item_apis_share_one_public_crate() {
        let input = json!({"tabs": []});
        let tab = Tab {
            id: "example:items".into(),
            title: "Example".into(),
            icon: "example:wand".into(),
            items: vec![Entry {
                snbt: "example:wand".into(),
                searchable: false,
                permissions_only: false,
                search_lines: vec![],
                search_tags: vec![],
            }],
        };
        let output = register_tabs(&input, &[tab], |s| Ok(s.to_owned())).unwrap();
        assert_eq!(output["tabs"][0]["items"][0], "example:wand");
        let mut data = ItemData::default();
        assert_eq!(Example.on_use(&mut data), UseOutcome::Handled);
        assert_eq!(data.get("uses"), Some(1));
    }
}
