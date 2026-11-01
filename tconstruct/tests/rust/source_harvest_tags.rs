#[path = "../../mixins/tconstruct/source-tags.rs"] mod tconstruct_source_tags;
#[path = "../../mixins/tconstruct/source-harvest-tags.rs"] mod tconstruct_source_harvest_tags;

#[test]
fn prepares_source_predicates_without_mutating_vanilla_tags() {
    use std::path::Path;
    fn put(root: &Path, name: &str, bytes: &[u8]) {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    let root = std::env::temp_dir().join(format!("tconstruct-harvest-fixture-{}", std::process::id()));
    assert!(!root.exists());
    std::fs::create_dir_all(&root).unwrap();
    let original = root.join("original");
    let vanilla = br#"{"values":["minecraft:stone"]}"#;
    put(&root, "data/minecraft/tags/block/mineable/pickaxe.json", vanilla);
    put(&root, "data/minecraft/tags/block/mineable/shovel.json", br#"{"values":["minecraft:dirt"]}"#);
    put(&root, "data/minecraft/tags/block/logs.json", br#"{"values":["minecraft:oak_log"]}"#);
    put(&root, "facts.json", br#"{"states":[["minecraft:stone"],["minecraft:dirt"],["minecraft:oak_log"],["minecraft:oak_planks"],["tconstruct:greenheart_log"]]}"#);
    put(&original, "data/minecraft/tags/blocks/logs.json", br#"{"values":["tconstruct:greenheart_log"]}"#);
    put(&original, "data/tconstruct/tags/blocks/mineable/mattock.json", br##"{"values":["#minecraft:mineable/shovel","#minecraft:logs","tconstruct:not_registered"]}"##);
    put(&original, "data/tconstruct/tinkering/tool_definitions/pickaxe.json", br#"{"modules":[{"type":"tconstruct:is_effective","predicate_type":"mantle:tag","tag":"minecraft:mineable/pickaxe"}]}"#);
    put(&original, "data/tconstruct/tinkering/tool_definitions/mattock.json", br#"{"modules":[{"type":"tconstruct:is_effective","predicate_type":"mantle:tag","tag":"tconstruct:mineable/mattock"}]}"#);
    put(&original, "data/tconstruct/tags/blocks/tree_log.json", br##"{"values":["#minecraft:logs"]}"##);
    put(&original, "data/tconstruct/tinkering/tool_definitions/broad_axe.json", br#"{"modules":[{"type":"tconstruct:conditional_aoe","condition":{"type":"mantle:tag","tag":"tconstruct:tree_log"},"if_true":{"type":"tconstruct:tree_aoe"}}]}"#);
    put(&original, "data/tconstruct/tags/blocks/harvestable/crops.json", br#"{"values":["minecraft:dirt"]}"#);
    put(&original, "data/tconstruct/tags/blocks/harvestable/interact.json", br#"{"values":["minecraft:stone"]}"#);
    let forge = Path::new("tconstruct/content/reference/forge-47.4.0-harvest-tags.zip");
    assert_eq!(tconstruct_source_harvest_tags::prepare(&root, &original, forge).unwrap(), 5);
    assert_eq!(std::fs::read(root.join("data/minecraft/tags/block/mineable/pickaxe.json")).unwrap(), vanilla);
    let path = root.join("data/tconstruct/tags/block/source_effective/tconstruct/mineable/mattock.json");
    let result: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let values = result["values"].as_array().unwrap();
    assert!(values.contains(&serde_json::json!("minecraft:dirt")));
    assert!(values.contains(&serde_json::json!("minecraft:oak_log")));
    assert!(values.contains(&serde_json::json!("tconstruct:greenheart_log")));
    assert!(!values.contains(&serde_json::json!("minecraft:oak_planks")));
    assert!(!values.contains(&serde_json::json!("tconstruct:not_registered")));
    let tree: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join("data/tconstruct/tags/block/source_effective/tconstruct/tree_log.json")).unwrap()).unwrap();
    assert!(tree["values"].as_array().unwrap().contains(&serde_json::json!("minecraft:oak_log")));
    assert!(tree["values"].as_array().unwrap().contains(&serde_json::json!("tconstruct:greenheart_log")));
    assert_eq!(tconstruct_source_harvest_tags::alias("minecraft:../bad"), None);
    std::fs::remove_dir_all(root).unwrap();
}
