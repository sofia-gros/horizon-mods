//! ShapedMaterialsRecipe (`tconstruct:crafting_shaped_materials`): the shaped
//! crafting recipes whose material ingredients resolve against the material
//! registry at craft time — the travellers armor, the material anvils and
//! forges, and the fake ingot's storage block (SlimeKnights, MIT; see
//! LICENSE).
//!
//! The importer converts these recipes to the game's shaped format with the
//! material ingredients and the `parts`/`extra_materials` fields verbatim
//! (the game's shaped parser ignores both fields). This consumer supplies
//! the semantics the plain format cannot express, through PlutoLib's recipe
//! API: the two ingredient kinds (`tconstruct:material`,
//! `tconstruct:material_value`) answer the game's load-time item list and
//! craft-time stack test, and an assembly provider re-checks the grid the
//! way `ShapedMaterialsRecipe.matches` does — the parts must all be found,
//! and repeated parts must agree on one material — then builds the result's
//! tool data the way `assemble` does.
//!
//! Documented reductions: the assembled component patch carries
//! `minecraft:custom_data` only, so a freshly crafted tool's model sprite
//! (custom model data) is chosen by the port's usual stack sync paths, not
//! at the craft itself; and on a pure client whose material data has not
//! synced, the material-value kind's load-time item list is empty, so the
//! recipe does not load there (the server's own evaluation is unaffected).

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock, RwLock};

use pluto_lib::recipe_api_v1::{
    AssemblyDecision, AssemblyRegistration, AssemblyRequest, IngredientKind, RecipeForm,
    StackView,
};
use pluto_lib::{Identifier, Nbt, NbtCompound, ResourceManagerHelper, ResourceType, ServerLifecycleEvents};
use serde_json::Value;

use crate::casting_recipes::{MaterialCasting, MaterialPredicate};
use crate::material_recipes::{Ingredient as MaterialIngredient, Recipes as MaterialRecipes};
use crate::materials::Definitions;
use crate::tool_materials::{MaterialList, MaterialVariant};

/// The game's recipe folder: the converted recipes ship beside every other
/// plain recipe, told apart by their `parts` field.
const FOLDER: &str = "recipe";

/* Material predicates (the recipe subset of Mantle's material predicates) */

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Predicate {
    Any,
    And(Vec<Predicate>),
    Or(Vec<Predicate>),
    /// `mantle:tag`: the material is in the named material tag.
    Tag(Identifier),
    /// `tconstruct:stat_type`: the material has stats of this type.
    StatType(Identifier),
    Castable,
    Composite,
    /// `tconstruct:has_part`: the named part item can use the material.
    HasPart(Identifier),
    Variant(Vec<MaterialVariant>),
    Id(Vec<Identifier>),
}

pub(crate) fn decode_predicate(value: Option<&Value>) -> Result<Predicate, String> {
    let variants = |value: Option<&Value>| -> Result<Vec<MaterialVariant>, String> {
        match value {
            Some(Value::String(text)) => MaterialVariant::parse(text).map(|v| vec![v]).ok_or_else(|| format!("invalid material variant {text}")),
            Some(Value::Array(values)) => values.iter().map(|v| v.as_str().and_then(MaterialVariant::parse).ok_or_else(|| "material variant must be a string".to_owned())).collect(),
            _ => Err("material predicate needs a variant".into()),
        }
    };
    let ids = |value: Option<&Value>| -> Result<Vec<Identifier>, String> {
        match value {
            Some(Value::String(text)) => Identifier::parse(text).map(|id| vec![id]).map_err(|_| format!("invalid material id {text}")),
            Some(Value::Array(values)) => values.iter().map(|v| {
                let text = v.as_str().ok_or_else(|| "material id must be a string".to_owned())?;
                Identifier::parse(text).map_err(|_| format!("invalid material id {text}"))
            }).collect(),
            _ => Err("material predicate needs an id".into()),
        }
    };
    match value {
        None => Ok(Predicate::Any),
        // A fieldless predicate may be written as its bare type name.
        Some(Value::String(text)) => match text.as_str() {
            "tconstruct:castable" => Ok(Predicate::Castable),
            "tconstruct:composite" => Ok(Predicate::Composite),
            _ => Err(format!("unimplemented material predicate {text}")),
        },
        Some(Value::Object(fields)) => match fields.get("type").and_then(Value::as_str) {
            Some("mantle:and") => Ok(Predicate::And(decode_predicate_list(fields)?)),
            Some("mantle:or") => Ok(Predicate::Or(decode_predicate_list(fields)?)),
            Some("mantle:tag") => Ok(Predicate::Tag(ids(fields.get("tag"))?.into_iter().next().ok_or("tag predicate needs a tag")?)),
            Some("tconstruct:stat_type") => Ok(Predicate::StatType(ids(fields.get("stat_type"))?.into_iter().next().ok_or("stat_type predicate needs a stat type")?)),
            Some("tconstruct:has_part") => Ok(Predicate::HasPart(ids(fields.get("part"))?.into_iter().next().ok_or("has_part predicate needs a part")?)),
            Some("tconstruct:castable") => Ok(Predicate::Castable),
            Some("tconstruct:composite") => Ok(Predicate::Composite),
            Some("tconstruct:variant") => Ok(Predicate::Variant(variants(fields.get("variant"))?)),
            Some("tconstruct:id") => Ok(Predicate::Id(ids(fields.get("id"))?)),
            Some(other) => Err(format!("unimplemented material predicate type {other}")),
            None => Err("material predicate needs a type".into()),
        },
        Some(_) => Err("material predicate must be an object".into()),
    }
}

