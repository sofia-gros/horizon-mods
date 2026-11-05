#[path = "../../mixins/tconstruct/source-tags.rs"] mod tconstruct_source_tags;
#[path = "../../mixins/tconstruct/source-melting-recipes.rs"] mod tconstruct_source_melting_recipes;
#[path = "../../mixins/tconstruct/source-melting-lookup.rs"] mod tconstruct_source_melting_lookup;

mod tconstruct_source_fluid_container {
    // The acquisition test never uses buckets. Model the bucket suppliers used
    // by the imported catalog tests, including their source/flowing aliases.
    // This fixture tests catalog lookup, not the production container adapter.
    pub(super) fn filled_bucket(fluid: &str) -> Option<String> {
        match fluid {
            "minecraft:lava" | "minecraft:flowing_lava" => Some("minecraft:lava_bucket".into()),
            "minecraft:water" | "minecraft:flowing_water" => Some("minecraft:water_bucket".into()),
            "tconstruct:molten_copper" | "tconstruct:flowing_molten_copper" => Some("tconstruct:molten_copper_bucket".into()),
            "tconstruct:powdered_snow" => Some("minecraft:powder_snow_bucket".into()),
            _ => None,
        }
    }
}

#[test]
fn pinned_soul_sand_and_soil_melt_and_cast_into_glass() {
    use std::collections::{BTreeMap, HashMap, HashSet};
    use std::io::Read;
    let root = std::env::var("SOUL_REPO_ROOT").unwrap_or_else(|_| ".".into());
    let archive = std::path::Path::new(&root).join("tconstruct/content/reference/tconstruct-recipes.zip");
    let mut zip = zip::ZipArchive::new(std::fs::File::open(archive).unwrap()).unwrap();
    let mut recipes = BTreeMap::new();
    for index in 0..zip.len() {
        let mut file = zip.by_index(index).unwrap();
        let name = file.name().unwrap().into_owned();
        if !name.starts_with("data/tconstruct/recipes/") || !name.ends_with(".json") { continue; }
        let mut bytes = Vec::new(); file.read_to_end(&mut bytes).unwrap(); recipes.insert(name, bytes);
    }
    let items: HashSet<String> = ["minecraft:soul_sand", "minecraft:soul_soil", "minecraft:sand", "tconstruct:soul_glass", "tconstruct:soul_glass_pane"].into_iter().map(str::to_owned).collect();
    let fluids: HashSet<String> = ["tconstruct:liquid_soul"].into_iter().map(str::to_owned).collect();
    let item_tags = HashMap::new();
    let fluid_tags = [("tconstruct:liquid_soul".to_owned(), fluids.clone())].into_iter().collect();
    let catalog = tconstruct_source_melting_lookup::Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &fluid_tags, known_items: &items, known_fluids: &fluids };
    for input in ["minecraft:soul_sand", "minecraft:soul_soil"] {
        let (_, recipe, fluid) = catalog.melting(input, &|_| true).unwrap();
        assert_eq!((fluid.as_str(), recipe.output.amount, recipe.temperature, recipe.time), ("tconstruct:liquid_soul", 1000, 400, 69));
    }
    assert!(catalog.melting("minecraft:sand", &|_| true).is_none());
    for (basin, result, amount, cooling) in [(true, "tconstruct:soul_glass", 1000, 154), (false, "tconstruct:soul_glass_pane", 250, 77)] {
        let (_, recipe, output) = catalog.casting(basin, None, "tconstruct:liquid_soul", &|_| true).unwrap();
        assert_eq!((output.as_str(), recipe.fluid.amount, recipe.cooling_time), (result, amount, cooling));
        assert!(catalog.casting(basin, Some("minecraft:sand"), "tconstruct:liquid_soul", &|_| true).is_none());
    }
    let empty_tags = HashMap::new();
    let without_tag = tconstruct_source_melting_lookup::Catalog { fluid_tags: &empty_tags, ..catalog };
    assert!(without_tag.casting(true, None, "tconstruct:liquid_soul", &|_| true).is_none());
}

#[test]
fn pinned_venombone_cast_requires_registered_output_and_resolved_tags() {
    use std::collections::{BTreeMap, HashMap, HashSet};
    use std::io::Read;
    let root = std::env::var("SOUL_REPO_ROOT").unwrap_or_else(|_| ".".into());
    let archive = std::path::Path::new(&root).join("tconstruct/content/reference/tconstruct-recipes.zip");
    let mut zip = zip::ZipArchive::new(std::fs::File::open(archive).unwrap()).unwrap();
    let mut bytes = Vec::new();
    zip.by_name("data/tconstruct/recipes/smeltery/casting/slime/venom/bone.json").unwrap().read_to_end(&mut bytes).unwrap();
    let recipes = BTreeMap::from([("tconstruct:smeltery/casting/slime/venom/bone".into(), bytes)]);
    let items = HashSet::from(["minecraft:bone".into(), "tconstruct:venombone".into()]);
    let fluids = HashSet::from(["tconstruct:venom".into(), "minecraft:water".into()]);
    let item_tags = HashMap::from([("forge:bones".into(), HashSet::from(["minecraft:bone".into()]))]);
    let fluid_tags = HashMap::from([("tconstruct:venom".into(), HashSet::from(["tconstruct:venom".into()]))]);
    let catalog = tconstruct_source_melting_lookup::Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &fluid_tags, known_items: &items, known_fluids: &fluids };
    let (_, recipe, output) = catalog.casting(false, Some("minecraft:bone"), "tconstruct:venom", &|_| true).unwrap();
    assert_eq!((output.as_str(), recipe.fluid.amount, recipe.cooling_time, recipe.consumes_cast), ("tconstruct:venombone", 250, 48, true));
    assert!(catalog.casting(true, Some("minecraft:bone"), "tconstruct:venom", &|_| true).is_none());
    assert!(catalog.casting(false, None, "tconstruct:venom", &|_| true).is_none());
    assert!(catalog.casting(false, Some("minecraft:bone"), "minecraft:water", &|_| true).is_none());
    let missing_output = HashSet::from(["minecraft:bone".into()]);
    let absent = tconstruct_source_melting_lookup::Catalog { known_items: &missing_output, ..catalog };
    assert!(absent.casting(false, Some("minecraft:bone"), "tconstruct:venom", &|_| true).is_none());
    let no_tags = HashMap::new();
    let absent = tconstruct_source_melting_lookup::Catalog { item_tags: &no_tags, ..catalog };
    assert!(absent.casting(false, Some("minecraft:bone"), "tconstruct:venom", &|_| true).is_none());
    let absent = tconstruct_source_melting_lookup::Catalog { fluid_tags: &no_tags, ..catalog };
    assert!(absent.casting(false, Some("minecraft:bone"), "tconstruct:venom", &|_| true).is_none());
}
