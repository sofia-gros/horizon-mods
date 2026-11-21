//! The enchantment converting worktable recipe (`tconstruct:enchantment_converting`,
//! EnchantmentConvertingRecipe): an enchanted book — or, for the second shipped recipe, an
//! enchantable tool — gives up its enchantments as modifier crystals, at the cost of the
//! recipe's inputs per level converted. This is the one consumer of the enchantment map
//! (`enchantment_map`): the options a stack offers are its enchantments, mapped.
//! SlimeKnights (MIT; see LICENSE) defines the semantics; this only mirrors them.
//!
//! The worktable window still dispatches only the removal recipe (worktable_menu), so this
//! loader and its result logic are wired into the reload and the registries, not yet into the
//! window. Two inputs the original reads from the game are supplied by the caller: whether a
//! tool stack is enchantable (Item.isEnchantable — Tinkers' own tools always answer false,
//! ModifiableItem.isEnchantable, and no PlutoLib API reports another item's answer) and the
//! stack's enchantments (for a book, its stored enchantments; both are plain slot facts in
//! every context that has the stack).
use crate::{
    enchantment_map::EnchantmentMap,
    materials::Conditions,
    material_recipes::StackFacts,
    modifier_modules::CausePredicate,
    modifier_recipes::{check_match_facts, Sized, CRYSTAL_ITEM, CRYSTAL_KEY},
};
use pluto_lib::{Identifier, ItemSlot, Nbt, ResourceManagerHelper, ResourceType, ServerLifecycleEvents};
use serde_json::Value;
use std::{collections::BTreeMap, sync::{Arc, RwLock}};

pub(crate) const FOLDER: &str = "recipes/tools/modifiers/worktable";
const ENCHANTED_BOOK: (&str, &str) = ("minecraft", "enchanted_book");
const BOOK: (&str, &str) = ("minecraft", "book");

#[derive(Clone, Debug)]
pub(crate) struct Recipe {
    pub id: Identifier,
    /// The recipe's display group (`name` in the data; both shipped recipes use "modifiers").
    pub name: String,
    pub inputs: Vec<Sized>,
    /// Match enchanted books (true) or enchantable tools (false).
    pub match_book: bool,
    /// Hand the input back with one level of the converted enchantment removed; the crystal
    /// is then level 1 whatever the enchantment's level was.
    pub return_unenchanted: bool,
    /// Which modifiers this recipe may convert to (absent in the data: any).
    pub predicate: CausePredicate,
}

impl Recipe {
    fn decode(id: &Identifier, value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("recipe must be an object")?;
        let name = fields.get("name").and_then(Value::as_str).ok_or("name must be a string")?.to_owned();
        let inputs = fields.get("inputs").and_then(Value::as_array).filter(|inputs| !inputs.is_empty()).ok_or("inputs must be a nonempty array")?
            .iter().map(Sized::decode).collect::<Result<Vec<_>, _>>()?;
        let boolean = |key: &str| fields.get(key).and_then(Value::as_bool).ok_or_else(|| format!("{key} must be a boolean"));
        let predicate = CausePredicate::decode(fields.get("modifier_predicate"))?.ok_or("unsupported modifier predicate")?;
        Ok(Self { id: id.clone(), name, inputs, match_book: boolean("match_book")?, return_unenchanted: boolean("return_unenchanted")?, predicate })
    }

    /// matches: the tool slot holds an enchanted book (for a book recipe) or an enchantable
    /// stack, and the inputs are exactly this recipe's. `enchantable` is the stack's own
    /// answer; see the module note for who supplies it.
    pub(crate) fn matches(&self, tool: &Identifier, enchantable: bool, inputs: &[Option<(Identifier, u32)>],
                          facts: &[StackFacts], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        let tool_matches = if self.match_book {
            tool.namespace() == ENCHANTED_BOOK.0 && tool.path() == ENCHANTED_BOOK.1
        } else { enchantable };
        tool_matches && check_match_facts(&self.inputs, inputs, facts, in_tag).is_some()
    }

