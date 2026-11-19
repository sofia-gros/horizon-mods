//! MaterialRecipe: the item -> material recipes (SlimeKnights, MIT; see LICENSE).
//! Loaded from the pack's recipes/tools/materials folder with the same data-condition tree as
//! materials. Ingredient types this port lacks (instrument variants, which need item components)
//! leave their recipe out with a reported problem instead of matching everything or nothing silently.
use crate::{materials::Conditions, tool_materials::MaterialVariant};
use pluto_lib::{Identifier, ResourceManagerHelper, ResourceType, ServerLifecycleEvents};
use serde_json::Value;
use std::sync::{Arc, RwLock};

pub(crate) const FOLDER: &str = "recipes/tools/materials";

/// The Forge ingredient forms the shipped recipes use.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Ingredient {
    Item(Identifier),
    Tag(Identifier),
    /// A JSON array: any of the members.
    Any(Vec<Ingredient>),
    /// forge:difference: matches base but not subtracted.
    Difference { base: Box<Ingredient>, subtracted: Box<Ingredient> },
    /// forge:intersection: matches every child.
    Intersection(Vec<Ingredient>),
    /// tconstruct:no_container (NoContainerIngredient): the nested ingredient, but only an item with no crafting remainder
    /// (an empty fluid container). The remainder is an item property the caller reports to `matches_with_remainder`.
    NoContainer(Box<Ingredient>),
}

/// The synthetic tag standing for "the tool's definition binds this hook".
pub(crate) fn hook_tag(hook: &Identifier) -> Identifier {
    Identifier::new("tconstruct", &format!("tool_hook/{}/{}", hook.namespace(), hook.path())).expect("valid hook tag")
}

/// The hook a synthetic hook tag stands for, None for an ordinary tag.
pub(crate) fn hook_of_tag(tag: &Identifier) -> Option<Identifier> {
    if tag.namespace() != "tconstruct" { return None; }
    let (namespace, path) = tag.path().strip_prefix("tool_hook/")?.split_once('/')?;
    Identifier::new(namespace, path).ok()
}

