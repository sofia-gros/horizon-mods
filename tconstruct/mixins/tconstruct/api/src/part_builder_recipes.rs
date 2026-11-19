//! Part builder recipes: PartRecipe (a material part from material items) and ItemPartRecipe
//! (an ordinary item from a pattern, optionally with a material), from SlimeKnights
//! (MIT; see LICENSE). The block, its menu and the pattern buttons are separate.
use crate::{
    material_recipes::{Ingredient, Recipe as MaterialRecipe},
    materials::{Conditions, Definitions},
    tool_materials::{set_part_material, MaterialVariant, ToolPart},
};
use pluto_lib::{Identifier, ItemSlot};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

pub(crate) const FOLDER: &str = "recipes/tools/parts/builder";

/// What the part builder holds: the pattern item, the material stack and the material recipe it resolved to.
pub(crate) struct Inventory<'a> {
    pub pattern_item: Option<&'a Identifier>,
    pub stack: Option<(&'a Identifier, u32)>,
    pub material: Option<&'a MaterialRecipe>,
}

pub(crate) struct Context<'a> {
    pub materials: &'a Definitions,
    pub parts: &'a BTreeMap<Identifier, ToolPart>,
    pub in_tag: &'a dyn Fn(&Identifier, &Identifier) -> bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Kind {
    /// PartRecipe: the output is a material part item, built from a material.
    Part { output: Identifier, count: u32, allow_uncraftable: bool },
    /// ItemPartRecipe: an ordinary item; `material` is the one variant it takes, if any.
    Item { material: Option<MaterialVariant>, result: Identifier, count: u32, title_key: Option<Identifier> },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Recipe {
    pub id: Identifier,
    pub pattern: Identifier,
    pub pattern_item: Ingredient,
    pub cost: i32,
    pub kind: Kind,
}

fn int(fields: &Map<String, Value>, key: &str, default: Option<i32>, min: i32) -> Result<i32, String> {
    match fields.get(key) {
        None => default.ok_or_else(|| format!("{key} missing")),
        Some(Value::Number(n)) => { let n = crate::materials::number_int(&n.to_string()); if n >= min { Ok(n) } else { Err(format!("{key} must be at least {min}")) } }
        Some(_) => Err(format!("{key} must be a number")),
    }
}

impl Recipe {
    fn decode(id: &Identifier, fields: &Map<String, Value>) -> Result<Self, String> {
        let kind_name = fields.get("type").and_then(Value::as_str).ok_or("type missing")?;
        let pattern = Identifier::parse(fields.get("pattern").and_then(Value::as_str).ok_or("pattern must be an ID string")?).map_err(|e| e.to_string())?;
        // The default pattern item is anything in the default-patterns tag.
        let pattern_item = match fields.get("pattern_item") {
            None => Ingredient::Tag(Identifier::new("tconstruct", "patterns/default").expect("valid tag")),
            Some(value) => Ingredient::decode(value)?,
        };
        let result_value = fields.get("result");
        let result = result_value.and_then(Value::as_object);
        let item = |fields: &Map<String, Value>| Identifier::parse(fields.get("item").and_then(Value::as_str).ok_or("result item must be an ID string")?).map_err(|e| e.to_string());
        let count = |fields: &Map<String, Value>| int(fields, "count", Some(1), 1).map(|n| n as u32);
        let (cost, kind) = match kind_name {
            "tconstruct:part_builder" => {
                let allow_uncraftable = match fields.get("allow_uncraftable") { None => false, Some(Value::Bool(value)) => *value, Some(_) => return Err("allow_uncraftable must be boolean".into()) };
                {
                let result = result.ok_or("result must be an object")?;
                (int(fields, "cost", None, 1)?, Kind::Part { output: item(result)?, count: count(result)?, allow_uncraftable })
            }
            }
            "tconstruct:item_part_builder" => {
                let material = match fields.get("material") {
                    None | Some(Value::Null) => None,
                    Some(Value::String(text)) => Some(MaterialVariant::parse(text).ok_or("invalid material variant")?).filter(|material| *material != MaterialVariant::unknown()),
                    Some(_) => return Err("material must be a string".into()),
                };
                let cost = int(fields, "cost", Some(0), 0)?;
                if cost == 0 && material.is_some() { return Err("cost must be greater than zero if material is defined".into()); }
                let title_key = match fields.get("title_key") {
                    None | Some(Value::Null) => None,
                    Some(Value::String(text)) => Some(Identifier::parse(text).map_err(|e| e.to_string())?),
                    Some(_) => return Err("title_key must be an ID string".into()),
                };
                // ItemOutput.REQUIRED_STACK: an item ID alone is one of that item, otherwise an object with an optional count.
                let (result, count) = match result_value {
                    Some(Value::String(text)) => (Identifier::parse(text).map_err(|e| e.to_string())?, 1),
                    _ => { let result = result.ok_or("result must be an object or an item ID")?; (item(result)?, count(result)?) }
                };
                (cost, Kind::Item { material, result, count, title_key })
            }
            other => return Err(format!("unsupported recipe type {other}")),
        };
        Ok(Self { id: id.clone(), pattern, pattern_item, cost, kind })
    }

