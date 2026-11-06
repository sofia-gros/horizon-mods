extern crate self as horizon_client;
// Link the real shared package; do not substitute a predicate implementation.
pub use pluto_inventory_api as pluto_inventory;
// Real PlutoLib graph engine under the native library export name.
#[path = "../../../pluto-lib/mixins/pluto_native_creative/tag-graph.rs"]
pub mod pluto_tag_graph;
#[path = "../../mixins/tconstruct/source-tags.rs"] mod tconstruct_source_tags;
#[path = "../../mixins/tconstruct/source-melting-recipes.rs"] mod tconstruct_source_melting_recipes;
#[path = "../../mixins/tconstruct/source-melting-lookup.rs"] mod tconstruct_source_melting_lookup;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Read;

#[test]
fn pinned_material_casts_join_multiple_fluids_and_preserve_source_costs() {
    let mut zip = zip::ZipArchive::new(std::fs::File::open("tconstruct/content/reference/tconstruct-recipes.zip").unwrap()).unwrap();
    let mut recipes = BTreeMap::new();
    for name in ["data/tconstruct/recipes/tools/parts/casting/pick_head_gold_cast.json", "data/tconstruct/recipes/tools/parts/casting/pick_head_sand_cast.json", "data/tconstruct/recipes/tools/materials/casting/cobalt.json", "data/tconstruct/recipes/tools/materials/casting/iron.json"] {
        let mut file = zip.by_name(name).unwrap(); let mut bytes = Vec::new(); file.read_to_end(&mut bytes).unwrap(); recipes.insert(name.to_owned(), bytes);
    }
    let item_tags: HashMap<String,HashSet<String>> = [
        ("tconstruct:casts/multi_use/pick_head".into(), ["tconstruct:pick_head_cast".into()].into_iter().collect()),
        ("tconstruct:casts/single_use/pick_head".into(), ["tconstruct:pick_head_sand_cast".into()].into_iter().collect()),
    ].into_iter().collect();
    let fluid_tags: HashMap<String,HashSet<String>> = [
        ("forge:molten_cobalt".into(), ["tconstruct:molten_cobalt".into()].into_iter().collect()),
        ("forge:molten_iron".into(), ["tconstruct:molten_iron".into()].into_iter().collect()),
    ].into_iter().collect();
    let known_items = ["tconstruct:pick_head", "tconstruct:pick_head_cast", "tconstruct:pick_head_sand_cast"].into_iter().map(str::to_owned).collect();
    let known_fluids = ["tconstruct:molten_cobalt", "tconstruct:molten_iron"].into_iter().map(str::to_owned).collect();
    let catalog = tconstruct_source_melting_lookup::Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &fluid_tags, known_items: &known_items, known_fluids: &known_fluids };
    for (cast, consumed) in [("tconstruct:pick_head_cast", false), ("tconstruct:pick_head_sand_cast", true)] {
        for (fluid, expected) in [("tconstruct:molten_cobalt", "tconstruct:cobalt"), ("tconstruct:molten_iron", "tconstruct:iron")] {
            let (_, recipe, part, material) = catalog.material_casting(cast, fluid, &|_,_| true, &|_,_| true, &|_| true).unwrap();
            assert_eq!((part.as_str(), material.as_str(), recipe.fluid.amount, recipe.consumes_cast), ("tconstruct:pick_head", expected, 180, consumed));
            assert!(recipe.cooling_time > 0);
            if expected == "tconstruct:cobalt" { assert_eq!(recipe.cooling_time, 92); }
            assert!(catalog.material_casting(cast, fluid, &|_,_| false, &|_,_| true, &|_| true).is_none());
        }
    }
}