impl Ingredient {
    pub fn decode(value: &Value) -> Result<Self, String> {
        let id = |value: &Value| Identifier::parse(value.as_str().ok_or("expected an identifier string")?).map_err(|e| e.to_string());
        match value {
            Value::Array(members) if !members.is_empty() => Ok(Self::Any(members.iter().map(Self::decode).collect::<Result<_, _>>()?)),
            Value::Object(fields) => match fields.get("type").and_then(Value::as_str) {
                None => match (fields.get("item"), fields.get("tag")) {
                    (Some(item), None) => Ok(Self::Item(id(item)?)),
                    (None, Some(tag)) => Ok(Self::Tag(id(tag)?)),
                    _ => Err("an ingredient names exactly one of item or tag".into()),
                },
                Some("forge:intersection") => Ok(Self::Intersection(
                    fields.get("children").and_then(Value::as_array).filter(|children| !children.is_empty()).ok_or("children must be a nonempty array")?
                        .iter().map(Self::decode).collect::<Result<_, _>>()?)),
                // MaterialIngredient with no material named: the part item of any material.
                Some("tconstruct:material") => Ok(Self::Item(id(fields.get("item").ok_or("item missing")?)?)),
                Some("tconstruct:no_container") => Ok(Self::NoContainer(Box::new(match fields.get("match") {
                    Some(nested) => Self::decode(nested)?,
                    // Without "match" the object itself is the vanilla item or tag ingredient.
                    None => { let mut plain = fields.clone(); plain.remove("type"); Self::decode(&Value::Object(plain))? }
                }))),
                Some("forge:difference") => Ok(Self::Difference {
                    base: Box::new(Self::decode(fields.get("base").ok_or("base missing")?)?),
                    subtracted: Box::new(Self::decode(fields.get("subtracted").ok_or("subtracted missing")?)?),
                }),
                // ToolHookIngredient: a tool of the tag whose definition binds the hook. The hook becomes a synthetic tag
                // that the tag query answers from the tool definitions (see `hook_tag`).
                Some("tconstruct:tool_hook") => {
                    let tag = match fields.get("tag") { Some(tag) => id(tag)?, None => Identifier::new("tconstruct", "modifiable").expect("valid tag") };
                    let hook = id(fields.get("hook").ok_or("hook missing")?)?;
                    Ok(Self::Intersection(vec![Self::Tag(tag), Self::Tag(hook_tag(&hook))]))
                }
                Some(other) => Err(format!("unimplemented ingredient type {other}")),
            },
            _ => Err("ingredient must be an object or a nonempty array".into()),
        }
    }
    /// Matches an item that has no crafting remainder (every ordinary item and an empty container).
    pub fn matches(&self, item: &Identifier, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        self.matches_with_remainder(item, in_tag, false)
    }
    /// `has_remainder` is whether the stack has a crafting remainder item, which only no_container looks at.
    pub fn matches_with_remainder(&self, item: &Identifier, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool, has_remainder: bool) -> bool {
        match self {
            Self::Item(own) => own == item,
            Self::Tag(tag) => in_tag(item, tag),
            Self::Any(members) => members.iter().any(|member| member.matches_with_remainder(item, in_tag, has_remainder)),
            Self::Difference { base, subtracted } => base.matches_with_remainder(item, in_tag, has_remainder) && !subtracted.matches_with_remainder(item, in_tag, has_remainder),
            Self::Intersection(children) => children.iter().all(|child| child.matches_with_remainder(item, in_tag, has_remainder)),
            Self::NoContainer(nested) => !has_remainder && nested.matches_with_remainder(item, in_tag, has_remainder),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Recipe {
    pub id: Identifier,
    pub ingredient: Ingredient,
    pub material: MaterialVariant,
    /// How many `value` units make one ingot's worth of the material.
    pub needed: i32,
    pub value: i32,
    /// A stack of value 1 handed back when a craft uses part of an item (ignored when value is 1).
    pub leftover: Option<Leftover>,
}

/// ItemOutput for a leftover: a fixed item, or a tag the host resolves to its preferred item.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum LeftoverSource { Item(Identifier), Tag(Identifier) }
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Leftover { pub source: LeftoverSource, pub count: u32 }

impl Leftover {
    fn decode(value: &Value) -> Result<Option<Self>, String> {
        // A bare ID string is a single item.
        if let Value::String(item) = value { return Ok(Some(Self { source: LeftoverSource::Item(Identifier::parse(item).map_err(|e| e.to_string())?), count: 1 })); }
        let fields = match value { Value::Null => return Ok(None), Value::Object(fields) if fields.is_empty() => return Ok(None), Value::Object(fields) => fields, _ => return Err("leftover must be an object or item ID".into()) };
        let count = match fields.get("count") {
            None => 1,
            Some(Value::Number(n)) => { let n = crate::materials::number_int(&n.to_string()); if n >= 1 { n as u32 } else { return Err("leftover count must be at least 1".into()); } }
            Some(_) => return Err("leftover count must be a number".into()),
        };
        let source = match (fields.get("item").and_then(Value::as_str), fields.get("tag").and_then(Value::as_str)) {
            (Some(item), None) => LeftoverSource::Item(Identifier::parse(item).map_err(|e| e.to_string())?),
            (None, Some(tag)) => LeftoverSource::Tag(Identifier::parse(tag).map_err(|e| e.to_string())?),
            _ => return Err("leftover needs exactly one of item or tag".into()),
        };
        Ok(Some(Self { source, count }))
    }
}

impl Recipe {
    fn decode(id: &Identifier, value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("recipe must be an object")?;
        let number = |key: &str| match fields.get(key) {
            Some(Value::Number(n)) => { let n = crate::materials::number_int(&n.to_string()); if n >= 1 { Ok(n) } else { Err(format!("{key} must be at least 1")) } }
            _ => Err(format!("{key} must be a number")),
        };
        Ok(Self {
            id: id.clone(),
            ingredient: Ingredient::decode(fields.get("ingredient").ok_or("ingredient missing")?)?,
            material: MaterialVariant::parse(fields.get("material").and_then(Value::as_str).ok_or("material must be a string")?).ok_or("invalid material variant")?,
            needed: number("needed")?,
            value: number("value")?,
            leftover: None,
        }).and_then(|mut recipe| {
            // MaterialRecipe ignores a leftover when the value is 1: it could never be partly used.
            recipe.leftover = fields.get("leftover").map_or(Ok(None), Leftover::decode)?.filter(|_| recipe.value > 1);
            Ok(recipe)
        })
    }
    /// IMaterialValue.getItemsUsed: the items consumed to pay `cost` (rounded up).
    pub fn items_used(&self, cost: i32) -> i32 {
        let needed = cost.wrapping_mul(self.needed);
        let mut used = needed / self.value;
        if needed % self.value != 0 { used += 1; }
        used
    }
    /// IMaterialValue.getRemainder: material left over once `cost` is paid from whole items.
    pub fn remainder(&self, cost: i32) -> i32 {
        let remainder = self.value.wrapping_sub(cost.wrapping_mul(self.needed)) % self.value;
        if remainder < 0 { remainder + self.value } else { remainder }
    }
    /// IMaterialValue.getLeftover(itemCost): the leftover and its count for the unused remainder.
    pub fn leftover_for(&self, cost: i32) -> Option<(&LeftoverSource, u32)> {
        let remainder = self.remainder(cost);
        let leftover = self.leftover.as_ref().filter(|_| remainder > 0)?;
        let count = (leftover.count as i64 * i64::from(remainder) / i64::from(self.needed)) as u32;
        (count > 0).then_some((&leftover.source, count))
    }
    /// MaterialRecipe.scaleRepair: the durability per item for a material repair amount.
    pub fn scale_repair(&self, amount: f32) -> f32 { self.value as f32 * amount / 3.0 / self.needed as f32 }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Recipes { recipes: Vec<Recipe>, pub problems: Vec<String>, pub loaded: bool }

impl Recipes {
    /// Decodes every file; a file whose condition is false is skipped, and a failing one is a problem.
    pub fn load<C: Conditions>(files: &[(Identifier, Value)], conditions: &C) -> Self {
        let mut result = Self { loaded: true, ..Self::default() };
        let mut sorted: Vec<&(Identifier, Value)> = files.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (id, value) in sorted {
            let outcome = (|| -> Result<Option<Recipe>, String> {
                let fields = value.as_object().ok_or("recipe must be an object")?;
                if fields.get("type").and_then(Value::as_str) != Some("tconstruct:material") { return Ok(None); }
                // Every condition is decoded before any is evaluated, and all must hold.
                if let Some(list) = fields.get("conditions") {
                    let parsed = list.as_array().ok_or("conditions must be an array")?.iter().map(|c| conditions.decode(c)).collect::<Result<Vec<_>, _>>()?;
                    for condition in &parsed { if !conditions.test(condition)? { return Ok(None); } }
                }
                Recipe::decode(id, value).map(Some)
            })();
            match outcome {
                Ok(Some(recipe)) => result.recipes.push(recipe),
                Ok(None) => {}
                Err(error) => result.problems.push(format!("{id}: {error}")),
            }
        }
        result
    }
    /// MaterialRecipeCache.findRecipe: the first recipe whose ingredient matches the item.
    pub fn find(&self, item: &Identifier, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Option<&Recipe> {
        self.recipes.iter().find(|recipe| recipe.ingredient.matches(item, in_tag))
    }
    pub fn len(&self) -> usize { self.recipes.len() }
}

/// Installs the reload listener. A missing condition context clears the recipes and fails the
/// reload step rather than loading with guessed answers; server teardown clears them too.
pub(crate) fn register<C: Conditions + 'static>(context: impl Fn() -> Result<C, String> + Send + Sync + 'static) -> Result<Arc<RwLock<Recipes>>, String> {
    let state = Arc::new(RwLock::new(Recipes::default()));
    let output = state.clone();
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "material_recipes").unwrap(), FOLDER, move |files| {
            let context = context();
            let mut state = output.write().map_err(|_| "material recipes poisoned")?;
            match context {
                Ok(context) => {
                    *state = Recipes::load(files, &context);
                    for problem in &state.problems { eprintln!("Tinkers: {problem}"); }
                    Ok(())
                }
                Err(error) => { *state = Recipes { problems: vec![error.clone()], ..Recipes::default() }; Err(error) }
            }
        })?;
    let stopped = state.clone();
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| *stopped.write().expect("material recipes poisoned") = Recipes::default());
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_tool_hook_ingredient_needs_the_tag_and_the_hook_through_a_synthetic_tag() {
        let hook = Identifier::parse("test:hook").unwrap();
        assert_eq!(hook_of_tag(&hook_tag(&hook)), Some(hook.clone()));
        assert_eq!(hook_of_tag(&Identifier::parse("tconstruct:modifiable").unwrap()), None);
        let ingredient = Ingredient::decode(&serde_json::json!({"type":"tconstruct:tool_hook","hook":"test:hook","tag":"test:tools"})).unwrap();
        let item = Identifier::parse("test:pick").unwrap();
        let with = |tags: &'static [&'static str]| move |_: &Identifier, tag: &Identifier| tags.contains(&tag.to_string().as_str());
        assert!(ingredient.matches(&item, &with(&["test:tools", "tconstruct:tool_hook/test/hook"])));
        assert!(!ingredient.matches(&item, &with(&["test:tools"])), "the tool lacks the hook");
        assert!(!ingredient.matches(&item, &with(&["tconstruct:tool_hook/test/hook"])), "the tool is not in the tag");
    }