    pub fn allow_uncraftable(&self) -> bool {
        match &self.kind { Kind::Part { allow_uncraftable, .. } => *allow_uncraftable, Kind::Item { .. } => true }
    }

    fn can_use_material(&self, output: &Identifier, material: &MaterialVariant, ctx: &Context) -> bool {
        let Some(id) = material.material_id() else { return false; };
        let craftable = ctx.materials.materials.get(ctx.materials.resolve_redirect(&id)).is_some_and(|definition| definition.craftable);
        (self.allow_uncraftable() || craftable) && ctx.parts.get(output).is_some_and(|part| part.can_use(material, ctx.materials))
    }

    /// partialMatch: can this recipe match once more is placed? An empty material slot matches.
    pub fn partial_match(&self, inv: &Inventory, ctx: &Context) -> bool {
        let Some(pattern_item) = inv.pattern_item else { return false; };
        if !self.pattern_item.matches(pattern_item, ctx.in_tag) { return false; }
        let Some((stack_item, _)) = inv.stack else { return true; };
        match &self.kind {
            Kind::Part { output, .. } => {
                // No sense allowing the output itself as the input.
                if stack_item == output { return false; }
                let Some(recipe) = inv.material else { return false; };
                self.can_use_material(output, &recipe.material, ctx)
            }
            Kind::Item { material, .. } => {
                let Some(material) = material else { return false; };
                inv.material.is_some_and(|recipe| material.matches(&recipe.material))
            }
        }
    }

    /// matches: the material is usable and enough of it is present.
    pub fn matches(&self, inv: &Inventory, ctx: &Context) -> bool {
        let count = inv.stack.map_or(0, |(_, count)| count);
        match &self.kind {
            Kind::Part { output, .. } => inv.material.is_some_and(|recipe| {
                self.can_use_material(output, &recipe.material, ctx) && i64::from(count) >= i64::from(recipe.items_used(self.cost))
            }),
            Kind::Item { material, .. } => {
                // An item recipe without a material needs no stack; with one, enough of it.
                let Some(material) = material else { return inv.stack.is_none(); };
                inv.material.is_some_and(|recipe| material.matches(&recipe.material) && i64::from(count) >= i64::from(recipe.items_used(self.cost)))
            }
        }
    }

    /// getItemsUsed: how many material items one craft consumes (1 when there is no material).
    pub fn items_used(&self, inv: &Inventory) -> i32 {
        inv.material.map_or(1, |recipe| recipe.items_used(self.cost))
    }