#[test]
fn original_gold_cast_creation_accepts_the_material_part() {
    use tconstruct_source_melting_recipes::CastingRecipe;
    let mut zip = zip::ZipArchive::new(std::fs::File::open("tconstruct/content/reference/tconstruct-recipes.zip").unwrap()).unwrap();
    let mut entry = zip.by_name("data/tconstruct/recipes/smeltery/casts/gold/pick_head.json").unwrap();
    let mut bytes = Vec::new(); entry.read_to_end(&mut bytes).unwrap();
    let recipe = CastingRecipe::parse(&bytes, &|_| true).unwrap();
    assert!(recipe.cast.as_ref().unwrap().matches("tconstruct:pick_head", &|_,_| false));
    assert!(!recipe.cast.as_ref().unwrap().matches("tconstruct:tool_handle", &|_,_| false));
    assert!(recipe.consumes_cast && recipe.switch_slots);
    assert_eq!(recipe.fluid.amount, 90);
    let recipes = [("source:gold_pick_head_cast".to_owned(), bytes)].into_iter().collect();
    let item_tags = HashMap::new();
    let fluid_tags = [("forge:molten_gold".to_owned(), ["tconstruct:molten_gold".to_owned()].into_iter().collect())].into_iter().collect();
    let known_items = ["tconstruct:pick_head", "tconstruct:pick_head_cast"].into_iter().map(str::to_owned).collect();
    let known_fluids = ["tconstruct:molten_gold".to_owned()].into_iter().collect();
    let catalog = tconstruct_source_melting_lookup::Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &fluid_tags, known_items: &known_items, known_fluids: &known_fluids };
    let (_, recipe, result) = catalog.casting(false, Some("tconstruct:pick_head"), "tconstruct:molten_gold", &|_| true).unwrap();
    assert_eq!(result, "tconstruct:pick_head_cast");
    assert!(recipe.consumes_cast && recipe.switch_slots);
}

#[test]
fn pinned_composite_converts_one_existing_part_and_preserves_variant() {
    let mut zip = zip::ZipArchive::new(std::fs::File::open("tconstruct/content/reference/tconstruct-recipes.zip").unwrap()).unwrap();
    let mut recipes = BTreeMap::new();
    for name in ["data/tconstruct/recipes/tools/parts/casting/pick_head_composite.json", "data/tconstruct/recipes/tools/materials/composite/copper_oxidized.json"] {
        let mut file = zip.by_name(name).unwrap(); let mut bytes = Vec::new(); file.read_to_end(&mut bytes).unwrap(); recipes.insert(name.to_owned(), bytes);
    }
    let item_tags = HashMap::new();
    let fluid_tags = [("mantle:water".into(), ["minecraft:water".into()].into_iter().collect())].into_iter().collect();
    let known_items = ["tconstruct:pick_head".into()].into_iter().collect();
    let known_fluids = ["minecraft:water".into()].into_iter().collect();
    let catalog = tconstruct_source_melting_lookup::Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &fluid_tags, known_items: &known_items, known_fluids: &known_fluids };
    let resolve = |input: &str| catalog.composite_casting(false, "tconstruct:pick_head", input, "minecraft:water", &|_,_| true, &|_,_| true, &|_,_| Some(false), &|_| true);
    let (_, recipe, item, material) = resolve("tconstruct:copper").unwrap();
    assert_eq!((item.as_str(), material.as_str(), recipe.fluid.amount), ("tconstruct:pick_head", "tconstruct:copper#oxidized", 500));
    assert!(recipe.consumes_cast); assert!(!recipe.switch_slots);
    assert!(resolve("tconstruct:copper#oxidized").is_none());
    assert!(resolve("tconstruct:iron").is_none());
    assert!(catalog.composite_casting(true, "tconstruct:pick_head", "tconstruct:copper", "minecraft:water", &|_,_| true, &|_,_| true, &|_,_| Some(false), &|_| true).is_none());
}

