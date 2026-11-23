//! Repairing a tool at the tinker station through a modifier it carries,
//! the original's `ModifierRepairTinkerStationRecipe` (SlimeKnights, MIT;
//! see LICENSE): a damaged or broken durability tool with the recipe's
//! modifier, plus only the recipe's ingredient on the table, is repaired
//! by the ingredient count times the per-item amount (the recipe's
//! `repair_amount` scaled by the modifier level and the tool's
//! REPAIR_FACTOR chain). The one shipped recipe is `tools/modifiers/tasty`
//! (bacon repairs a tasty tool by 25 per level). The crafting-grid form of
//! the same rule lives in crafting_modifier_repair.rs.
use crate::{
    material_recipes::Ingredient,
    materials::Conditions,
    tool_slot,
    tool_stack::{self, Environment},
};
use pluto_lib::{Identifier, ItemSlot, ResourceManagerHelper, ResourceType, ServerLifecycleEvents};
use serde_json::Value;
use std::sync::{Arc, RwLock};

pub(crate) const FOLDER: &str = crate::modifier_recipes::FOLDER;
/// `TinkerTags.Items.DURABILITY`.
const DURABILITY_TAG: &str = "tconstruct:modifiable/durability";

/// `IncrementalModifierRecipe.containsOnlyIngredient`: every staged stack
/// matches the ingredient and at least one is staged; returns the total
/// item count (`getAvailableAmount`'s basis).
pub(crate) fn contains_only_ingredient(
    raw: &[Option<(Identifier, u32)>],
    ingredient: &Ingredient,
    in_tag: &dyn Fn(&Identifier, &Identifier) -> bool,
) -> Option<i32> {
    let stacks: Vec<&(Identifier, u32)> = raw.iter().flatten().collect();
    if stacks.is_empty() || !stacks.iter().all(|stack| ingredient.matches(&stack.0, in_tag)) {
        return None;
    }
    Some(stacks.iter().fold(0i32, |sum, stack| sum.wrapping_add(stack.1 as i32)))
}

/// `IncrementalModifierRecipe.updateInputs` with no leftover item: take
/// `items` items from the ingredient's slots, in slot order.
pub(crate) fn consume_ingredient(
    raw: &[Option<(Identifier, u32)>],
    ingredient: &Ingredient,
    in_tag: &dyn Fn(&Identifier, &Identifier) -> bool,
    items: i32,
) -> Vec<u32> {
    let mut remaining = items.max(0) as u32;
    let mut consumed = vec![0u32; raw.len()];
    for (slot, stack) in raw.iter().enumerate() {
        let Some((item, count)) = stack else { continue };
        if !ingredient.matches(item, in_tag) {
            continue;
        }
        let take = (*count).min(remaining);
        consumed[slot] = take;
        remaining -= take;
    }
    consumed
}