    /// assemble: the crafted stack.
    pub fn assemble(&self, inv: &Inventory) -> ItemSlot {
        match &self.kind {
            Kind::Part { output, count, .. } => {
                let mut material = MaterialVariant::unknown();
                let mut count = i64::from(*count);
                if let Some(recipe) = inv.material {
                    material = recipe.material.clone();
                    // With no leftover, a worth-more item makes proportionally more parts.
                    if recipe.leftover.is_none() && recipe.value > self.cost { count = count * i64::from(recipe.value) / i64::from(self.cost); }
                }
                let mut stack = ItemSlot::new(output.clone(), u32::try_from(count).unwrap_or(u32::MAX));
                set_part_material(&mut stack, &material);
                stack
            }
            Kind::Item { result, count, .. } => ItemSlot::new(result.clone(), *count),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Recipes { recipes: Vec<Recipe>, pub problems: Vec<String>, pub loaded: bool }

impl Recipes {
    pub fn load<C: Conditions>(files: &[(Identifier, Value)], conditions: &C) -> Self {
        let mut result = Self { loaded: true, ..Self::default() };
        let mut sorted: Vec<&(Identifier, Value)> = files.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (id, value) in sorted {
            let outcome = (|| -> Result<Option<Recipe>, String> {
                let fields = value.as_object().ok_or("recipe must be an object")?;
                if !matches!(fields.get("type").and_then(Value::as_str), Some("tconstruct:part_builder" | "tconstruct:item_part_builder")) { return Ok(None); }
                if let Some(list) = fields.get("conditions") {
                    let parsed = list.as_array().ok_or("conditions must be an array")?.iter().map(|c| conditions.decode(c)).collect::<Result<Vec<_>, _>>()?;
                    for condition in &parsed { if !conditions.test(condition)? { return Ok(None); } }
                }
                Recipe::decode(id, fields).map(Some)
            })();
            match outcome {
                Ok(Some(recipe)) => result.recipes.push(recipe),
                Ok(None) => {}
                Err(error) => result.problems.push(format!("{id}: {error}")),
            }
        }
        result
    }
    pub fn len(&self) -> usize { self.recipes.len() }
    /// The patterns offered for the current inventory, in recipe order, each once (recipes sharing
    /// a pattern are merged), limited to recipes that can still match.
    pub fn patterns(&self, inv: &Inventory, ctx: &Context) -> Vec<Identifier> {
        self.buttons(inv, ctx).into_iter().map(|(pattern, _)| pattern).collect()
    }
    /// The pattern buttons: the first recipe (in ID order) of each pattern decides its cost, and
    /// buttons are sorted by that cost, then by pattern.
    pub fn buttons(&self, inv: &Inventory, ctx: &Context) -> Vec<(Identifier, i32)> {
        let mut matching: Vec<&Recipe> = self.recipes.iter().filter(|recipe| recipe.partial_match(inv, ctx)).collect();
        matching.sort_by(|a, b| a.id.cmp(&b.id));
        let mut buttons: Vec<(Identifier, i32)> = Vec::new();
        for recipe in matching {
            if !buttons.iter().any(|(pattern, _)| *pattern == recipe.pattern) { buttons.push((recipe.pattern.clone(), recipe.cost)); }
        }
        buttons.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        buttons
    }
    /// The recipe for the chosen pattern that fully matches, in recipe order.
    pub fn find(&self, pattern: &Identifier, inv: &Inventory, ctx: &Context) -> Option<&Recipe> {
        self.recipes.iter().find(|recipe| &recipe.pattern == pattern && recipe.partial_match(inv, ctx) && recipe.matches(inv, ctx))
    }
}

/// Reload listener, shaped like the other recipe listeners: a missing condition context clears and fails.
pub(crate) fn register<C: Conditions + 'static>(context: impl Fn() -> Result<C, String> + Send + Sync + 'static) -> Result<std::sync::Arc<std::sync::RwLock<Recipes>>, String> {
    use pluto_lib::{ResourceManagerHelper, ResourceType, ServerLifecycleEvents};
    let state = std::sync::Arc::new(std::sync::RwLock::new(Recipes::default()));
    let output = state.clone();
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "part_builder_recipes").unwrap(), FOLDER, move |files| {
            let context = context();
            let mut state = output.write().map_err(|_| "part builder recipes poisoned")?;
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
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| *stopped.write().expect("part builder recipes poisoned") = Recipes::default());
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    struct Flags;
    impl Conditions for Flags {
        type Parsed = bool;
        fn decode(&self, value: &Value) -> Result<bool, String> { value.as_bool().ok_or("condition must be boolean".into()) }
        fn test(&self, value: &bool) -> Result<bool, String> { Ok(*value) }
    }
    fn recipe(extra: Value) -> Recipe {
        let mut base = json!({"type":"tconstruct:part_builder","cost":2,"pattern":"tconstruct:pick_head","pattern_item":{"item":"tconstruct:pattern"},"result":{"item":"tconstruct:pick_head"}});
        base.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        Recipe::decode(&id("test:r"), base.as_object().unwrap()).unwrap()
    }
    fn material(name: &str, value: i32, needed: i32, leftover: bool) -> MaterialRecipe {
        let mut json = json!({"type":"tconstruct:material","ingredient":{"item":"a:b"},"material":format!("tconstruct:{name}"),"value":value,"needed":needed});
        if leftover { json["leftover"] = json!({"item":"a:leftover","count":1}); }
        let files = vec![(id("test:m"), json)];
        let recipes = crate::material_recipes::Recipes::load(&files, &Flags);
        assert!(recipes.problems.is_empty(), "{:?}", recipes.problems);
        recipes.find(&id("a:b"), &|_, _| false).expect("the recipe loads").clone()
    }
    fn inv<'a>(pattern: &'a Identifier, stack: &'a Identifier, count: u32, material: Option<&'a MaterialRecipe>) -> Inventory<'a> {
        Inventory { pattern_item: Some(pattern), stack: Some((stack, count)), material }
    }
    #[test]
    fn material_value_math_matches_the_original() {
        let m = material("wood", 2, 3, true);
        assert_eq!((m.items_used(2), m.remainder(2)), (3, 0));
        // value 4: cost 1 takes one item (4 units) and leaves 3 units.
        let m = material("wood", 4, 1, true);
        assert_eq!((m.items_used(1), m.remainder(1), m.leftover_for(1).map(|(_, n)| n)), (1, 3, Some(3)));
        assert_eq!(material("wood", 4, 1, false).leftover_for(1), None);
        assert_eq!(material("wood", 1, 1, true).leftover, None, "a leftover is ignored when the value is 1");
    }
    #[test]
    fn a_part_recipe_needs_a_usable_material_and_enough_of_it() {
        let r = recipe(json!({}));
        // Materials whose stats are present for the head stat type are usable; craftability is checked.
        let mut definitions = Definitions::default();
        for (name, craftable) in [("wood", true), ("obsidian", false)] {
            let material = id(&format!("tconstruct:{name}"));
            definitions.materials.insert(material.clone(), crate::materials::Material { id: material.clone(), tier: 1, sort_order: 0, rarity: crate::materials::Rarity::Common, craftable, hidden: false });
            let head = crate::material_stats::Stats::Head { durability: 10, mining_speed: 1.0, tier: id("minecraft:wood"), attack: 1.0 };
            definitions.stats.entry(material).or_default().insert(id("tconstruct:head"), head);
        }
        let parts: BTreeMap<Identifier, ToolPart> = [(id("tconstruct:pick_head"), ToolPart { item: id("tconstruct:pick_head"), stat: id("tconstruct:head") })].into();
        let in_tag = |_: &Identifier, _: &Identifier| false;
        let ctx = Context { materials: &definitions, parts: &parts, in_tag: &in_tag };
        let (pattern, plank) = (id("tconstruct:pattern"), id("a:b"));
        let wood = material("wood", 1, 1, false);
        assert!(r.partial_match(&inv(&pattern, &plank, 1, Some(&wood)), &ctx) && !r.matches(&inv(&pattern, &plank, 1, Some(&wood)), &ctx), "cost 2 needs 2 items");
        assert!(r.matches(&inv(&pattern, &plank, 2, Some(&wood)), &ctx));
        let crafted = r.assemble(&inv(&pattern, &plank, 2, Some(&wood)));
        assert_eq!((crafted.item.to_string(), crate::tool_materials::part_material(&crafted.custom_data).to_string()), ("tconstruct:pick_head".to_owned(), "tconstruct:wood".to_owned()));
        let obsidian = material("obsidian", 1, 1, false);
        assert!(!r.partial_match(&inv(&pattern, &plank, 9, Some(&obsidian)), &ctx), "an uncraftable material needs allow_uncraftable");
        assert!(recipe(json!({"allow_uncraftable":true})).matches(&inv(&pattern, &plank, 9, Some(&obsidian)), &ctx));
        assert!(r.partial_match(&Inventory { pattern_item: Some(&pattern), stack: None, material: None }, &ctx), "an empty material slot still matches");
        assert!(!r.partial_match(&Inventory { pattern_item: Some(&plank), stack: None, material: None }, &ctx), "the wrong pattern item");
        // A worth-more item with no leftover yields proportionally more parts.
        let rich = material("wood", 6, 1, false);
        assert_eq!(r.assemble(&inv(&pattern, &plank, 1, Some(&rich))).count, 3);
    }
    #[test]
    fn item_recipes_validate_and_match_with_or_without_a_material() {
        let item = |extra: Value| {
            let mut base = json!({"type":"tconstruct:item_part_builder","pattern":"tconstruct:x","result":{"item":"a:out","count":4}});
            base.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
            Recipe::decode(&id("test:i"), base.as_object().unwrap())
        };
        assert!(item(json!({"material":"tconstruct:wood"})).is_err(), "a material needs a cost");
        let r = item(json!({})).unwrap();
        let definitions = crate::materials::Definitions::default();
        let parts = BTreeMap::new();
        let in_tag = |_: &Identifier, tag: &Identifier| tag.path() == "patterns/default";
        let ctx = Context { materials: &definitions, parts: &parts, in_tag: &in_tag };
        let any = id("tconstruct:pattern");
        assert!(r.matches(&Inventory { pattern_item: Some(&any), stack: None, material: None }, &ctx));
        assert!(!r.partial_match(&Inventory { pattern_item: Some(&any), stack: Some((&any, 1)), material: None }, &ctx), "no material expected");
        assert_eq!(r.assemble(&Inventory { pattern_item: Some(&any), stack: None, material: None }).count, 4);
    }
    #[test]
    fn recipes_load_with_conditions_and_report_problems() {
        let good = json!({"type":"tconstruct:part_builder","cost":1,"pattern":"a:p","result":{"item":"a:o"}});
        let files = vec![(id("test:b"), good.clone()), (id("test:off"), { let mut v = good.clone(); v["conditions"] = json!([false]); v }),
            (id("test:bad"), json!({"type":"tconstruct:part_builder","cost":0,"pattern":"a:p","result":{"item":"a:o"}})), (id("test:other"), json!({"type":"tconstruct:material"}))];
        let recipes = Recipes::load(&files, &Flags);
        assert_eq!((recipes.len(), recipes.problems.len()), (1, 1), "{:?}", recipes.problems);
    }
}
