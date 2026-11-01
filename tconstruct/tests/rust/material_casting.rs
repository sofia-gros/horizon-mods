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
            let (_, recipe, part, material) = catalog.material_casting(cast, fluid, &|_,_| true, &|_| true).unwrap();
            assert_eq!((part.as_str(), material.as_str(), recipe.fluid.amount, recipe.consumes_cast), ("tconstruct:pick_head", expected, 180, consumed));
            assert!(recipe.cooling_time > 0);
            if expected == "tconstruct:cobalt" { assert_eq!(recipe.cooling_time, 92); }
            assert!(catalog.material_casting(cast, fluid, &|_,_| false, &|_| true).is_none());
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
}