/// Whole items needed to cover `amount` at `per_item` each, rounding up
/// (the original consumes item by item until the amount is covered).
pub(crate) fn items_to_cover(amount: i32, per_item: i32) -> i32 {
    if amount <= 0 || per_item <= 0 {
        return 0;
    }
    amount / per_item + i32::from(amount % per_item > 0)
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Recipe {
    pub id: Identifier,
    pub modifier: Identifier,
    pub ingredient: Ingredient,
    pub repair_amount: i32,
}

impl Recipe {
    fn decode(id: &Identifier, value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("recipe must be an object")?;
        let modifier = Identifier::parse(
            fields.get("modifier").and_then(Value::as_str).ok_or("modifier must be an ID string")?,
        )
        .map_err(|e| e.to_string())?;
        let repair_amount = match fields.get("repair_amount") {
            Some(Value::Number(n)) => {
                let n = crate::materials::number_int(&n.to_string());
                if n >= 1 {
                    n
                } else {
                    return Err("repair_amount must be at least 1".into());
                }
            }
            _ => return Err("repair_amount must be a number".into()),
        };
        Ok(Self {
            id: id.clone(),
            modifier,
            ingredient: Ingredient::decode(fields.get("ingredient").ok_or("ingredient missing")?)?,
            repair_amount,
        })
    }

    /// matches + getValidatedResult + updateInputs in one evaluation, as
    /// the station needs all three from the same facts. None is the
    /// original's pass (no match, or a repair factor of zero).
    pub fn attempt(
        &self,
        tool: &ItemSlot,
        env: &Environment,
        raw: &[Option<(Identifier, u32)>],
        in_tag: &dyn Fn(&Identifier, &Identifier) -> bool,
    ) -> Option<Result<(ItemSlot, Vec<u32>), String>> {
        let durability = Identifier::parse(DURABILITY_TAG).ok()?;
        if !in_tag(&tool.item, &durability) {
            return None;
        }
        let data = &tool.custom_data;
        let level = tool_stack::saved_modifiers(data, env.modifiers)
            .iter()
            .filter(|(entry, _)| entry.id == self.modifier)
            .fold(0, |sum, (entry, _)| sum.wrapping_add(entry.level));
        if level <= 0 {
            return None;
        }
        if !tool_stack::is_broken(data) && tool_stack::damage(data) == 0 {
            return None;
        }
        let total = contains_only_ingredient(raw, &self.ingredient, in_tag)?;
        // RepairFactorModifierHook.getRepairFactor(tool, 1): the chain's
        // own multiplier, before it scales the per-item amount.
        let factor = tool_stack::repair_factor(data, env, 1.0);
        if factor <= 0.0 {
            return None;
        }
        let per_item = ((level * self.repair_amount) as f32 * factor) as i32;
        if per_item <= 0 {
            return None;
        }
        let available = total.wrapping_mul(per_item);
        if available <= 0 {
            return None;
        }
        let mut result = tool.clone();
        // The original's result is the tool at count 1 (shrinkToolSlotBy 1).
        result.count = 1;
        if let Err(error) = tool_slot::repair_slot(&mut result, env, available) {
            return Some(Err(error));
        }
        let repaired = tool_stack::damage(data) - tool_stack::damage(&result.custom_data);
        let consumed = consume_ingredient(raw, &self.ingredient, in_tag, items_to_cover(repaired, per_item));
        Some(Ok((result, consumed)))
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Recipes {
    recipes: Vec<Recipe>,
    pub problems: Vec<String>,
    pub loaded: bool,
}

impl Recipes {
    pub fn load<C: Conditions>(files: &[(Identifier, Value)], conditions: &C) -> Self {
        let mut result = Self { loaded: true, ..Self::default() };
        let mut sorted: Vec<&(Identifier, Value)> = files.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (id, value) in sorted {
            let outcome = (|| -> Result<Option<Recipe>, String> {
                let fields = value.as_object().ok_or("recipe must be an object")?;
                if fields.get("type").and_then(Value::as_str) != Some("tconstruct:modifier_repair") {
                    return Ok(None);
                }
                if let Some(list) = fields.get("conditions") {
                    let parsed = list
                        .as_array()
                        .ok_or("conditions must be an array")?
                        .iter()
                        .map(|c| conditions.decode(c))
                        .collect::<Result<Vec<_>, _>>()?;
                    for condition in &parsed {
                        if !conditions.test(condition)? {
                            return Ok(None);
                        }
                    }
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
    pub fn len(&self) -> usize {
        self.recipes.len()
    }
    /// The first recipe (in id order) that repairs this tool, if any.
    pub fn attempt(
        &self,
        tool: &ItemSlot,
        env: &Environment,
        raw: &[Option<(Identifier, u32)>],
        in_tag: &dyn Fn(&Identifier, &Identifier) -> bool,
    ) -> Option<Result<(ItemSlot, Vec<u32>), String>> {
        self.recipes.iter().find_map(|recipe| recipe.attempt(tool, env, raw, in_tag))
    }
}

/// Reload listener, shaped like the other recipe listeners.
pub(crate) fn register<C: Conditions + 'static>(
    context: impl Fn() -> Result<C, String> + Send + Sync + 'static,
) -> Result<Arc<RwLock<Recipes>>, String> {
    let state = Arc::new(RwLock::new(Recipes::default()));
    let output = state.clone();
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "modifier_repair_recipes").unwrap(),
        FOLDER,
        move |files| {
            let context = context();
            let mut state = output.write().map_err(|_| "modifier repair recipes poisoned")?;
            match context {
                Ok(context) => {
                    *state = Recipes::load(files, &context);
                    for problem in &state.problems {
                        eprintln!("Tinkers: {problem}");
                    }
                    Ok(())
                }
                Err(error) => {
                    *state = Recipes { problems: vec![error.clone()], ..Recipes::default() };
                    Err(error)
                }
            }
        },
    )?;
    let stopped = state.clone();
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| {
        *stopped.write().expect("modifier repair recipes poisoned") = Recipes::default()
    });
    Ok(state)
}