fn decode_predicate_list(fields: &serde_json::Map<String, Value>) -> Result<Vec<Predicate>, String> {
    fields
        .get("predicates")
        .and_then(Value::as_array)
        .ok_or("and/or predicate needs a predicates list")?
        .iter()
        .map(|value| decode_predicate(Some(value)))
        .collect()
}

/// IJsonPredicate<MaterialVariantId>.matches, against the loaded registries.
pub(crate) fn predicate_matches(predicate: &Predicate, variant: &MaterialVariant, loaded: &Data) -> bool {
    let definitions = &loaded.definitions;
    match predicate {
        Predicate::Any => true,
        Predicate::And(list) => list.iter().all(|p| predicate_matches(p, variant, loaded)),
        Predicate::Or(list) => list.iter().any(|p| predicate_matches(p, variant, loaded)),
        Predicate::Tag(tag) => variant.material_id().is_some_and(|id| {
            definitions.tags.get(tag).is_some_and(|members| members.contains(&id))
        }),
        Predicate::StatType(stat) => variant
            .material_id()
            .is_some_and(|id| definitions.get_stats(&id, stat).is_some()),
        Predicate::Castable => MaterialPredicate::Castable.matches(variant, &loaded.casting),
        Predicate::Composite => MaterialPredicate::Composite.matches(variant, &loaded.casting),
        Predicate::HasPart(part) => has_part(part, variant, loaded),
        Predicate::Variant(values) => values.iter().any(|value| value.normalized() == variant.normalized()),
        Predicate::Id(values) => variant.material_id().is_some_and(|id| values.contains(&id)),
    }
}

/// MaterialHasPartPredicate: the part item can use the material. Tool
/// parts answer through their stat type; the two fake items answer
/// through their own rule in the original (FakeIngotItem /
/// FakeStorageBlockItem.canUseMaterial): the material is in the item's
/// compatibility tag and has no material recipe of the item's own value,
/// as the fake item only stands in for a form the material lacks.
fn has_part(part: &Identifier, variant: &MaterialVariant, loaded: &Data) -> bool {
    let fake = [
        ("fake_ingot", "compatibility_metals", 1.0),
        ("fake_storage_block", "compatibility_metals/blocks", 9.0),
    ];
    if let Some((_, tag, amount)) = fake.iter().find(|(name, _, _)| {
        Identifier::new("tconstruct", name).is_ok_and(|item| item == *part)
    }) {
        let Some(material) = variant.material_id() else { return false };
        let tag = Identifier::new("tconstruct", tag).expect("valid material tag");
        let in_tag = loaded
            .definitions
            .tags
            .get(&tag)
            .is_some_and(|members| members.contains(&material));
        let has_form = loaded.material_recipes.recipes().iter().any(|recipe| {
            recipe.material.material_id() == Some(material.clone())
                && recipe.value as f64 / recipe.needed as f64 == *amount
        });
        return in_tag && !has_form;
    }
    crate::tool_parts::registry()
        .get(part)
        .is_some_and(|part| part.can_use(variant, &loaded.definitions))
}

/// The items that carry their own material (IMaterialItem): the tool
/// parts and repair kit of the parts registry, and the two fake items,
/// which the parts registry does not list.
fn is_material_item(item: &Identifier) -> bool {
    crate::tool_parts::material_items().contains_key(item)
        || ["fake_ingot", "fake_storage_block"]
            .iter()
            .any(|name| Identifier::new("tconstruct", name).is_ok_and(|fake| fake == *item))
}

/* The loaded registries the handlers read, in one snapshot */

