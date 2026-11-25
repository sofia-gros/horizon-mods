//! Actual map-transfer helper, with identity JSON transport in place of native
//! json_to_tag. This checks snapshot ownership/selection, not native NBT widths
//! or the direct native final-map attachment (which needs native validation).
use std::{collections::{BTreeMap, HashMap}, sync::Arc};
use serde_json::{json, Value};
mod sunrise_world { pub mod item_defaults { pub type Defaults = std::collections::BTreeMap<String, serde_json::Value>; } }
mod sunrise_client_core { pub mod scoreboard { pub fn json_to_tag(value: &serde_json::Value) -> serde_json::Value { value.clone() } } }
#[derive(Default)]
struct ItemInfo { custom_components: BTreeMap<String, Value>, max_damage: u32 }
include!("../../../mod/src/pluto-item-prototypes.rs");

#[test]
fn actual_item_map_transfer_uses_held_defaults_and_keeps_missing_invalid_and_other_data_distinct() {
    let source = Arc::new(BTreeMap::from([
        ("test:tool".into(), json!({"minecraft:custom_data":{"generation":1,"values":[1,"x",true]}})),
        ("test:plain".into(), json!({})),
        ("test:invalid".into(), json!({"minecraft:custom_data":"not a compound"})),
    ]));
    let before = source.as_ref().clone();
    let newer: Arc<BTreeMap<String, Value>> = Arc::new(BTreeMap::from([("test:tool".into(), json!({"minecraft:custom_data":{"generation":2}}))]));
    let items = HashMap::from([
        (Arc::from("test:tool"), ItemInfo { max_damage:41, custom_components:BTreeMap::from([("other:component".into(),json!([7,8]))]) }),
        (Arc::from("test:plain"), ItemInfo::default()),
        (Arc::from("test:invalid"), ItemInfo::default()),
    ]);
    let mut items = pluto_item_prototypes(items, &source);
    assert_eq!(items["test:tool"].custom_components["minecraft:custom_data"], source["test:tool"]["minecraft:custom_data"]);
    assert_ne!(items["test:tool"].custom_components["minecraft:custom_data"], newer["test:tool"]["minecraft:custom_data"]);
    assert_eq!(items["test:tool"].custom_components["other:component"], json!([7,8]));
    assert_eq!(items["test:tool"].max_damage, 41);
    assert!(!items["test:plain"].custom_components.contains_key("minecraft:custom_data"));
    assert_eq!(items["test:invalid"].custom_components["minecraft:custom_data"], json!("not a compound"));
    items.get_mut("test:tool").unwrap().custom_components.clear();
    assert_eq!(*source, before);
    assert_eq!(source["test:tool"]["minecraft:custom_data"]["generation"], 1);
}

