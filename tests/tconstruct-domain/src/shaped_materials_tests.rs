//! The shaped-materials consumer against fixtures and the shipped data:
//! the predicate forms of the ten recipes, findMaterials' agreement rules,
//! and the conversion itself (every shipped shaped-materials recipe
//! decodes and is registered by its result item).
use std::collections::BTreeMap;
use std::path::PathBuf;

use pluto_lib::recipe_api_v1::{AssemblyRequest, GridFacts, RecipeFacts, RecipeForm, StackView};
use pluto_lib::{Identifier, JsonResourceStack, Nbt, NbtCompound};
use serde_json::{Value, json};

use crate::shaped_materials::{
    Data, KeyIngredient, Predicate, decode_predicate, decode_recipe, decode_shapeless_recipe,
    find_materials, find_materials_shapeless, grid_matches, load_recipes, load_shapeless_recipes,
    predicate_matches, value_recipe_matches,
};
use crate::tool_materials::MaterialVariant;

fn id(text: &str) -> Identifier {
    Identifier::parse(text).unwrap()
}

fn variant(text: &str) -> MaterialVariant {
    MaterialVariant::parse(text).unwrap()
}

/// A small registry: iron (head stats, in the alloys tag, castable through
/// a casting fluid) and wood (no stats).
fn data() -> Data {
    let mut definitions = crate::materials::Definitions::default();
    definitions.server_loaded = true;
    let mut problems = Vec::new();
    definitions.stats = crate::material_stats::load(
        &[JsonResourceStack {
            id: id("tconstruct:iron"),
            layers: vec![Ok(json!({"stats": {"tconstruct:head": {"durability": 204, "mining_speed": 6.0, "mining_level": 2, "melee_attack": 2.0}}}))],
        }],
        &mut problems,
    );
    assert!(problems.is_empty(), "{problems:?}");
    definitions
        .tags
        .insert(id("tconstruct:compatibility_metals/alloys"), vec![id("tconstruct:iron")]);
    definitions
        .tags
        .insert(id("tconstruct:compatibility_metals/blocks"), vec![id("tconstruct:iron"), id("tconstruct:bronze")]);
    let files = vec![
        (
            id("tconstruct:tools/materials/iron_ingot"),
            json!({"type": "tconstruct:material", "ingredient": {"item": "minecraft:iron_ingot"}, "material": "tconstruct:iron", "needed": 1, "value": 1}),
        ),
        (
            id("tconstruct:tools/materials/iron_block"),
            json!({"type": "tconstruct:material", "ingredient": {"item": "minecraft:iron_block"}, "material": "tconstruct:iron", "needed": 1, "value": 9}),
        ),
        (
            id("tconstruct:tools/materials/bronze_ingot"),
            json!({"type": "tconstruct:material", "ingredient": {"item": "tconstruct:bronze_ingot"}, "material": "tconstruct:bronze", "needed": 1, "value": 1}),
        ),
        (
            id("tconstruct:tools/materials/stick"),
            json!({"type": "tconstruct:material", "ingredient": {"item": "minecraft:stick"}, "material": "tconstruct:wood", "needed": 1, "value": 1}),
        ),
    ];
    let material_recipes = crate::material_recipes::Recipes::load(&files, &crate::original_data::conditions());
    assert!(material_recipes.problems.is_empty(), "{:?}", material_recipes.problems);
    let mut casting = crate::casting_recipes::MaterialCasting::default();
    casting.casting.push(crate::casting_recipes::MaterialFluid {
        id: id("tconstruct:iron_casting"),
        fluid: crate::fluid_ingredient::FluidIngredient { options: Vec::new() },
        temperature: 800,
        input: None,
        output: variant("tconstruct:iron"),
        hide_in_book: false,
    });
    Data { definitions, material_recipes, casting, items: None }
}