pub(crate) struct Data {
    pub definitions: Definitions,
    pub material_recipes: MaterialRecipes,
    pub casting: MaterialCasting,
    pub items: Option<Arc<pluto_lib::RegistryLookup>>,
}

/// The server's loaded data; None while any registry has not loaded, so a
/// handler never answers from partial state.
fn load_data() -> Option<Data> {
    let definitions = crate::common::register().read().ok()?.clone();
    if !definitions.server_loaded {
        return None;
    }
    let material_recipes = crate::common::material_recipes()?.read().ok()?.clone();
    if !material_recipes.loaded {
        return None;
    }
    let casting = crate::common::smeltery_data()?.read().ok()?.recipes.material_casting.clone();
    Some(Data { definitions, material_recipes, casting, items: crate::common::item_snapshot() })
}

impl Data {
    pub(crate) fn in_tag(&self, item: &Identifier, tag: &Identifier) -> bool {
        self.items
            .as_ref()
            .and_then(|items| items.get_tag(tag))
            .is_some_and(|members| members.contains(item))
    }
    /// MaterialRecipeCache.getMaterial: a material item's own material
    /// (unknown when the item carries none), else the material of the
    /// item's material recipe (unknown when it has none).
    pub(crate) fn stack_material(&self, stack: &StackView<'_>) -> MaterialVariant {
        let is_material_item = stack.custom_data.is_some_and(|data| {
            data.as_compound().is_some_and(|data| data.contains_key("Material"))
        }) || Identifier::parse(stack.id).is_ok_and(|item| is_material_item(&item));
        if is_material_item {
            return stack
                .custom_data
                .and_then(|data| data.as_compound())
                .map(crate::tool_materials::part_material)
                .unwrap_or_else(MaterialVariant::unknown);
        }
        Identifier::parse(stack.id)
            .ok()
            .and_then(|item| {
                self.material_recipes
                    .find(&item, &|item, tag| self.in_tag(item, tag))
                    .map(|recipe| recipe.material.clone())
            })
            .unwrap_or_else(MaterialVariant::unknown)
    }
}

/* The two ingredient kinds */

/// MaterialIngredient: the named item, carrying a material the predicate
/// accepts (any material when the predicate names none).
fn material_test(data: &Value, stack: &StackView<'_>) -> bool {
    let Some(loaded) = load_data() else { return false };
    material_test_with(data, stack, &loaded)
}

fn material_test_with(data: &Value, stack: &StackView<'_>, loaded: &Data) -> bool {
    let Some(item) = data.get("item").and_then(Value::as_str) else { return false };
    if stack.id != item {
        return false;
    }
    let Ok(predicate) = decode_predicate(data.get("material")) else { return false };
    if predicate == Predicate::Any {
        return true;
    }
    let variant = stack
        .custom_data
        .and_then(|data| data.as_compound())
        .map(|data| crate::tool_materials::part_material(data))
        .unwrap_or_else(MaterialVariant::unknown);
    predicate_matches(&predicate, &variant, loaded)
}

/// MaterialValueIngredient.test(recipe): the recipe's value ratio inside
/// [min, max] and its material satisfying the predicate.
pub(crate) fn value_recipe_matches(data: &Value, recipe: &crate::material_recipes::Recipe, loaded: &Data) -> bool {
    let (min, max) = value_range(data);
    let ratio = recipe.value as f64 / recipe.needed as f64;
    if ratio < min || ratio > max {
        return false;
    }
    let Ok(predicate) = decode_predicate(data.get("material")) else { return false };
    predicate_matches(&predicate, &recipe.material, &loaded)
}

fn value_range(data: &Value) -> (f64, f64) {
    match data.get("value").and_then(Value::as_f64) {
        Some(value) => (value, value),
        None => (
            data.get("min_value").and_then(Value::as_f64).unwrap_or(0.0),
            data.get("max_value").and_then(Value::as_f64).unwrap_or(f64::MAX),
        ),
    }
}

fn material_value_test(data: &Value, stack: &StackView<'_>) -> bool {
    let Some(loaded) = load_data() else { return false };
    material_value_test_with(data, stack, &loaded)
}

fn material_value_test_with(data: &Value, stack: &StackView<'_>, loaded: &Data) -> bool {
    let Ok(item) = Identifier::parse(stack.id) else { return false };
    loaded
        .material_recipes
        .find(&item, &|item, tag| loaded.in_tag(item, tag))
        .is_some_and(|recipe| value_recipe_matches(data, recipe, loaded))
}