    /// getModifierOptions for a stack: one entry per enchantment that maps to a modifier this
    /// recipe accepts, at the enchantment's level — or at level 1 when the input is returned,
    /// as only one level is converted per craft. Distinct (modifier, level) pairs.
    pub(crate) fn options(&self, enchantments: &BTreeMap<String, i32>, map: &EnchantmentMap,
                          modifier_in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Vec<(Identifier, i32)> {
        let mut options = Vec::new();
        for (name, level) in enchantments {
            let Ok(enchantment) = Identifier::parse(name) else { continue };
            let Some(modifier) = map.modifier_for(&enchantment) else { continue };
            if !self.predicate.matches(Some(modifier), modifier_in_tag) { continue; }
            let entry = (modifier.clone(), if self.return_unenchanted { 1 } else { *level });
            if !options.contains(&entry) { options.push(entry); }
        }
        options
    }

    /// getResult: the modifier crystal for the selected option — a crystal stack whose count
    /// is the level and whose data names the modifier, as ModifierCrystalItem.withModifier
    /// makes it. Above level 1, every input must be present at amount × level in its own
    /// slot; the original fails the craft (its TOO_FEW result) when one is short, which is
    /// None here.
    pub(crate) fn result(&self, selected: &(Identifier, i32), inputs: &[Option<(Identifier, u32)>],
                         facts: &[StackFacts], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Option<ItemSlot> {
        let (modifier, level) = selected;
        if *level > 1 {
            let mut used = vec![false; inputs.len()];
            for ingredient in &self.inputs {
                let needed = ingredient.amount.saturating_mul(u32::try_from(*level).unwrap_or(0));
                let slot = (0..inputs.len()).find(|&slot| !used[slot] && inputs[slot].as_ref().is_some_and(|stack| {
                    stack.1 >= needed && ingredient.ingredient.matches_facts(&stack.0, in_tag, &facts.get(slot).copied().unwrap_or_default())
                }))?;
                used[slot] = true;
            }
        }
        let mut crystal = ItemSlot::new(Identifier::new(CRYSTAL_ITEM.0, CRYSTAL_ITEM.1).ok()?, u32::try_from(*level).ok()?);
        crystal.custom_data.insert(CRYSTAL_KEY.into(), Nbt::String(modifier.to_string()));
        Some(crystal)
    }

    /// updateInputs' returned stack, when this recipe returns its input: the first enchantment
    /// (in the stack's order) mapping to the crystal's modifier loses one level and drops out
    /// at zero. A book left with no enchantments becomes a plain book, keeping a custom name
    /// the stack had; anything else keeps its item with the reduced enchantments. The result
    /// is a description — the window materializes the stack, name included, when it gives the
    /// item back. None when the recipe does not return its input.
    pub(crate) fn unenchanted_return(&self, tool: &Identifier, enchantments: &BTreeMap<String, i32>,
                                     map: &EnchantmentMap, crystal_modifier: &Identifier) -> Option<Returned> {
        if !self.return_unenchanted { return None; }
        let mut reduced = enchantments.clone();
        for (name, level) in enchantments {
            let Ok(enchantment) = Identifier::parse(name) else { continue };
            if map.modifier_for(&enchantment) == Some(crystal_modifier) {
                if *level <= 1 { reduced.remove(name); } else { reduced.insert(name.clone(), level - 1); }
                break;
            }
        }
        if self.match_book && reduced.is_empty() {
            Some(Returned { item: Identifier::new(BOOK.0, BOOK.1).ok()?, enchantments: BTreeMap::new() })
        } else {
            Some(Returned { item: tool.clone(), enchantments: reduced })
        }
    }
}

/// The stack a converting craft hands back, before it is materialized (see `unenchanted_return`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Returned {
    pub item: Identifier,
    pub enchantments: BTreeMap<String, i32>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Conversions {
    pub recipes: Vec<Recipe>,
    pub problems: Vec<String>,
    pub loaded: bool,
}

impl Conversions {
    pub fn load<C: Conditions>(files: &[(Identifier, Value)], conditions: &C) -> Self {
        let mut result = Self { loaded: true, ..Self::default() };
        let mut sorted: Vec<&(Identifier, Value)> = files.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (id, value) in sorted {
            let outcome = (|| -> Result<Option<Recipe>, String> {
                let fields = value.as_object().ok_or("recipe must be an object")?;
                if fields.get("type").and_then(Value::as_str) != Some("tconstruct:enchantment_converting") { return Ok(None); }
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

    /// The first recipe matching the tool and inputs, in id order.
    pub fn find(&self, tool: &Identifier, enchantable: bool, inputs: &[Option<(Identifier, u32)>],
                facts: &[StackFacts], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Option<&Recipe> {
        self.recipes.iter().find(|recipe| recipe.matches(tool, enchantable, inputs, facts, in_tag))
    }
}

/// Installs the reload listener, shaped like the worktable removals': a missing condition
/// context clears the recipes and fails the reload step; server teardown clears them too.
pub(crate) fn register<C: Conditions + 'static>(context: impl Fn() -> Result<C, String> + Send + Sync + 'static) -> Result<Arc<RwLock<Conversions>>, String> {
    let state = Arc::new(RwLock::new(Conversions::default()));
    let output = state.clone();
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "enchantment_converting").unwrap(), FOLDER, move |files| {
            let context = context();
            let mut state = output.write().map_err(|_| "enchantment converting recipes poisoned")?;
            match context {
                Ok(context) => {
                    *state = Conversions::load(files, &context);
                    for problem in &state.problems { eprintln!("Tinkers: {problem}"); }
                    Ok(())
                }
                Err(error) => { *state = Conversions { problems: vec![error.clone()], ..Conversions::default() }; Err(error) }
            }
        })?;
    let stopped = state.clone();
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| *stopped.write().expect("enchantment converting recipes poisoned") = Conversions::default());
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct Always;
    impl Conditions for Always {
        type Parsed = ();
        fn decode(&self, _: &Value) -> Result<(), String> { Ok(()) }
        fn test(&self, _: &()) -> Result<bool, String> { Ok(true) }
    }
    fn book_recipe() -> Recipe {
        let id = Identifier::parse("tconstruct:unenchant_book").unwrap();
        Recipe::decode(&id, &json!({"type":"tconstruct:enchantment_converting","name":"modifiers",
            "inputs":[{"item":"tconstruct:ender_slime_crystal"},{"amount_needed":5,"item":"minecraft:dragon_breath"}],
            "match_book":true,"return_unenchanted":true})).unwrap()
    }
    fn map() -> EnchantmentMap {
        let stack = pluto_lib::JsonResourceStack { id: Identifier::parse("tconstruct:enchantments_to_modifiers").unwrap(),
            layers: vec![Ok(json!({"minecraft:fortune": "tconstruct:luck", "minecraft:efficiency": "tconstruct:haste"}))] };
        EnchantmentMap::load(Some(&stack), &[], &|id| ["tconstruct:luck", "tconstruct:haste"].iter().any(|known| Identifier::parse(known).unwrap() == *id))
    }
    fn in_tag(_: &Identifier, _: &Identifier) -> bool { false }

    #[test]
    fn a_book_offers_its_enchantments_at_level_one_and_returns_disenchanted() {
        let recipe = book_recipe();
        let map = map();
        let enchantments: BTreeMap<String, i32> = [("minecraft:fortune".to_owned(), 3), ("minecraft:unbreaking".to_owned(), 2)].into_iter().collect();
        // Unbreaking maps to nothing here, so only fortune is offered, at level 1 (the input returns).
        assert_eq!(recipe.options(&enchantments, &map, &in_tag), vec![(Identifier::parse("tconstruct:luck").unwrap(), 1)]);
        let book = Identifier::parse("minecraft:enchanted_book").unwrap();
        let inputs = [Some((Identifier::parse("tconstruct:ender_slime_crystal").unwrap(), 1)), Some((Identifier::parse("minecraft:dragon_breath").unwrap(), 5))];
        assert!(recipe.matches(&book, false, &inputs, &[], &in_tag));
        let crystal = recipe.result(&(Identifier::parse("tconstruct:luck").unwrap(), 1), &inputs, &[], &in_tag).unwrap();
        assert_eq!((crystal.item.to_string().as_str(), crystal.count), ("tconstruct:modifier_crystal", 1));
        assert_eq!(crystal.custom_data.get("modifier"), Some(&Nbt::String("tconstruct:luck".into())));
        // One fortune level is converted: the book comes back at fortune 2.
        let returned = recipe.unenchanted_return(&book, &enchantments, &map, &Identifier::parse("tconstruct:luck").unwrap()).unwrap();
        assert_eq!(returned, Returned { item: book.clone(), enchantments: [("minecraft:fortune".to_owned(), 2), ("minecraft:unbreaking".to_owned(), 2)].into_iter().collect() });
        // Converting the last level of a book's only enchantment leaves a plain book.
        let single: BTreeMap<String, i32> = [("minecraft:efficiency".to_owned(), 1)].into_iter().collect();
        let returned = recipe.unenchanted_return(&book, &single, &map, &Identifier::parse("tconstruct:haste").unwrap()).unwrap();
        assert_eq!(returned.item, Identifier::parse("minecraft:book").unwrap());
        assert!(returned.enchantments.is_empty());
    }

    #[test]
    fn a_level_above_one_needs_every_input_at_amount_times_level_in_its_own_slot() {
        let id = Identifier::parse("tconstruct:unenchant_tool").unwrap();
        let recipe = Recipe::decode(&id, &json!({"type":"tconstruct:enchantment_converting","name":"modifiers",
            "inputs":[{"item":"tconstruct:ender_slime_crystal"},{"amount_needed":5,"item":"minecraft:dragon_breath"}],
            "match_book":false,"return_unenchanted":false})).unwrap();
        let selected = (Identifier::parse("tconstruct:haste").unwrap(), 3);
        // 15 dragon breath in one slot and the crystal: enough.
        let enough = [Some((Identifier::parse("minecraft:dragon_breath").unwrap(), 15)), Some((Identifier::parse("tconstruct:ender_slime_crystal").unwrap(), 3))];
        assert_eq!(recipe.result(&selected, &enough, &[], &in_tag).map(|crystal| crystal.count), Some(3));
        // 14 is short of 5 × 3, and splitting one ingredient over two slots does not count.
        let short = [Some((Identifier::parse("minecraft:dragon_breath").unwrap(), 14)), Some((Identifier::parse("tconstruct:ender_slime_crystal").unwrap(), 3))];
        assert!(recipe.result(&selected, &short, &[], &in_tag).is_none());
        let split = [Some((Identifier::parse("minecraft:dragon_breath").unwrap(), 10)), Some((Identifier::parse("minecraft:dragon_breath").unwrap(), 5)), Some((Identifier::parse("tconstruct:ender_slime_crystal").unwrap(), 3))];
        assert!(recipe.result(&selected, &split, &[], &in_tag).is_none(), "no single slot holds 15");
    }
}