#[test]
fn source_material_predicates_reject_unknown_nested_conditions() {
    use tconstruct_source_melting_lookup::material_predicate;
    let tags = |tag: &str, id: &str| tag == "tconstruct:metal" && id == "tconstruct:copper";
    let parts = |_: &str, part: &str| Some(part == "tconstruct:pick_head");
    let predicate = serde_json::json!({"type":"mantle:and","predicates":[
        {"type":"mantle:tag","tag":"tconstruct:metal"},
        {"type":"tconstruct:has_part","part":"tconstruct:pick_head"},
        {"type":"mantle:inverted","inverted_type":"tconstruct:variant","variant":"tconstruct:copper#oxidized"}
    ]});
    assert_eq!(material_predicate(Some(&predicate), "tconstruct:copper", &tags, &parts, &|_,_,_| None), Some(true));
    assert_eq!(material_predicate(Some(&predicate), "tconstruct:copper#oxidized", &tags, &parts, &|_,_,_| None), Some(false));
    assert_eq!(material_predicate(Some(&serde_json::json!("mantle:any")), "tconstruct:copper", &tags, &parts, &|_,_,_| None), Some(true));
    let unknown = serde_json::json!({"type":"mantle:inverted","inverted_type":"example:unsupported"});
    assert_eq!(material_predicate(Some(&unknown), "tconstruct:copper", &tags, &parts, &|_,_,_| None), None);
    let missing = serde_json::json!({"type":"mantle:inverted","inverted_type":"tconstruct:has_part","part":"tconstruct:pick_head"});
    assert_eq!(material_predicate(Some(&missing), "tconstruct:copper", &tags, &|_,_| None, &|_,_,_| None), None);
}

#[test]
fn source_shield_conflict_uses_normal_casting_material_stat() {
    let composite = br#"{"type":"tconstruct:table_casting_composite","item_cost":4,"result":"tconstruct:shield_core","casting_stat_conflict":"tconstruct:plating_shield"}"#;
    let transform = br#"{"type":"tconstruct:material_fluid","input":"tconstruct:wood","output":"tconstruct:nahuatl","temperature":1000,"fluid":{"fluid":"tconstruct:molten_obsidian","amount":250}}"#;
    let normal = br#"{"type":"tconstruct:material_fluid","output":"tconstruct:obsidian","temperature":1000,"fluid":{"fluid":"tconstruct:molten_obsidian","amount":250}}"#;
    let recipes = [("part".to_owned(), composite.to_vec()), ("transform".to_owned(), transform.to_vec()), ("normal".to_owned(), normal.to_vec())].into_iter().collect();
    let tags = HashMap::new(); let items = ["tconstruct:shield_core".into()].into_iter().collect(); let fluids = ["tconstruct:molten_obsidian".into()].into_iter().collect();
    let catalog = tconstruct_source_melting_lookup::Catalog { recipes: &recipes, item_tags: &tags, fluid_tags: &tags, known_items: &items, known_fluids: &fluids };
    let resolve = |conflict| catalog.composite_casting(false, "tconstruct:shield_core", "tconstruct:wood", "tconstruct:molten_obsidian", &|_,_| true, &|_,_| true, &|_,_| conflict, &|_| true);
    assert!(resolve(Some(true)).is_none()); assert!(resolve(None).is_none()); assert!(resolve(Some(false)).is_some());
}

#[test]
fn composite_variant_wildcard_and_precise_variant_follow_source() {
    let part = br#"{"type":"tconstruct:table_casting_composite","item_cost":1,"result":"tconstruct:tool_handle"}"#;
    let transform = br#"{"type":"tconstruct:material_fluid","input":"tconstruct:wood#oak","output":"tconstruct:nahuatl","temperature":1000,"fluid":{"fluid":"tconstruct:molten_obsidian","amount":250}}"#;
    let recipes = [("part".to_owned(), part.to_vec()), ("fluid".to_owned(), transform.to_vec())].into_iter().collect();
    let tags = HashMap::new(); let items = ["tconstruct:tool_handle".into()].into_iter().collect(); let fluids = ["tconstruct:molten_obsidian".into()].into_iter().collect();
    let catalog = tconstruct_source_melting_lookup::Catalog { recipes: &recipes, item_tags: &tags, fluid_tags: &tags, known_items: &items, known_fluids: &fluids };
    let resolve = |input| catalog.composite_casting(false, "tconstruct:tool_handle", input, "tconstruct:molten_obsidian", &|_,_| true, &|_,_| true, &|_,_| Some(false), &|_| true);
    assert!(resolve("tconstruct:wood#oak").is_some()); assert!(resolve("tconstruct:wood#spruce").is_none()); assert!(resolve("tconstruct:wood").is_none());
}