/// The items an ingredient names, for the load-time list: an item id, a
/// tag's members in the item snapshot, or a composite's expansion.
fn ingredient_items(ingredient: &MaterialIngredient, loaded: &Data, out: &mut Vec<String>) {
    match ingredient {
        MaterialIngredient::Item(item) => out.push(item.to_string()),
        MaterialIngredient::Tag(tag) => {
            if let Some(members) = loaded.items.as_ref().and_then(|items| items.get_tag(tag)) {
                out.extend(members.iter().map(ToString::to_string));
            }
        }
        MaterialIngredient::Any(members) => {
            for member in members {
                ingredient_items(member, loaded, out);
            }
        }
        MaterialIngredient::Difference { base, subtracted } => {
            let mut taken = Vec::new();
            ingredient_items(subtracted, loaded, &mut taken);
            let mut base_items = Vec::new();
            ingredient_items(base, loaded, &mut base_items);
            out.extend(base_items.into_iter().filter(|item| !taken.contains(item)));
        }
        MaterialIngredient::Intersection(children) => {
            if let Some((first, rest)) = children.split_first() {
                let mut items = Vec::new();
                ingredient_items(first, loaded, &mut items);
                out.extend(items.into_iter().filter(|item| {
                    Identifier::parse(item).is_ok_and(|id| {
                        rest.iter().all(|child| child.matches(&id, &|i, t| loaded.in_tag(i, t)))
                    })
                }));
            }
        }
        MaterialIngredient::NoContainer(inner) => ingredient_items(inner, loaded, out),
        MaterialIngredient::Instrument { item, .. } => out.push(item.to_string()),
        // A fluid container's items cannot be enumerated from its fluids.
        MaterialIngredient::FluidContainer(_) => {}
    }
}

/// MaterialValueIngredient.getItems: every material recipe passing the
/// filter, flattened over its ingredient's items. The host asks for this
/// list while it parses the recipe table and drops the recipe when the
/// answer is empty, so the answer is only as good as the data loaded at
/// that moment: before the reload listeners apply it is empty, after
/// them it names the real items.
fn material_value_items(data: &Value) -> Vec<String> {
    let Some(loaded) = load_data() else { return Vec::new() };
    material_value_items_with(data, &loaded)
}

/// The same list against an already-loaded snapshot, for the tests that
/// pin the parse-time behaviour above.
pub(crate) fn material_value_items_with(data: &Value, loaded: &Data) -> Vec<String> {
    let mut out = Vec::new();
    for recipe in loaded.material_recipes.recipes() {
        if value_recipe_matches(data, recipe, loaded) {
            ingredient_items(&recipe.ingredient, loaded, &mut out);
        }
    }
    out.sort();
    out.dedup();
    out
}

/* The converted recipes this consumer owns */

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum KeyIngredient {
    /// A plain item id, or `#tag` for a tag ingredient.
    Plain(String),
    Any(Vec<KeyIngredient>),
    /// A `tconstruct:material` or `tconstruct:material_value` object, verbatim.
    Material(Value),
}

#[derive(Clone, Debug)]
pub(crate) struct ShapedMaterials {
    pub pattern: Vec<String>,
    pub key: BTreeMap<char, KeyIngredient>,
    /// The pattern symbols naming material parts, in `parts` order.
    pub parts: Vec<char>,
    pub extra_materials: Vec<MaterialVariant>,
    pub result: Identifier,
}

fn decode_key_ingredient(value: &Value) -> Result<KeyIngredient, String> {
    match value {
        Value::String(text) => Ok(KeyIngredient::Plain(text.clone())),
        Value::Array(members) => Ok(KeyIngredient::Any(
            members.iter().map(decode_key_ingredient).collect::<Result<_, _>>()?,
        )),
        Value::Object(fields) => match fields.get("type").and_then(Value::as_str) {
            Some("tconstruct:material") | Some("tconstruct:material_value") => {
                Ok(KeyIngredient::Material(value.clone()))
            }
            _ => Err("shaped materials key holds an unconverted ingredient".into()),
        },
        _ => Err("shaped materials key ingredient must be a string, list or object".into()),
    }
}

fn decode_extra_materials(fields: &serde_json::Map<String, Value>) -> Result<Vec<MaterialVariant>, String> {
    match fields.get("extra_materials") {
        None => Ok(Vec::new()),
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .and_then(MaterialVariant::parse)
                    .ok_or_else(|| "extra material must be a material variant string".to_owned())
            })
            .collect(),
        Some(_) => Err("extra_materials must be a list".into()),
    }
}

fn decode_result(fields: &serde_json::Map<String, Value>) -> Result<Identifier, String> {
    let result = fields
        .get("result")
        .and_then(|result| result.get("id"))
        .and_then(Value::as_str)
        .ok_or("recipe needs a result id")?;
    Identifier::parse(result).map_err(|_| format!("invalid result id {result}"))
}