#[test]
fn the_predicate_forms_of_the_ten_recipes_decode_and_match() {
    let loaded = data();
    let iron = variant("tconstruct:iron");
    let wood = variant("tconstruct:wood");
    let matches = |value: Value, variant: &MaterialVariant| {
        let predicate = decode_predicate(Some(&value)).unwrap();
        predicate_matches(&predicate, variant, &loaded)
    };
    // The travellers chestplate's c predicate: (castable or composite) and plating stats.
    let chestplate = json!({"type": "mantle:and", "predicates": [
        {"type": "mantle:or", "predicates": ["tconstruct:castable", "tconstruct:composite"]},
        {"type": "tconstruct:stat_type", "stat_type": "tconstruct:head"},
    ]});
    assert!(matches(chestplate.clone(), &iron));
    assert!(!matches(chestplate, &wood), "wood is neither castable nor a head material here");
    // The anvil's mantle:tag predicate.
    let alloys = json!({"type": "mantle:tag", "tag": "tconstruct:compatibility_metals/alloys"});
    assert!(matches(alloys.clone(), &iron));
    assert!(!matches(alloys, &wood));
    // The fake ingot conversion's has_part predicate: the fake storage
    // block serves materials in its compatibility tag that lack a block
    // form of their own (FakeStorageBlockItem.canUseMaterial).
    let has_part = json!({"type": "tconstruct:has_part", "part": "tconstruct:fake_storage_block"});
    assert!(matches(has_part.clone(), &variant("tconstruct:bronze")), "bronze is in the tag and has no ratio-9 recipe");
    assert!(!matches(has_part.clone(), &iron), "iron has a real block recipe (ratio 9), so the fake block cannot serve it");
    assert!(!matches(has_part, &wood), "wood is not in the compatibility tag");
    // Bare fieldless forms and an absent predicate.
    assert!(matches(json!("tconstruct:castable"), &iron));
    assert!(!matches(json!("tconstruct:composite"), &iron));
    assert_eq!(decode_predicate(None).unwrap(), Predicate::Any);
    assert!(decode_predicate(Some(&json!({"type": "mantle:unknown"}))).is_err());
}

#[test]
fn material_value_recipes_filter_by_ratio_and_predicate() {
    let loaded = data();
    let recipe = |item: &str| {
        loaded
            .material_recipes
            .recipes()
            .iter()
            .find(|recipe| recipe.ingredient.matches(&id(item), &|_, _| false))
            .unwrap()
    };
    let exact_one = json!({"type": "tconstruct:material_value", "material": {"type": "tconstruct:stat_type", "stat_type": "tconstruct:head"}, "value": 1.0});
    assert!(value_recipe_matches(&exact_one, recipe("minecraft:iron_ingot"), &loaded));
    assert!(!value_recipe_matches(&exact_one, recipe("minecraft:iron_block"), &loaded), "ratio 9 is not 1");
    assert!(!value_recipe_matches(&exact_one, recipe("minecraft:stick"), &loaded), "wood has no head stats");
    let any_value = json!({"type": "tconstruct:material_value", "value": 9.0});
    assert!(value_recipe_matches(&any_value, recipe("minecraft:iron_block"), &loaded));
}

fn stack<'a>(item: &'a str, custom_data: Option<&'a Nbt>) -> StackView<'a> {
    StackView { id: item, count: 1, damage: 0, custom_data }
}

