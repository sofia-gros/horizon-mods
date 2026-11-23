//! Damaging a tool at the tinker station, the original's
//! `TinkerStationDamagingRecipe` (SlimeKnights, MIT; see LICENSE): a
//! durability tool plus only the recipe's ingredient on the table takes
//! direct damage — `damage_amount` per ingredient item, bypassing the
//! damage hooks as `ToolDamageUtil.directDamage` does — and the table
//! spends only the items the damage actually taken accounts for. The
//! shipped recipes are the `tables/tinker_station_damaging` files (lava
//! and blazing blood buckets, magma and venom bottles and buckets);
//! modifiers that feed on tool damage (for example blazing) are applied
//! through the ordinary modifier recipes on the damaged tool.
use crate::{
    material_recipes::Ingredient,
    materials::Conditions,
    modifier_repair::{consume_ingredient, contains_only_ingredient, items_to_cover},
    tool_slot,
    tool_stack::{self, Environment},
};
use pluto_lib::{Identifier, ItemSlot, ResourceManagerHelper, ResourceType, ServerLifecycleEvents};
use serde_json::Value;
use std::sync::{Arc, RwLock};

pub(crate) const FOLDER: &str = "recipes/tables";
/// `TinkerTags.Items.DURABILITY`.
const DURABILITY_TAG: &str = "tconstruct:modifiable/durability";
/// The failure for damaging an already broken tool.
const BROKEN: &str = "recipe.tconstruct.damaging.broken";

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Recipe {
    pub id: Identifier,
    pub ingredient: Ingredient,
    pub damage_amount: i32,
}

impl Recipe {
    fn decode(id: &Identifier, value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("recipe must be an object")?;
        let damage_amount = match fields.get("damage_amount") {
            Some(Value::Number(n)) => {
                let n = crate::materials::number_int(&n.to_string());
                if n >= 1 {
                    n
                } else {
                    return Err("damage_amount must be at least 1".into());
                }
            }
            _ => return Err("damage_amount must be a number".into()),
        };
        Ok(Self {
            id: id.clone(),
            ingredient: Ingredient::decode(fields.get("ingredient").ok_or("ingredient missing")?)?,
            damage_amount,
        })
    }

    /// matches + getValidatedResult + updateInputs in one evaluation.
    /// None is no match; a broken tool is the original's failure result.
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
        let total = contains_only_ingredient(raw, &self.ingredient, in_tag)?;
        if tool_stack::is_broken(&tool.custom_data) {
            return Some(Err(BROKEN.to_owned()));
        }
        let mut result = tool.clone();
        // The original's result is the tool at count 1 (shrinkToolSlotBy 1).
        result.count = 1;
        let available = total.wrapping_mul(self.damage_amount);
        if let Err(error) = tool_slot::direct_damage_slot(&mut result, env, available, true) {
            return Some(Err(error));
        }
        let taken = tool_stack::damage(&result.custom_data) - tool_stack::damage(&tool.custom_data);
        let consumed = consume_ingredient(raw, &self.ingredient, in_tag, items_to_cover(taken, self.damage_amount));
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
                if fields.get("type").and_then(Value::as_str) != Some("tconstruct:tinker_station_damaging") {
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
    /// The first recipe (in id order) that damages this tool, if any.
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
        Identifier::new("tconstruct", "damaging_recipes").unwrap(),
        FOLDER,
        move |files| {
            let context = context();
            let mut state = output.write().map_err(|_| "damaging recipes poisoned")?;
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
        *stopped.write().expect("damaging recipes poisoned") = Recipes::default()
    });
    Ok(state)
}