pub(crate) fn decode_recipe(value: &Value) -> Result<ShapedMaterials, String> {
    let fields = value.as_object().ok_or("recipe must be an object")?;
    let parts = fields.get("parts").and_then(Value::as_str).ok_or("recipe needs parts")?;
    let pattern: Vec<String> = fields
        .get("pattern")
        .and_then(Value::as_array)
        .ok_or("recipe needs a pattern")?
        .iter()
        .map(|row| row.as_str().map(str::to_owned).ok_or("pattern rows must be strings"))
        .collect::<Result<_, _>>()?;
    let key = fields
        .get("key")
        .and_then(Value::as_object)
        .ok_or("recipe needs a key")?
        .iter()
        .map(|(symbol, ingredient)| {
            let symbol = symbol.chars().next().ok_or("key symbol must not be empty")?;
            Ok((symbol, decode_key_ingredient(ingredient)?))
        })
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    let extra_materials = decode_extra_materials(fields)?;
    let result = decode_result(fields)?;
    Ok(ShapedMaterials { pattern, key, parts: parts.chars().collect(), extra_materials, result })
}

/// ShapelessMaterialsRecipe: the converted form carries its ingredients verbatim and a
/// numeric `parts`: how many of them, from the first, bear materials.
#[derive(Clone, Debug)]
pub(crate) struct ShapelessMaterials {
    pub ingredients: Vec<Value>,
    pub part_count: usize,
    pub extra_materials: Vec<MaterialVariant>,
    pub result: Identifier,
}

pub(crate) fn decode_shapeless_recipe(value: &Value) -> Result<ShapelessMaterials, String> {
    let fields = value.as_object().ok_or("recipe must be an object")?;
    let ingredients: Vec<Value> = fields
        .get("ingredients")
        .and_then(Value::as_array)
        .ok_or("recipe needs ingredients")?
        .clone();
    if ingredients.is_empty() {
        return Err("recipe needs at least one ingredient".into());
    }
    // The serializer refuses a part count outside 1..=ingredients.
    let part_count = fields.get("parts").and_then(Value::as_u64).ok_or("recipe needs a numeric parts")? as usize;
    if part_count < 1 || part_count > ingredients.len() {
        return Err("parts must be between 1 and the number of ingredients".into());
    }
    Ok(ShapelessMaterials {
        ingredients,
        part_count,
        extra_materials: decode_extra_materials(fields)?,
        result: decode_result(fields)?,
    })
}

/// This consumer's recipes by result item, from the current reload. One
/// result can have several recipes (the material anvil and its forge
/// variant share a result item in the original data, told apart by their
/// patterns).
fn registry() -> &'static RwLock<BTreeMap<Identifier, Vec<ShapedMaterials>>> {
    static REGISTRY: OnceLock<RwLock<BTreeMap<Identifier, Vec<ShapedMaterials>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| RwLock::new(BTreeMap::new()))
}

pub(crate) fn load_recipes(files: &[(Identifier, Value)]) -> BTreeMap<Identifier, Vec<ShapedMaterials>> {
    let mut recipes: BTreeMap<Identifier, Vec<ShapedMaterials>> = BTreeMap::new();
    for (_, value) in files {
        // Only the shaped-materials conversions carry a string `parts` field.
        if !matches!(value.get("parts"), Some(Value::String(_))) {
            continue;
        }
        match decode_recipe(value) {
            Ok(recipe) => {
                recipes.entry(recipe.result.clone()).or_default().push(recipe);
            }
            Err(error) => eprintln!("Tinkers: shaped materials recipe: {error}"),
        }
    }
    recipes
}

fn shapeless_registry() -> &'static RwLock<BTreeMap<Identifier, Vec<ShapelessMaterials>>> {
    static REGISTRY: OnceLock<RwLock<BTreeMap<Identifier, Vec<ShapelessMaterials>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| RwLock::new(BTreeMap::new()))
}

pub(crate) fn load_shapeless_recipes(files: &[(Identifier, Value)]) -> BTreeMap<Identifier, Vec<ShapelessMaterials>> {
    let mut recipes: BTreeMap<Identifier, Vec<ShapelessMaterials>> = BTreeMap::new();
    for (_, value) in files {
        // Only the shapeless-materials conversions are shapeless recipes with a numeric `parts`.
        if value.get("type").and_then(Value::as_str) != Some("minecraft:crafting_shapeless")
            || !matches!(value.get("parts"), Some(Value::Number(_)))
        {
            continue;
        }
        match decode_shapeless_recipe(value) {
            Ok(recipe) => {
                recipes.entry(recipe.result.clone()).or_default().push(recipe);
            }
            Err(error) => eprintln!("Tinkers: shapeless materials recipe: {error}"),
        }
    }
    recipes
}

