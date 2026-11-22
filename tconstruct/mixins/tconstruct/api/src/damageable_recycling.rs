//! Part builder damageable recycling (`tconstruct:part_builder_recycling`), from SlimeKnights
//! PartBuilderRecycle (MIT; see LICENSE): an item with no materials of its own — a vanilla tool or
//! armor piece, a staff, the flint and brick — in the material slot and a pattern item break into
//! fixed results, scaled by the stack's remaining durability. The recipes sit in two folders of the
//! original's tree (`recipes/tools/recycling` beside the tool recycling, `recipes/tables/recycling`),
//! so the registry fills from two listeners, shaped like the smeltery data's.
use crate::{
    fluid_ingredient::ItemOutput,
    material_recipes::Ingredient,
    materials::Conditions,
    tool_stack,
};
use pluto_lib::{Identifier, ItemSlot, ResourceManagerHelper, ResourceType, ServerLifecycleEvents};
use serde_json::Value;
use std::{collections::BTreeMap, sync::{Arc, RwLock}};

pub(crate) const TOOLS_FOLDER: &str = "recipes/tools/recycling";
pub(crate) const TABLES_FOLDER: &str = "recipes/tables/recycling";
const RECIPE_TYPE: &str = "tconstruct:part_builder_recycling";
/// The tag of items the original treats as modifiable for the no-upgrades rule (`TinkerTags.Items.MODIFIABLE`).
const MODIFIABLE: (&str, &str) = ("tconstruct", "modifiable");

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Recipe {
    pub id: Identifier,
    /// The item to recycle (an ingredient: an item or a tag).
    pub tool: Ingredient,
    /// The item the pattern slot must hold.
    pub pattern: Ingredient,
    /// The fixed results by the pattern that selects them, in pattern order.
    pub results: BTreeMap<Identifier, ItemOutput>,
}

impl Recipe {
    fn decode(id: &Identifier, value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("recipe must be an object")?;
        let tool = Ingredient::decode(fields.get("tool").ok_or("tool missing")?)?;
        let pattern = Ingredient::decode(fields.get("pattern").ok_or("pattern missing")?)?;
        let results = fields.get("results").and_then(Value::as_object).filter(|results| !results.is_empty()).ok_or("results must be a nonempty object")?;
        let results = results.iter().map(|(pattern, output)| {
            Ok((Identifier::parse(pattern).map_err(|e| e.to_string())?, ItemOutput::decode(output, true)?))
        }).collect::<Result<BTreeMap<_, _>, String>>()?;
        Ok(Self { id: id.clone(), tool, pattern, results })
    }

    /// The total result count a fresh stack breaks into (`resultCount`).
    pub fn result_count(&self) -> i32 { self.results.values().map(|output| output.count).sum() }

    /// getAmount: the result budget of a stack, scaled by its remaining durability; an item that
    /// cannot be damaged (max damage 0) always gives the full count.
    pub fn budget(&self, damage: u32, max_damage: i32) -> i32 {
        let total = self.result_count();
        if max_damage <= 0 { return total; }
        (i64::from(total) * i64::from(max_damage - i32::try_from(damage).unwrap_or(i32::MAX).min(max_damage)) / i64::from(max_damage)) as i32
    }

    /// partialMatch: the pattern and the tool both match.
    pub fn partial_match(&self, tool: &Identifier, pattern: &Identifier, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        self.pattern.matches(pattern, in_tag) && self.tool.matches(tool, in_tag)
    }

    /// matches: a partial match whose budget is not spent, on a stack that carries no upgrades
    /// (a modifiable item) or no enchantments (anything else). `max_damage` is the stack's own
    /// max damage, resolved by the caller (the stack's component, else the item's prototype).
    pub fn matches(&self, stack: &ItemSlot, pattern: &Identifier, max_damage: i32, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        if !self.partial_match(&stack.item, pattern, in_tag) || self.budget(stack.damage, max_damage) <= 0 { return false; }
        let modifiable = Identifier::new(MODIFIABLE.0, MODIFIABLE.1).expect("valid tag");
        if in_tag(&stack.item, &modifiable) {
            tool_stack::upgrades(&stack.custom_data).entries().is_empty()
        } else {
            stack.enchantments.as_ref().is_none_or(|enchantments| enchantments.is_empty())
        }
    }