#[test]
fn find_materials_agrees_disagrees_and_discards_variants_like_the_original() {
    let loaded = data();
    // A two-part recipe in the travellers shape: c (material_value, head
    // materials, value 1) and l (material_value, any material, value 1).
    let recipe = decode_recipe(&json!({
        "type": "minecraft:crafting_shaped",
        "pattern": ["lcl"],
        "key": {
            "c": {"type": "tconstruct:material_value", "material": {"type": "tconstruct:stat_type", "stat_type": "tconstruct:head"}, "value": 1.0},
            "l": {"type": "tconstruct:material_value", "value": 1.0},
        },
        "parts": "cl",
        "result": {"id": "tconstruct:travelers_chestplate"},
    }))
    .unwrap();
    let iron = stack("minecraft:iron_ingot", None);
    let wood = stack("minecraft:stick", None);
    // Both parts iron: found in parts order.
    let cells = [Some(iron), Some(iron), Some(iron)];
    assert_eq!(
        find_materials(&recipe, &cells, &loaded).unwrap().iter().map(ToString::to_string).collect::<Vec<_>>(),
        ["tconstruct:iron", "tconstruct:iron"],
    );
    // The leather part in wood, the core in iron: each stack claims the
    // first part it satisfies, so the wood stack takes l only if it does
    // not satisfy c — it does not (no head stats).
    let cells = [Some(wood), Some(iron), Some(wood)];
    assert_eq!(
        find_materials(&recipe, &cells, &loaded).unwrap().iter().map(ToString::to_string).collect::<Vec<_>>(),
        ["tconstruct:iron", "tconstruct:wood"],
    );
    // A part never satisfied fails the match.
    let cells = [Some(wood), Some(wood), Some(wood)];
    assert!(find_materials(&recipe, &cells, &loaded).is_none());
    // A material item stack contributes its own material, variant first:
    // the same material in two variants collapses to the bare material.
    let mut data_a = NbtCompound::new();
    data_a.insert("Material".into(), Nbt::String("tconstruct:iron#shiny".into()));
    let nbt_a = Nbt::Compound(data_a);
    let mut data_b = NbtCompound::new();
    data_b.insert("Material".into(), Nbt::String("tconstruct:iron#dull".into()));
    let nbt_b = Nbt::Compound(data_b);
    let part_recipe = decode_recipe(&json!({
        "type": "minecraft:crafting_shaped",
        "pattern": ["##"],
        "key": {"#": {"type": "tconstruct:material", "item": "tconstruct:fake_ingot"}},
        "parts": "#",
        "result": {"id": "tconstruct:fake_storage_block"},
    }))
    .unwrap();
    let cells = [
        Some(stack("tconstruct:fake_ingot", Some(&nbt_a))),
        Some(stack("tconstruct:fake_ingot", Some(&nbt_b))),
    ];
    assert_eq!(
        find_materials(&part_recipe, &cells, &loaded).unwrap().iter().map(ToString::to_string).collect::<Vec<_>>(),
        ["tconstruct:iron"],
        "same material, differing variants: the variant is discarded",
    );
    // And a grid that does not fit the pattern is not this recipe's grid.
    let facts = RecipeFacts { form: RecipeForm::Shaped { width: 2, height: 1 }, result_item: Some("tconstruct:fake_storage_block"), result_count: 1 };
    let request = AssemblyRequest {
        recipe: &facts,
        grid: GridFacts { width: 2, height: 1, left: 0, top: 0, cells: &cells },
        result: None,
    };
    assert!(grid_matches(&part_recipe, &request, &loaded));
    let wrong = [Some(stack("minecraft:stick", None)), Some(stack("tconstruct:fake_ingot", Some(&nbt_a)))];
    let request = AssemblyRequest { grid: GridFacts { cells: &wrong, ..request.grid }, ..request };
    assert!(!grid_matches(&part_recipe, &request, &loaded));
}