    use serde_json::json;
    struct Flags;
    impl Conditions for Flags {
        type Parsed = bool;
        fn decode(&self, value: &Value) -> Result<bool, String> { value.as_bool().ok_or("condition must be boolean".into()) }
        fn test(&self, value: &bool) -> Result<bool, String> { Ok(*value) }
    }
    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    #[test]
    fn ingredients_match_items_tags_lists_and_differences_and_reject_unknown_types() {
        let in_tag = |item: &Identifier, tag: &Identifier| tag.path() == "logs" && item.path().ends_with("_log") || tag.path() == "variant" && item.path() == "oak_log";
        let item = |text: &str| Ingredient::decode(&json!({"item": text})).unwrap();
        assert!(item("minecraft:stick").matches(&id("minecraft:stick"), &in_tag));
        assert!(!item("minecraft:stick").matches(&id("minecraft:dirt"), &in_tag));
        let tag = Ingredient::decode(&json!({"tag":"minecraft:logs"})).unwrap();
        assert!(tag.matches(&id("minecraft:birch_log"), &in_tag));
        let list = Ingredient::decode(&json!([{"tag":"minecraft:logs"},{"item":"minecraft:stick"}])).unwrap();
        assert!(list.matches(&id("minecraft:stick"), &in_tag) && list.matches(&id("minecraft:oak_log"), &in_tag));
        let difference = Ingredient::decode(&json!({"type":"forge:difference","base":{"tag":"minecraft:logs"},"subtracted":{"tag":"tconstruct:variant"}})).unwrap();
        assert!(difference.matches(&id("minecraft:birch_log"), &in_tag) && !difference.matches(&id("minecraft:oak_log"), &in_tag));
        let both = Ingredient::decode(&json!({"type":"forge:intersection","children":[{"tag":"minecraft:logs"},{"tag":"tconstruct:variant"}]})).unwrap();
        assert!(both.matches(&id("minecraft:oak_log"), &in_tag) && !both.matches(&id("minecraft:birch_log"), &in_tag));
        assert!(Ingredient::decode(&json!({"type":"tconstruct:material","item":"tconstruct:pick_head"})).unwrap().matches(&id("tconstruct:pick_head"), &in_tag));
        for bad in [json!({"type":"tconstruct:instrument","item":"minecraft:goat_horn"}), json!({"item":"a:b","tag":"c:d"}), json!([]), json!("minecraft:stick"), json!({})] {
            assert!(Ingredient::decode(&bad).is_err(), "{bad}");
        }
    }
    #[test]
    fn recipes_load_in_id_order_honor_conditions_and_scale_repair() {
        let recipe = |ingredient: Value, material: &str, needed: i64, value: i64| json!({"type":"tconstruct:material","ingredient":ingredient,"material":material,"needed":needed,"value":value});
        let files = vec![
            (id("tconstruct:tools/materials/b"), recipe(json!({"item":"minecraft:iron_ingot"}), "tconstruct:iron", 1, 1)),
            (id("tconstruct:tools/materials/a"), recipe(json!({"item":"minecraft:iron_ingot"}), "tconstruct:pig_iron", 1, 1)),
            (id("tconstruct:tools/materials/off"), json!({"type":"tconstruct:material","conditions":[false],"ingredient":{"item":"minecraft:gold_ingot"},"material":"tconstruct:gold","needed":1,"value":1})),
            (id("tconstruct:tools/materials/block"), recipe(json!({"item":"minecraft:iron_block"}), "tconstruct:iron", 1, 9)),
            (id("tconstruct:tools/materials/horn"), recipe(json!({"type":"tconstruct:instrument","item":"minecraft:goat_horn"}), "tconstruct:horn", 1, 4)),
            (id("tconstruct:tools/materials/bad"), recipe(json!({"item":"minecraft:iron_nugget"}), "tconstruct:iron", 0, 1)),
            (id("tconstruct:tools/materials/other"), json!({"type":"tconstruct:material_melting"})),
        ];
        let recipes = Recipes::load(&files, &Flags);
        assert_eq!(recipes.len(), 3, "a, b and block; conditions, other types and bad data do not load");
        assert_eq!(recipes.problems.len(), 2, "{:?}", recipes.problems);
        let none = |_: &Identifier, _: &Identifier| false;
        // Two recipes share an ingredient: the first in id order wins.
        assert_eq!(recipes.find(&id("minecraft:iron_ingot"), &none).unwrap().material.to_string(), "tconstruct:pig_iron");
        assert!(recipes.find(&id("minecraft:gold_ingot"), &none).is_none());
        // value 9 of needed 1 per item: 9 * amount / 3 / 1.
        let block = recipes.find(&id("minecraft:iron_block"), &none).unwrap();
        assert!((block.scale_repair(30.0) - 90.0).abs() < 1e-4);
    }
}