    /// The patterns the window offers as buttons, in pattern order.
    pub fn buttons(&self) -> Vec<Identifier> { self.results.keys().cloned().collect() }

    /// assemble: the chosen pattern's result, its count capped by the stack's budget. A tag result
    /// resolves through `first_of` (the part builder's tag stand-in: the tag's first member).
    pub fn assemble(&self, chosen: &Identifier, stack: &ItemSlot, max_damage: i32, first_of: &dyn Fn(&crate::fluid_ingredient::Source) -> Option<Identifier>) -> Option<ItemSlot> {
        let output = self.results.get(chosen)?;
        let item = first_of(&output.source)?;
        let count = self.budget(stack.damage, max_damage).min(output.count);
        (count > 0).then(|| ItemSlot::new(item, count as u32))
    }

    /// getLeftover: what the budget leaves after the chosen result buys a second result, picked at
    /// random among the others (`random` is a value in [0, 1)).
    pub fn leftover(&self, chosen: &Identifier, stack: &ItemSlot, max_damage: i32, first_of: &dyn Fn(&crate::fluid_ingredient::Source) -> Option<Identifier>, random: &mut dyn FnMut() -> f32) -> Option<ItemSlot> {
        let chosen_output = self.results.get(chosen)?;
        let mut remaining = self.budget(stack.damage, max_damage) - chosen_output.count;
        if remaining <= 0 || self.results.len() <= 1 { return None; }
        let alternatives: Vec<&ItemOutput> = self.results.iter().filter(|(pattern, _)| *pattern != chosen).map(|(_, output)| output).collect();
        if alternatives.is_empty() { return None; }
        let pick = ((random() * alternatives.len() as f32) as usize).min(alternatives.len() - 1);
        let output = alternatives[pick];
        remaining = remaining.min(output.count);
        let item = first_of(&output.source)?;
        (remaining > 0).then(|| ItemSlot::new(item, remaining as u32))
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Recycles {
    tools: Vec<Recipe>,
    tables: Vec<Recipe>,
    tools_problems: Vec<String>,
    tables_problems: Vec<String>,
    tools_loaded: bool,
    tables_loaded: bool,
}

impl Recycles {
    /// True once both folders of the current reload have arrived.
    pub(crate) fn loaded(&self) -> bool { self.tools_loaded && self.tables_loaded }

    pub fn len(&self) -> usize { self.tools.len() + self.tables.len() }

    /// The current reload's problems from both folders.
    pub fn problems(&self) -> Vec<&String> { self.tools_problems.iter().chain(&self.tables_problems).collect() }

    fn load<C: Conditions>(files: &[(Identifier, Value)], conditions: &C) -> (Vec<Recipe>, Vec<String>) {
        let mut recipes = Vec::new();
        let mut problems = Vec::new();
        let mut sorted: Vec<&(Identifier, Value)> = files.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (id, value) in sorted {
            let outcome = (|| -> Result<Option<Recipe>, String> {
                let fields = value.as_object().ok_or("recipe must be an object")?;
                if fields.get("type").and_then(Value::as_str) != Some(RECIPE_TYPE) { return Ok(None); }
                if let Some(list) = fields.get("conditions") {
                    let parsed = list.as_array().ok_or("conditions must be an array")?.iter().map(|c| conditions.decode(c)).collect::<Result<Vec<_>, _>>()?;
                    for condition in &parsed { if !conditions.test(condition)? { return Ok(None); } }
                }
                Recipe::decode(id, value).map(Some)
            })();
            match outcome {
                Ok(Some(recipe)) => recipes.push(recipe),
                Ok(None) => {}
                Err(error) => problems.push(format!("{id}: {error}")),
            }
        }
        (recipes, problems)
    }

    /// Both folders at once, as a completed reload leaves the registry (the domain tests load the
    /// shipped files through here; the listeners fill the same state one folder at a time).
    pub(crate) fn load_folders<C: Conditions>(tools: &[(Identifier, Value)], tables: &[(Identifier, Value)], conditions: &C) -> Self {
        let (tools, tools_problems) = Self::load(tools, conditions);
        let (tables, tables_problems) = Self::load(tables, conditions);
        Self { tools, tables, tools_problems, tables_problems, tools_loaded: true, tables_loaded: true }
    }

    /// The first recipe that fully matches, the tools folder before the tables folder, each in ID order.
    pub fn find(&self, stack: &ItemSlot, pattern: &Identifier, max_damage: i32, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Option<&Recipe> {
        self.tools.iter().chain(&self.tables).find(|recipe| recipe.matches(stack, pattern, max_damage, in_tag))
    }
}

/// Reload listeners, shaped like the smeltery data's: a missing condition context clears that
/// folder's recipes and fails the reload step; server teardown clears both.
pub(crate) fn register<C: Conditions + 'static>(context: impl Fn() -> Result<C, String> + Send + Sync + 'static) -> Result<Arc<RwLock<Recycles>>, String> {
    let context = Arc::new(context);
    let state = Arc::new(RwLock::new(Recycles::default()));
    for (name, folder, tools) in [("damageable_recycling_tools", TOOLS_FOLDER, true), ("damageable_recycling_tables", TABLES_FOLDER, false)] {
        let output = state.clone();
        let context = context.clone();
        ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
            Identifier::new("tconstruct", name).unwrap(), folder, move |files| {
                let context = context();
                let mut state = output.write().map_err(|_| "damageable recycling poisoned")?;
                match context {
                    Ok(context) => {
                        let (recipes, problems) = Recycles::load(files, &context);
                        for problem in &problems { eprintln!("Tinkers: {problem}"); }
                        if tools { state.tools = recipes; state.tools_problems = problems; state.tools_loaded = true; }
                        else { state.tables = recipes; state.tables_problems = problems; state.tables_loaded = true; }
                        Ok(())
                    }
                    Err(error) => {
                        if tools { state.tools = Vec::new(); state.tools_problems = vec![error.clone()]; state.tools_loaded = false; }
                        else { state.tables = Vec::new(); state.tables_problems = vec![error.clone()]; state.tables_loaded = false; }
                        Err(error)
                    }
                }
            })?;
    }
    let stopped = state.clone();
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| *stopped.write().expect("damageable recycling poisoned") = Recycles::default());
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fluid_ingredient::Source;
    use serde_json::json;

