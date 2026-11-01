#[path = "../../mixins/tconstruct/source-tags.rs"] mod tconstruct_source_tags;
#[path = "../../mixins/tconstruct/source-melting-recipes.rs"] mod tconstruct_source_melting_recipes;
#[path = "../../mixins/tconstruct/source-melting-lookup.rs"] mod tconstruct_source_melting_lookup;

#[test]
fn pinned_controller_recipes_accept_only_currently_rendered_brick_variants() {
    use std::collections::{BTreeMap, HashMap, HashSet};
    use std::io::Read;
    let file = std::fs::File::open("tconstruct/content/reference/tconstruct-recipes.zip").unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();
    let mut recipes = BTreeMap::new();
    for index in 0..zip.len() {
        let mut file = zip.by_index(index).unwrap();
        let name = file.name().to_owned();
        if !name.starts_with("data/tconstruct/recipes/") || !name.ends_with(".json") { continue; }
        let mut bytes = Vec::new(); file.read_to_end(&mut bytes).unwrap(); recipes.insert(name, bytes);
    }
    let items: HashSet<String> = ["tconstruct:seared_bricks", "tconstruct:seared_cobble", "tconstruct:scorched_bricks", "tconstruct:scorched_stone", "tconstruct:smeltery_controller", "tconstruct:foundry_controller"].into_iter().map(str::to_owned).collect();
    let fluids: HashSet<String> = ["tconstruct:molten_copper", "tconstruct:molten_obsidian"].into_iter().map(str::to_owned).collect();
    let item_tags: HashMap<String, HashSet<String>> = [
        ("tconstruct:smeltery_bricks", ["tconstruct:seared_bricks", "tconstruct:seared_cobble"]),
        ("tconstruct:foundry_bricks", ["tconstruct:scorched_bricks", "tconstruct:scorched_stone"]),
    ].into_iter().map(|(k,v)| (k.to_owned(), v.into_iter().map(str::to_owned).collect())).collect();
    let fluid_tags: HashMap<String, HashSet<String>> = [
        ("forge:molten_copper", ["tconstruct:molten_copper"]),
        ("tconstruct:molten_obsidian", ["tconstruct:molten_obsidian"]),
    ].into_iter().map(|(k,v)| (k.to_owned(), v.into_iter().map(str::to_owned).collect())).collect();
    let catalog = tconstruct_source_melting_lookup::Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &fluid_tags, known_items: &items, known_fluids: &fluids };
    for (cast, fluid, output, amount, time) in [
        ("tconstruct:seared_bricks", "tconstruct:molten_copper", "tconstruct:smeltery_controller", 360, 100),
        ("tconstruct:scorched_bricks", "tconstruct:molten_obsidian", "tconstruct:foundry_controller", 1000, 221),
    ] {
        let (_, recipe, result) = catalog.casting(true, Some(cast), fluid, &|_| true).unwrap();
        assert!(recipe.retextured && recipe.consumes_cast);
        assert_eq!((result.as_str(), recipe.fluid.amount, recipe.cooling_time), (output, amount, time));
    }
    assert!(catalog.casting(true, Some("tconstruct:seared_cobble"), "tconstruct:molten_copper", &|_| true).is_none());
    assert!(catalog.casting(true, Some("tconstruct:scorched_stone"), "tconstruct:molten_obsidian", &|_| true).is_none());
}