/* Matching and assembly */

pub(crate) fn key_test(ingredient: &KeyIngredient, stack: &StackView<'_>, loaded: &Data) -> bool {
    match ingredient {
        KeyIngredient::Plain(text) => match text.strip_prefix('#') {
            Some(tag) => Identifier::parse(tag)
                .is_ok_and(|tag| Identifier::parse(stack.id).is_ok_and(|item| loaded.in_tag(&item, &tag))),
            None => stack.id == text,
        },
        KeyIngredient::Any(members) => members.iter().any(|member| key_test(member, stack, loaded)),
        KeyIngredient::Material(value) => match value.get("type").and_then(Value::as_str) {
            Some("tconstruct:material") => material_test_with(value, stack, loaded),
            Some("tconstruct:material_value") => material_value_test_with(value, stack, loaded),
            _ => false,
        },
    }
}

/// ShapedMaterialsRecipe.findMaterials: walk the grid in slot order; a
/// stack claims the first part it satisfies that is not yet found, and a
/// stack whose material agrees with an already-found part may still claim
/// a later part; a part's repeats must agree on the material (a differing
/// variant is discarded, a differing material fails the match). Every
/// part must be found.
pub(crate) fn find_materials(
    recipe: &ShapedMaterials,
    cells: &[Option<StackView<'_>>],
    loaded: &Data,
) -> Option<Vec<MaterialVariant>> {
    // checkRepeats in the original: the parts name distinct ingredients
    // (one symbol twice shares its ingredient, so repeats are not checked).
    let check_repeats =
        recipe.parts.iter().collect::<std::collections::BTreeSet<_>>().len() == recipe.parts.len();
    let mut found: Vec<Option<MaterialVariant>> = vec![None; recipe.parts.len()];
    for stack in cells.iter().flatten() {
        for (index, symbol) in recipe.parts.iter().enumerate() {
            if found[index].is_some() && !check_repeats {
                continue;
            }
            let Some(ingredient) = recipe.key.get(symbol) else { return None };
            if !key_test(ingredient, stack, loaded) {
                continue;
            }
            let material = loaded.stack_material(stack);
            match &found[index] {
                None => {
                    found[index] = Some(material);
                    break;
                }
                Some(existing) => {
                    if *existing == material {
                        // Agreement: this stack may still claim a later part.
                        continue;
                    }
                    if existing.material_id() == material.material_id() {
                        // Same material, different variant: keep the bare material.
                        let bare = existing
                            .material_id()
                            .and_then(|id| MaterialVariant::parse(&id.to_string()))
                            .unwrap_or_else(MaterialVariant::unknown);
                        found[index] = Some(bare);
                        break;
                    }
                    return None;
                }
            }
        }
    }
    found.into_iter().collect()
}

/// The grid against the pattern, in the pattern's orientation or mirrored
/// (the game's shaped match accepts both).
pub(crate) fn grid_matches(recipe: &ShapedMaterials, request: &AssemblyRequest<'_>, loaded: &Data) -> bool {
    let (width, height) = (request.grid.width, request.grid.height);
    if recipe.pattern.len() != height
        || recipe.pattern.iter().any(|row| row.chars().count() != width)
        || request.grid.cells.len() != width * height
    {
        return false;
    }
    [false, true].iter().any(|&mirrored| {
        recipe.pattern.iter().enumerate().all(|(y, row)| {
            row.chars().enumerate().all(|(x, symbol)| {
                let cell = request.grid.cells[y * width + if mirrored { width - 1 - x } else { x }];
                match (symbol, cell) {
                    (' ', None) => true,
                    (' ', Some(_)) => false,
                    (_, None) => false,
                    (_, Some(stack)) => recipe
                        .key
                        .get(&symbol)
                        .is_some_and(|ingredient| key_test(ingredient, &stack, loaded)),
                }
            })
        })
    })
}

/// ShapedMaterialsRecipe.assemble: one material and no extras on a
/// material item sets its material; anything else is a tool stack whose
/// materials are the parts' materials followed by the extras.
fn assemble(recipe: &ShapedMaterials, materials: Vec<MaterialVariant>) -> Option<NbtCompound> {
    assemble_result(&recipe.result, materials, &recipe.extra_materials)
}

/// The assemble tail both materials forms share (ShapelessMaterialsRecipe
/// delegates to the same logic in the original).
fn assemble_result(result_item: &Identifier, mut materials: Vec<MaterialVariant>, extra_materials: &[MaterialVariant]) -> Option<NbtCompound> {
    materials.extend(extra_materials.iter().cloned());
    if materials.len() == 1 && extra_materials.is_empty() {
        if is_material_item(result_item) {
            let mut data = NbtCompound::new();
            data.insert("Material".into(), Nbt::String(materials[0].to_string()));
            return Some(data);
        }
    }
    let list = MaterialList::from_variants(materials);
    if let Some(definition) = tool_definition(result_item) {
        let definitions = crate::common::register().read().ok()?;
        let modifiers = crate::common::server_modifiers()?.read().ok()?;
        let items = crate::common::item_snapshot();
        let env = crate::tool_stack::Environment {
            definition: Some(&definition),
            has_materials: definition.material_stat_types().is_some(),
            materials: &definitions,
            modifiers: &modifiers,
            item_tags_loaded: items.is_some(),
            item: Some(result_item),
            item_tags: items.as_deref(),
        };
        return crate::tool_build::create_tool(&env, &list, false).ok();
    }
    let mut data = NbtCompound::new();
    crate::tool_stack::set_materials_raw(&mut data, &list);
    Some(data)
}

/// One shapeless ingredient against a stack: the two material kinds by their
/// own tests, anything else as a plain ingredient.
fn shapeless_test(ingredient: &Value, stack: &StackView<'_>, loaded: &Data) -> bool {
    if let Some(text) = ingredient.as_str() {
        return match text.strip_prefix('#') {
            Some(tag) => Identifier::parse(tag)
                .is_ok_and(|tag| Identifier::parse(stack.id).is_ok_and(|item| loaded.in_tag(&item, &tag))),
            None => stack.id == text,
        };
    }
    match ingredient.get("type").and_then(Value::as_str) {
        Some("tconstruct:material") => material_test_with(ingredient, stack, loaded),
        Some("tconstruct:material_value") => material_value_test_with(ingredient, stack, loaded),
        _ => match MaterialIngredient::decode(ingredient) {
            Ok(parsed) => Identifier::parse(stack.id)
                .is_ok_and(|item| parsed.matches(&item, &|i, t| loaded.in_tag(i, t))),
            Err(_) => false,
        },
    }
}

/// The grid against a shapeless recipe: every ingredient takes a distinct
/// stack and nothing else is on the grid (the game's own shapeless match).
fn shapeless_grid_matches(recipe: &ShapelessMaterials, cells: &[Option<StackView<'_>>], loaded: &Data) -> bool {
    let stacks: Vec<&StackView<'_>> = cells.iter().flatten().collect();
    if stacks.len() != recipe.ingredients.len() {
        return false;
    }
    let mut used = vec![false; stacks.len()];
    for ingredient in &recipe.ingredients {
        let Some((slot, _)) = stacks.iter().enumerate().find(|(index, stack)| !used[*index] && shapeless_test(ingredient, stack, loaded)) else {
            return false;
        };
        used[slot] = true;
    }
    true
}

/// findMaterials for the shapeless form (checkRepeats is false there): walk
/// the grid in slot order; a stack claims the first part it satisfies that
/// is not yet found. Every part must be found.
pub(crate) fn find_materials_shapeless(
    recipe: &ShapelessMaterials,
    cells: &[Option<StackView<'_>>],
    loaded: &Data,
) -> Option<Vec<MaterialVariant>> {
    let mut found: Vec<Option<MaterialVariant>> = vec![None; recipe.part_count];
    for stack in cells.iter().flatten() {
        for index in 0..recipe.part_count {
            if found[index].is_some() {
                continue;
            }
            if !shapeless_test(&recipe.ingredients[index], stack, loaded) {
                continue;
            }
            found[index] = Some(loaded.stack_material(stack));
            break;
        }
    }
    found.into_iter().collect()
}

fn shapeless_interest(request: &pluto_lib::recipe_api_v1::RecipeFacts<'_>) -> bool {
    matches!(request.form, RecipeForm::Shapeless)
        && request
            .result_item
            .is_some_and(|item| Identifier::parse(item).is_ok_and(|id| shapeless_registry().read().is_ok_and(|recipes| recipes.contains_key(&id))))
}

fn shapeless_callback(request: &AssemblyRequest<'_>) -> AssemblyDecision {
    let RecipeForm::Shapeless = request.recipe.form else { return AssemblyDecision::Pass };
    let Some(result_item) = request.recipe.result_item else { return AssemblyDecision::Pass };
    let Ok(result) = Identifier::parse(result_item) else { return AssemblyDecision::Pass };
    let candidates = shapeless_registry()
        .read()
        .ok()
        .and_then(|recipes| recipes.get(&result).cloned());
    let Some(candidates) = candidates else { return AssemblyDecision::Pass };
    let Some(loaded) = load_data() else { return AssemblyDecision::Pass };
    for recipe in &candidates {
        if !shapeless_grid_matches(recipe, request.grid.cells, &loaded) {
            continue;
        }
        // The grid is this recipe's grid; a part disagreement is a real
        // non-match (Java's matches returns false), not a plain result.
        let Some(materials) = find_materials_shapeless(recipe, request.grid.cells, &loaded) else {
            return AssemblyDecision::Deny;
        };
        return match assemble_result(&recipe.result, materials, &recipe.extra_materials) {
            Some(custom_data) => AssemblyDecision::Assemble { custom_data },
            None => AssemblyDecision::Deny,
        };
    }
    AssemblyDecision::Pass
}

fn tool_definition(item: &Identifier) -> Option<crate::tool_definitions::Data> {
    crate::tool_definitions::register()
        .read()
        .ok()?
        .data
        .get(item)
        .cloned()
}

fn assembly_interest(request: &pluto_lib::recipe_api_v1::RecipeFacts<'_>) -> bool {
    matches!(request.form, RecipeForm::Shaped { .. })
        && request
            .result_item
            .is_some_and(|item| Identifier::parse(item).is_ok_and(|id| registry().read().is_ok_and(|recipes| recipes.contains_key(&id))))
}

fn assembly_callback(request: &AssemblyRequest<'_>) -> AssemblyDecision {
    let RecipeForm::Shaped { .. } = request.recipe.form else { return AssemblyDecision::Pass };
    let Some(result_item) = request.recipe.result_item else { return AssemblyDecision::Pass };
    let Ok(result) = Identifier::parse(result_item) else { return AssemblyDecision::Pass };
    let candidates = registry()
        .read()
        .ok()
        .and_then(|recipes| recipes.get(&result).cloned());
    let Some(candidates) = candidates else { return AssemblyDecision::Pass };
    let Some(loaded) = load_data() else { return AssemblyDecision::Pass };
    for recipe in &candidates {
        if !grid_matches(recipe, request, &loaded) {
            continue;
        }
        // The grid is this recipe's grid; a part disagreement is a real
        // non-match (Java's matches returns false), not a plain result.
        let Some(materials) = find_materials(recipe, request.grid.cells, &loaded) else {
            return AssemblyDecision::Deny;
        };
        return match assemble(recipe, materials) {
            Some(custom_data) => AssemblyDecision::Assemble { custom_data },
            None => AssemblyDecision::Deny,
        };
    }
    AssemblyDecision::Pass
}

/// Installs the ingredient kinds, the assembly provider and the recipe
/// reload listener.
pub(crate) fn register() {
    let _ = pluto_lib::recipe_api_v1::register_ingredient_kind(IngredientKind {
        kind: "tconstruct:material",
        items: |data| {
            data.get("item")
                .and_then(Value::as_str)
                .map(|item| vec![item.to_owned()])
                .unwrap_or_default()
        },
        test: material_test,
    });
    let _ = pluto_lib::recipe_api_v1::register_ingredient_kind(IngredientKind {
        kind: "tconstruct:material_value",
        items: material_value_items,
        test: material_value_test,
    });
    let _ = pluto_lib::recipe_api_v1::register_assembly(AssemblyRegistration {
        id: "tconstruct:shaped_materials",
        interest: assembly_interest,
        callback: assembly_callback,
    });
    let _ = pluto_lib::recipe_api_v1::register_assembly(AssemblyRegistration {
        id: "tconstruct:shapeless_materials",
        interest: shapeless_interest,
        callback: shapeless_callback,
    });
    let output = registry();
    ResourceManagerHelper::get(ResourceType::ServerData)
        .register_json_reload_listener(
            Identifier::new("tconstruct", "shaped_materials").unwrap(),
            FOLDER,
            move |files| {
                let mut state = output.write().map_err(|_| "shaped materials recipes poisoned")?;
                *state = load_recipes(files);
                let mut shapeless = shapeless_registry().write().map_err(|_| "shapeless materials recipes poisoned")?;
                *shapeless = load_shapeless_recipes(files);
                Ok(())
            },
        )
        .expect("Tinkers shaped materials listener registration failed");
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| {
        *registry().write().expect("shaped materials recipes poisoned") = BTreeMap::new();
        *shapeless_registry().write().expect("shapeless materials recipes poisoned") = BTreeMap::new();
    });
}