    struct Always;
    impl Conditions for Always {
        type Parsed = ();
        fn decode(&self, _: &Value) -> Result<(), String> { Ok(()) }
        fn test(&self, _: &()) -> Result<bool, String> { Ok(true) }
    }

    fn in_tag(_: &Identifier, _: &Identifier) -> bool { false }
    fn first_of(source: &Source) -> Option<Identifier> {
        // A tag result stands in with a fixed member, as the window's snapshot stand-in does.
        match source { Source::Id(item) => Some(item.clone()), Source::Tag(_) => Some(id("minecraft:iron_ingot")) }
    }
    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }

    fn staff() -> Recipe {
        Recipe::decode(&id("tconstruct:earth_staff"), &json!({"type": "tconstruct:part_builder_recycling",
            "pattern": {"item": "tconstruct:venom_bottle"},
            "results": {"tconstruct:block": {"count": 2, "item": "tconstruct:greenheart_log"},
                        "tconstruct:crystal": {"count": 2, "item": "tconstruct:earth_slime_crystal"},
                        "tconstruct:ingot": {"count": 1, "tag": "forge:ingots/cobalt"}},
            "tool": {"item": "tconstruct:earth_staff"}})).unwrap()
    }

    #[test]
    fn the_budget_scales_with_the_remaining_durability() {
        let recipe = staff();
        assert_eq!(recipe.result_count(), 5);
        assert_eq!(recipe.budget(0, 0), 5, "an undamageable item gives everything");
        assert_eq!(recipe.budget(0, 100), 5);
        assert_eq!(recipe.budget(50, 100), 2, "half worn: 5 * 50 / 100");
        assert_eq!(recipe.budget(99, 100), 0, "nearly broken: nothing left to recover");
        assert_eq!(recipe.budget(100, 100), 0);
    }

    #[test]
    fn matches_needs_the_pattern_the_tool_and_an_unspent_budget() {
        let recipe = staff();
        let fresh = ItemSlot::new(id("tconstruct:earth_staff"), 1);
        assert!(recipe.matches(&fresh, &id("tconstruct:venom_bottle"), 0, &in_tag));
        assert!(!recipe.matches(&fresh, &id("tconstruct:pattern"), 0, &in_tag), "the wrong pattern item");
        let mut worn = fresh.clone();
        worn.damage = 90;
        assert!(!recipe.matches(&worn, &id("tconstruct:venom_bottle"), 100, &in_tag), "budget 0 at 90% wear of 5 results");
        let mut enchanted = fresh.clone();
        enchanted.enchantments = Some([("minecraft:sharpness".to_owned(), 1)].into_iter().collect());
        assert!(!recipe.matches(&enchanted, &id("tconstruct:venom_bottle"), 0, &in_tag), "an enchanted stack is refused");
    }

    #[test]
    fn assemble_caps_at_the_budget_and_the_leftover_spends_what_remains() {
        let recipe = staff();
        let mut stack = ItemSlot::new(id("tconstruct:earth_staff"), 1);
        stack.damage = 40; // budget 5 * 60 / 100 = 3
        let crystal = id("tconstruct:crystal");
        let made = recipe.assemble(&crystal, &stack, 100, &first_of).unwrap();
        assert_eq!((made.item, made.count), (id("tconstruct:earth_slime_crystal"), 2), "the crystal's own count, under the budget of 3");
        // One budget remains: the leftover is one of the other results, capped at 1.
        let leftover = recipe.leftover(&crystal, &stack, 100, &first_of, &mut || 0.0).unwrap();
        assert_eq!((leftover.item, leftover.count), (id("tconstruct:greenheart_log"), 1), "the first other result, capped by the remaining budget");
        // A fresh stack: budget 5, the crystal takes 2, leaving 3 for a 2-count or 1-count other result.
        let fresh = ItemSlot::new(id("tconstruct:earth_staff"), 1);
        let leftover = recipe.leftover(&crystal, &fresh, 0, &first_of, &mut || 0.0).unwrap();
        assert_eq!(leftover.count, 2, "the block result (count 2) fits in the remaining 3");
    }

    #[test]
    fn both_folders_load_and_other_types_are_left_out() {
        let files = vec![
            (id("tconstruct:flint_and_brick"), json!({"type": "tconstruct:part_builder_recycling", "pattern": {"item": "tconstruct:venom_bottle"},
                "results": {"tconstruct:shard": {"item": "minecraft:flint"}}, "tool": {"item": "tconstruct:flint_and_brick"}})),
            (id("tconstruct:general"), json!({"type": "tconstruct:part_builder_tool_recycling", "tools": {}, "pattern": {"item": "tconstruct:venom_bottle"}})),
        ];
        let (recipes, problems) = Recycles::load(&files, &Always);
        assert_eq!(recipes.len(), 1);
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(recipes[0].buttons(), vec![id("tconstruct:shard")]);
    }
}
