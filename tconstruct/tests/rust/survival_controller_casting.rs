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
    let mut items: HashSet<String> = ["tconstruct:seared_bricks", "tconstruct:seared_cobble", "tconstruct:scorched_bricks", "tconstruct:scorched_stone", "tconstruct:smeltery_controller", "tconstruct:foundry_controller"].into_iter().map(str::to_owned).collect();
    let mut fluids: HashSet<String> = ["tconstruct:molten_copper", "tconstruct:molten_obsidian"].into_iter().map(str::to_owned).collect();
    let mut item_tags: HashMap<String, HashSet<String>> = [
        ("tconstruct:smeltery_bricks", ["tconstruct:seared_bricks", "tconstruct:seared_cobble"]),
        ("tconstruct:foundry_bricks", ["tconstruct:scorched_bricks", "tconstruct:scorched_stone"]),
    ].into_iter().map(|(k,v)| (k.to_owned(), v.into_iter().map(str::to_owned).collect())).collect();
    let mut fluid_tags: HashMap<String, HashSet<String>> = [
        ("forge:molten_copper", ["tconstruct:molten_copper"]),
        ("tconstruct:molten_obsidian", ["tconstruct:molten_obsidian"]),
    ].into_iter().map(|(k,v)| (k.to_owned(), v.into_iter().map(str::to_owned).collect())).collect();
    items.extend(["minecraft:copper_ingot", "minecraft:gold_ingot", "tconstruct:rose_gold_block"].into_iter().map(str::to_owned));
    fluids.extend(["tconstruct:molten_gold", "tconstruct:molten_rose_gold"].into_iter().map(str::to_owned));
    for (tag, member) in [("forge:ingots/copper", "minecraft:copper_ingot"), ("forge:ingots/gold", "minecraft:gold_ingot"), ("forge:storage_blocks/rose_gold", "tconstruct:rose_gold_block")] {
        item_tags.insert(tag.to_owned(), [member.to_owned()].into_iter().collect());
    }
    for (tag, member) in [("forge:molten_gold", "tconstruct:molten_gold"), ("forge:molten_rose_gold", "tconstruct:molten_rose_gold")] {
        fluid_tags.insert(tag.to_owned(), [member.to_owned()].into_iter().collect());
    }
    let catalog = tconstruct_source_melting_lookup::Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &fluid_tags, known_items: &items, known_fluids: &fluids };
    for (cast, fluid, output, amount, time) in [
        ("tconstruct:seared_bricks", "tconstruct:molten_copper", "tconstruct:smeltery_controller", 360, 100),
        ("tconstruct:scorched_bricks", "tconstruct:molten_obsidian", "tconstruct:foundry_controller", 1000, 221),
    ] {
        let (_, recipe, result) = catalog.casting(true, Some(cast), fluid, &|_| true).unwrap();
        assert!(recipe.retextured && recipe.consumes_cast);
        assert_eq!((result.as_str(), recipe.fluid.amount, recipe.cooling_time), (output, amount, time));
    }
    for (item, expected) in [("minecraft:copper_ingot", "tconstruct:molten_copper"), ("minecraft:gold_ingot", "tconstruct:molten_gold")] {
        let (_, recipe, fluid) = catalog.melting(item, &|_| true).unwrap();
        assert_eq!((fluid.as_str(), recipe.output.amount), (expected, 90));
    }
    let inputs = [("tconstruct:molten_copper".to_owned(), 90), ("tconstruct:molten_gold".to_owned(), 90)];
    let (_, _, alloy, amount) = catalog.alloy(&inputs, 2160, 1000, &|_| true).unwrap();
    assert_eq!((alloy.as_str(), amount), ("tconstruct:molten_rose_gold", 180));
    let (_, block, result) = catalog.casting(true, None, "tconstruct:molten_rose_gold", &|_| true).unwrap();
    assert_eq!((result.as_str(), block.fluid.amount), ("tconstruct:rose_gold_block", 810));
    assert!(catalog.casting(true, Some("tconstruct:seared_cobble"), "tconstruct:molten_copper", &|_| true).is_none());
    assert!(catalog.casting(true, Some("tconstruct:scorched_stone"), "tconstruct:molten_obsidian", &|_| true).is_none());
}