#[test]
fn source_composite_uses_item_cost_conditions_and_part_compatibility() {
    let mut zip = zip::ZipArchive::new(std::fs::File::open("tconstruct/content/reference/tconstruct-recipes.zip").unwrap()).unwrap();
    let mut recipes = BTreeMap::new();
    for name in ["data/tconstruct/recipes/tools/parts/casting/tool_handle_composite.json", "data/tconstruct/recipes/tools/materials/composite/nahuatl.json"] {
        let mut file = zip.by_name(name).unwrap(); let mut bytes = Vec::new(); file.read_to_end(&mut bytes).unwrap(); recipes.insert(name.to_owned(), bytes);
    }
    let item_tags = HashMap::new();
    let fluid_tags = [("tconstruct:molten_obsidian".into(), ["tconstruct:molten_obsidian".into()].into_iter().collect())].into_iter().collect();
    let known_items = ["tconstruct:tool_handle".into()].into_iter().collect();
    let known_fluids = ["tconstruct:molten_obsidian".into()].into_iter().collect();
    let catalog = tconstruct_source_melting_lookup::Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &fluid_tags, known_items: &known_items, known_fluids: &known_fluids };
    let (_, recipe, _, material) = catalog.composite_casting(false, "tconstruct:tool_handle", "tconstruct:wood#oak", "tconstruct:molten_obsidian", &|_,_| true, &|_,_| true, &|_,_| Some(false), &|_| true).unwrap();
    assert_eq!((material.as_str(),recipe.fluid.amount), ("tconstruct:nahuatl",250));
    assert!(catalog.composite_casting(false, "tconstruct:tool_handle", "tconstruct:wood", "tconstruct:molten_obsidian", &|_,_| false, &|_,_| true, &|_,_| Some(false), &|_| true).is_none());
    assert!(catalog.composite_casting(false, "tconstruct:tool_handle", "tconstruct:wood", "tconstruct:molten_obsidian", &|_,_| true, &|_,_| false, &|_,_| Some(false), &|_| true).is_none());
}

#[test]
fn conditional_composite_does_not_bypass_recipe_conditions() {
    let part = br#"{"type":"tconstruct:table_casting_composite","item_cost":1,"result":"tconstruct:tool_handle"}"#;
    let fluid = br#"{"type":"tconstruct:material_fluid","conditions":[{"type":"mantle:tag_filled","tag":"forge:ingots/uranium"}],"input":"tconstruct:wood","output":"tconstruct:necronium","temperature":830,"fluid":{"fluid":"tconstruct:molten_obsidian","amount":90}}"#;
    let recipes = [("part".to_owned(), part.to_vec()), ("fluid".to_owned(), fluid.to_vec())].into_iter().collect();
    let tags = HashMap::new(); let items = ["tconstruct:tool_handle".into()].into_iter().collect(); let fluids = ["tconstruct:molten_obsidian".into()].into_iter().collect();
    let catalog = tconstruct_source_melting_lookup::Catalog { recipes: &recipes, item_tags: &tags, fluid_tags: &tags, known_items: &items, known_fluids: &fluids };
    assert!(catalog.composite_casting(false, "tconstruct:tool_handle", "tconstruct:wood", "tconstruct:molten_obsidian", &|_,_| true, &|_,_| true, &|_,_| Some(false), &|_| false).is_none());
}

#[test]
fn source_mantle_water_tag_resolves_for_tank_fluids() {
    let mut raw = BTreeMap::new();
    tconstruct_source_tags::merge_mantle_fluid_tags(&mut raw);
    let (fluids, tags) = tconstruct_source_melting_lookup::tank_fluid_tags(&raw, &HashSet::new()).unwrap();
    assert!(fluids.contains("minecraft:water"));
    assert!(tags["mantle:water"].contains("minecraft:water"));
    assert!(tags["mantle:water"].contains("minecraft:flowing_water"));
}