#[test]
fn all_ten_shipped_shaped_materials_recipes_decode() {
    let content = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tconstruct/content/data/tconstruct/recipe");
    let expected: BTreeMap<&str, (&str, &str)> = [
        ("tables/scorched_anvil_material", ("tconstruct:scorched_anvil", "m")),
        ("tables/scorched_forge_material", ("tconstruct:scorched_anvil", "m")),
        ("tables/seared_forge_material", ("tconstruct:tinkers_anvil", "m")),
        ("tables/tinkers_anvil_material", ("tconstruct:tinkers_anvil", "m")),
        ("tools/armor/travelers/boots", ("tconstruct:travelers_boots", "cl")),
        ("tools/armor/travelers/chestplate", ("tconstruct:travelers_chestplate", "cl")),
        ("tools/armor/travelers/goggles", ("tconstruct:travelers_helmet", "cl")),
        ("tools/armor/travelers/pants", ("tconstruct:travelers_leggings", "cl")),
        ("tools/armor/travelers/shield", ("tconstruct:travelers_shield", "cl")),
        ("tools/parts/fake_ingot_to_block", ("tconstruct:fake_storage_block", "#")),
    ]
    .into_iter()
    .collect();
    let mut files = Vec::new();
    for (stem, _) in &expected {
        let path = content.join(format!("{stem}.json"));
        let value: Value = serde_json::from_slice(&std::fs::read(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"))).unwrap();
        files.push((id(&format!("tconstruct:{stem}")), value));
    }
    let recipes = load_recipes(&files);
    let total: usize = recipes.values().map(Vec::len).sum();
    assert_eq!(total, expected.len(), "every converted recipe registers");
    // A material ingredient counts wherever it sits: the host-parse
    // workaround wraps it in an alternatives list behind a plain
    // representative item, so it can nest inside `Any`.
    fn keeps_material(ingredient: &KeyIngredient) -> bool {
        match ingredient {
            KeyIngredient::Material(_) => true,
            KeyIngredient::Any(members) => members.iter().any(keeps_material),
            KeyIngredient::Plain(_) => false,
        }
    }
    for (stem, (result, parts)) in &expected {
        let candidates = &recipes[&id(result)];
        let recipe = candidates
            .iter()
            .find(|recipe| recipe.parts.iter().collect::<String>() == *parts)
            .unwrap_or_else(|| panic!("{stem} registers under {result} with parts {parts}"));
        assert!(
            recipe.key.values().any(keeps_material),
            "{stem} keeps a material ingredient",
        );
    }
}

#[test]
fn the_shipped_shapeless_materials_recipe_decodes_and_finds_its_material() {
    // The one shipped shapeless-materials conversion: fake_block_to_ingots.
    let content = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tconstruct/content/data/tconstruct/recipe");
    let path = content.join("tools/parts/fake_block_to_ingots.json");
    let value: Value = serde_json::from_slice(&std::fs::read(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"))).unwrap();
    let file = (id("tconstruct:tools/parts/fake_block_to_ingots"), value);
    let registered = load_shapeless_recipes(std::slice::from_ref(&file));
    let candidates = &registered[&id("tconstruct:fake_ingot")];
    assert_eq!(candidates.len(), 1, "the conversion registers under its result item");
    let recipe = &candidates[0];
    assert_eq!(recipe.part_count, 1);
    // The shaped loader does not claim it (its parts is a count, not symbols).
    assert!(load_recipes(&[file.clone()]).is_empty());
    // The serializer's bounds: parts must be between 1 and the ingredient count.
    assert!(decode_shapeless_recipe(&json!({"type": "minecraft:crafting_shapeless", "ingredients": [{"item": "minecraft:stone"}], "parts": 0, "result": {"id": "minecraft:stone"}})).is_err());
    assert!(decode_shapeless_recipe(&json!({"type": "minecraft:crafting_shapeless", "ingredients": [{"item": "minecraft:stone"}], "parts": 2, "result": {"id": "minecraft:stone"}})).is_err());

    // A registry where mythril has a block form (value 9) but no ingot form,
    // so the fake ingot may stand in for it; iron has both.
    let mut definitions = crate::materials::Definitions::default();
    definitions.server_loaded = true;
    definitions
        .tags
        .insert(id("tconstruct:compatibility_metals"), vec![id("tconstruct:mythril"), id("tconstruct:iron")]);
    let files = vec![
        (id("tconstruct:tools/materials/mythril_block"), json!({"type": "tconstruct:material", "ingredient": {"item": "tconstruct:mythril_block"}, "material": "tconstruct:mythril", "needed": 1, "value": 9})),
        (id("tconstruct:tools/materials/iron_ingot"), json!({"type": "tconstruct:material", "ingredient": {"item": "minecraft:iron_ingot"}, "material": "tconstruct:iron", "needed": 1, "value": 1})),
    ];
    let material_recipes = crate::material_recipes::Recipes::load(&files, &crate::original_data::conditions());
    assert!(material_recipes.problems.is_empty(), "{:?}", material_recipes.problems);
    let loaded = Data { definitions, material_recipes, casting: crate::casting_recipes::MaterialCasting::default(), items: None };
    let block = |material: &str| {
        let mut data = NbtCompound::new();
        data.insert("Material".into(), Nbt::String(material.into()));
        Nbt::Compound(data)
    };
    let mythril = block("tconstruct:mythril");
    let cells = [Some(stack("tconstruct:fake_storage_block", Some(&mythril))), None, None];
    assert_eq!(
        find_materials_shapeless(recipe, &cells, &loaded).unwrap().iter().map(ToString::to_string).collect::<Vec<_>>(),
        ["tconstruct:mythril"],
        "the block's material carries into the ingots",
    );
    // Iron already has an ingot form, so the fake ingot cannot stand in for
    // it: the ingredient refuses the stack and the part is never found.
    let iron = block("tconstruct:iron");
    let cells = [Some(stack("tconstruct:fake_storage_block", Some(&iron))), None, None];
    assert!(find_materials_shapeless(recipe, &cells, &loaded).is_none());
}

#[test]
fn shipped_material_value_ingredients_resolve_once_the_data_has_loaded() {
    // The host asks each mod ingredient kind for its load-time item list
    // while it parses the recipe table, and drops the recipe when the
    // answer is empty. Before the reload listeners apply, the material
    // data has not loaded and the answer is empty — the parse-order bug
    // the data workaround routes around. With the data loaded (the state
    // the listeners leave behind, which the host now reaches before the
    // parse), every material_value ingredient in the shipped grid
    // recipes names at least one item, so the recipes survive the parse.
    let content = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tconstruct/content/data/tconstruct/recipe");
    let stems = [
        "tools/parts/pick_head",
        "tools/parts/tool_binding",
        "tools/parts/tool_handle",
        "tools/armor/travelers/boots",
        "tools/armor/travelers/chestplate",
        "tools/armor/travelers/goggles",
        "tools/armor/travelers/pants",
        "tools/armor/travelers/shield",
    ];
    fn collect(value: &Value, out: &mut Vec<Value>) {
        match value {
            Value::Object(fields) => {
                if fields.get("type").and_then(Value::as_str) == Some("tconstruct:material_value") {
                    out.push(value.clone());
                }
            }
            Value::Array(members) => members.iter().for_each(|member| collect(member, out)),
            _ => {}
        }
    }
    let mut ingredients: Vec<Value> = Vec::new();
    for stem in stems {
        let path = content.join(format!("{stem}.json"));
        let recipe: Value = serde_json::from_slice(&std::fs::read(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"))).unwrap();
        let before = ingredients.len();
        for member in recipe["key"].as_object().unwrap().values() {
            collect(member, &mut ingredients);
        }
        assert!(ingredients.len() > before, "{stem} keys a material_value ingredient");
    }
    // The registries as the reload listeners leave them: the shipped
    // material definitions, stats and traits, the shipped material
    // recipes, the shipped casting tables, and the item snapshot the
    // host hands over with them (declared items and flattened item
    // tags, built the way casting_loop builds it), which is what a
    // tag-keyed material ingredient enumerates through.
    let mut definitions = crate::materials::Definitions::load(
        &crate::original_data::files("materials/definition"),
        &crate::original_data::conditions(),
    );
    let mut problems = Vec::new();
    definitions.stats = crate::material_stats::load(&crate::original_data::stacks("materials/stats"), &mut problems);
    definitions.traits = crate::material_traits::load(&crate::original_data::stacks("materials/traits"), &mut problems);
    assert!(problems.is_empty(), "{problems:?}");
    definitions.server_loaded = true;
    let mut material_recipes = crate::material_recipes::Recipes::load(
        &crate::original_data::recursive_files("tools/materials", "tools/materials"),
        &crate::original_data::conditions(),
    );
    assert!(material_recipes.problems.is_empty(), "{:?}", material_recipes.problems);
    material_recipes.loaded = true;
    let loaded = Data {
        definitions,
        material_recipes,
        casting: crate::original_data::fluid_recipes(false).material_casting,
        items: Some(std::sync::Arc::new(crate::casting_loop::item_registry())),
    };
    for ingredient in &ingredients {
        assert!(
            !crate::shaped_materials::material_value_items_with(ingredient, &loaded).is_empty(),
            "{ingredient} names at least one item once the material data has loaded",
        );
    }
    // And before any of it loads, the same questions answer empty: the
    // state the host's recipe parse used to run against.
    let unloaded = Data {
        definitions: crate::materials::Definitions::default(),
        material_recipes: crate::material_recipes::Recipes::load(&[], &crate::original_data::conditions()),
        casting: crate::casting_recipes::MaterialCasting::default(),
        items: None,
    };
    for ingredient in &ingredients {
        assert!(
            crate::shaped_materials::material_value_items_with(ingredient, &unloaded).is_empty(),
            "{ingredient} names nothing before the material data loads",
        );
    }
}
