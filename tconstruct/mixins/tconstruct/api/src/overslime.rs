//! Overslime: the modifier's definition, its persistent damage shield and the slime-ball recipe that
//! fills it, from SlimeKnights OverslimeModifier, OverslimeModule, DurabilityShieldModule and
//! OverslimeModifierRecipe (MIT; see LICENSE). The durability bar color and the incremental name are
//! presentation work that is not ported yet.
use crate::{
    material_recipes::Ingredient,
    materials::Conditions,
    modifier_entries::List,
    modifiers::Modifier,
    tool_slot,
    tool_stack::{rebuild_stats, saved_modifiers, stat, Environment, TAG_PERSISTENT, TAG_UPGRADES},
    tool_stats::Stat,
};
use pluto_lib::{Identifier, ItemSlot, Nbt, NbtCompound, ResourceManagerHelper, ResourceType, ServerLifecycleEvents};
use serde_json::{json, Value};
use std::sync::{Arc, RwLock};

pub(crate) fn id() -> Identifier { Identifier::new("tconstruct", "overslime").expect("valid id") }
fn key() -> String { id().to_string() }
fn overworked() -> Identifier { Identifier::new("tconstruct", "overworked").expect("valid id") }

/// The overslime modifier (OverslimeModifier): priority 150, +50 overslime capacity and the stat
/// penalties of a slimy tool unless a modifier in the overslime_friend tag is present. The shield
/// itself is applied by `absorb`, not by a module.
pub(crate) fn modifier() -> Result<Modifier, String> {
    let unfriendly = |tag: &str| json!({"type":"mantle:and","predicates":[
        {"type":"mantle:tag","tag":tag},
        {"type":"mantle:inverted","inverted_type":"tconstruct:has_modifier","check":"all","modifier":{"type":"tconstruct:tag","tag":"tconstruct:overslime_friend"}}]});
    let penalty = |stat: &str, operation: &str, flat: f64, tag: &str| json!({"type":"tconstruct:stat_boost","stat":format!("tconstruct:{stat}"),"operation":operation,"flat":flat,"tool":unfriendly(tag)});
    Modifier::decode(id(), &json!({"priority":150,"level_display":"tconstruct:no_levels","modules":[
        {"type":"tconstruct:stat_boost","stat":"tconstruct:overslime","operation":"add","flat":50.0},
        penalty("attack_damage", "multiply_base", -0.1, "tconstruct:modifiable/melee"),
        penalty("mining_speed", "multiply_base", -0.1, "tconstruct:modifiable/harvest"),
        penalty("armor", "add", -0.5, "tconstruct:modifiable/armor"),
        penalty("velocity", "multiply_base", -0.1, "tconstruct:modifiable/ranged"),
    ]}))
}

/// The code-defined modifiers registered with overslime: overworked is a plain modifier whose level
/// only scales overslime restoration (see `overworked_bonus`).
pub(crate) fn code_modifiers() -> Result<Vec<Modifier>, String> {
    let creative = Modifier::decode(Identifier::new("tconstruct", "creative_slot").expect("valid id"),
        &json!({"level_display":"tconstruct:no_levels","modules":[{"type":"tconstruct:creative_slots","hooks":["tconstruct:volatile_data","tconstruct:remove"]}]}))?;
    let stat_override = Modifier::decode(Identifier::new("tconstruct", "stat_override").expect("valid id"),
        &json!({"level_display":"tconstruct:no_levels","show_in_tooltips":"advanced","modules":[{"type":"tconstruct:stat_override","hooks":["tconstruct:tool_stats","tconstruct:remove"]}]}))?;
    Ok(vec![modifier()?, Modifier::decode(overworked(), &json!({}))?, creative, stat_override])
}

/// PersistentDataCapacityBar.getAmount.
pub(crate) fn amount(nbt: &NbtCompound) -> i32 {
    match nbt.get(TAG_PERSISTENT) { Some(Nbt::Compound(data)) => crate::modifier_entries::nbt_int(data.get(&key())), _ => 0 }
}

/// OverslimeModule.getCapacity: the overslime stat as an int.
pub(crate) fn capacity(nbt: &NbtCompound) -> i32 { stat(nbt, Stat::Overslime) as i32 }

/// PersistentDataCapacityBar.setAmount: zero or less removes the key; otherwise capped at capacity.
pub(crate) fn set_amount(nbt: &mut NbtCompound, amount: i32) {
    let capacity = capacity(nbt);
    let mut persistent = match nbt.get(TAG_PERSISTENT) { Some(Nbt::Compound(data)) => data.clone(), _ => NbtCompound::new() };
    if amount <= 0 { persistent.remove(&key()); } else { persistent.insert(key(), Nbt::Int(amount.min(capacity))); }
    nbt.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
}

/// OverslimeModule.getOverworkedBonus: 1 plus the overworked level.
pub(crate) fn overworked_bonus(nbt: &NbtCompound, env: &Environment) -> i32 {
    let level: i32 = saved_modifiers(nbt, env.modifiers).iter().filter(|(entry, _)| entry.id == overworked()).fold(0, |sum, (entry, _)| sum.wrapping_add(entry.level));
    level.wrapping_add(1)
}

/// OverslimeModule.addAmount: the amount scaled by overworked, capped at capacity.
pub(crate) fn add_amount(nbt: &mut NbtCompound, env: &Environment, amount: i32) {
    let scaled = amount.wrapping_mul(overworked_bonus(nbt, env));
    if scaled != 0 { let current = self::amount(nbt); set_amount(nbt, current.wrapping_add(scaled)); }
}

/// The shield part of tool damage (DurabilityShieldModule.onDamageTool with cost 1): overslime soaks
/// damage first, unless the damaging modifier bypasses it. Returns the damage left over.
pub(crate) fn absorb(nbt: &mut NbtCompound, env: &Environment, cause: Option<&Identifier>, damage: i32) -> i32 {
    if damage <= 0 { return damage; }
    let present = saved_modifiers(nbt, env.modifiers).iter().any(|(entry, _)| entry.id == id());
    let shield = amount(nbt);
    if !present || shield <= 0 { return damage; }
    let bypass = Identifier::new("tconstruct", "durability/bypass_overslime").expect("valid tag");
    if cause.is_some_and(|cause| env.modifiers.reverse_tags.get(cause).is_some_and(|tags| tags.contains(&bypass))) { return damage; }
    if shield >= damage { set_amount(nbt, shield - damage); 0 } else { set_amount(nbt, 0); damage - shield }
}

/// `tconstruct:overslime_modifier`: slimes restore overslime on any tool that has durability.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Recipe { pub id: Identifier, pub tools: Ingredient, pub ingredient: Ingredient, pub restore: i32 }

impl Recipe {
    fn decode(id: &Identifier, value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("recipe must be an object")?;
        // The default tool requirement is anything with durability.
        let tools = match fields.get("tools") {
            None => Ingredient::Tag(Identifier::new("tconstruct", "modifiable/durability").expect("valid tag")),
            Some(value) => Ingredient::decode(value)?,
        };
        let restore = match fields.get("restore_amount") {
            Some(Value::Number(n)) => { let n = crate::materials::number_int(&n.to_string()); if n >= 1 { n } else { return Err("restore_amount must be at least 1".into()); } }
            _ => return Err("restore_amount must be a number".into()),
        };
        Ok(Self { id: id.clone(), tools, ingredient: Ingredient::decode(fields.get("ingredient").ok_or("ingredient missing")?)?, restore })
    }
    /// matches: the tool qualifies and the table holds only this ingredient (at least one).
    pub fn matches(&self, tool: &Identifier, inputs: &[Option<(Identifier, u32)>], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        let stacks: Vec<&(Identifier, u32)> = inputs.iter().flatten().collect();
        self.tools.matches(tool, in_tag) && !stacks.is_empty() && stacks.iter().all(|stack| self.ingredient.matches(&stack.0, in_tag))
    }
    /// getValidatedResult: the tool with overslime added (when absent) and filled as far as the inputs reach.
    pub fn apply(&self, tool: &ItemSlot, env: &Environment, inputs: &[Option<(Identifier, u32)>], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Result<Result<ItemSlot, String>, String> {
        const AT_CAPACITY: &str = "recipe.tconstruct.overslime.at_capacity";
        let upgrades = List::from_nbt(tool.custom_data.get(TAG_UPGRADES));
        let mut copy = tool.clone();
        if upgrades.level(&id()) == 0 {
            // Overslime from a trait that has already reached its cap cannot take more.
            let from_traits = saved_modifiers(&tool.custom_data, env.modifiers).iter().any(|(entry, _)| entry.id == id());
            if from_traits && amount(&tool.custom_data) >= capacity(&tool.custom_data) { return Ok(Err(AT_CAPACITY.into())); }
            copy.custom_data.insert(TAG_UPGRADES.into(), upgrades.with_modifier(id(), 1)?.to_nbt());
            if !rebuild_stats(&mut copy.custom_data, env)? { return Err("the tool could not be rebuilt".into()); }
        } else if amount(&tool.custom_data) >= capacity(&tool.custom_data) {
            return Ok(Err(AT_CAPACITY.into()));
        }
        let available = inputs.iter().flatten().filter(|stack| self.ingredient.matches(&stack.0, in_tag))
            .fold(0i32, |sum, stack| sum.wrapping_add((stack.1 as i32).wrapping_mul(self.restore)));
        add_amount(&mut copy.custom_data, env, available);
        tool_slot::sync_visuals(&mut copy, env)?;
        copy.count = tool.count.min(16);
        Ok(Ok(copy))
    }
    /// updateInputs: items to take from each slot, from the overslime actually gained.
    pub fn consumption(&self, before: &ItemSlot, after: &ItemSlot, env: &Environment, inputs: &[Option<(Identifier, u32)>], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Vec<u32> {
        let needed = (amount(&after.custom_data) - amount(&before.custom_data)).max(0);
        let per_item = (self.restore.wrapping_mul(overworked_bonus(&before.custom_data, env))).max(1);
        let mut remaining = (needed / per_item + i32::from(needed % per_item > 0)) as u32;
        let mut consumed = vec![0u32; inputs.len()];
        for (slot, stack) in inputs.iter().enumerate() {
            let Some((item, count)) = stack else { continue };
            if !self.ingredient.matches(item, in_tag) { continue; }
            let take = (*count).min(remaining);
            consumed[slot] = take;
            remaining -= take;
        }
        consumed
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
                if fields.get("type").and_then(Value::as_str) != Some("tconstruct:overslime_modifier") { return Ok(None); }
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
    pub fn len(&self) -> usize { self.recipes.len() }
    pub fn find(&self, tool: &Identifier, inputs: &[Option<(Identifier, u32)>], in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Option<&Recipe> {
        self.recipes.iter().find(|recipe| recipe.matches(tool, inputs, in_tag))
    }
}

/// Reload listener, shaped like the other recipe listeners.
pub(crate) fn register<C: Conditions + 'static>(context: impl Fn() -> Result<C, String> + Send + Sync + 'static) -> Result<Arc<RwLock<Recipes>>, String> {
    let state = Arc::new(RwLock::new(Recipes::default()));
    let output = state.clone();
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "overslime_recipes").unwrap(), crate::modifier_recipes::FOLDER, move |files| {
            let context = context();
            let mut state = output.write().map_err(|_| "overslime recipes poisoned")?;
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
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| *stopped.write().expect("overslime recipes poisoned") = Recipes::default());
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Flags;
    impl Conditions for Flags {
        type Parsed = bool;
        fn decode(&self, value: &Value) -> Result<bool, String> { value.as_bool().ok_or("condition must be boolean".into()) }
        fn test(&self, value: &bool) -> Result<bool, String> { Ok(*value) }
    }
    fn item(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    #[test]
    fn the_modifier_decodes_and_the_recipe_matches_only_slimes() {
        assert!(modifier().is_ok());
        let files = vec![(item("test:slime"), json!({"type":"tconstruct:overslime_modifier","ingredient":{"item":"minecraft:slime_ball"},"restore_amount":20})),
            (item("test:bad"), json!({"type":"tconstruct:overslime_modifier","ingredient":{"item":"a:b"},"restore_amount":0}))];
        let recipes = Recipes::load(&files, &Flags);
        assert_eq!((recipes.len(), recipes.problems.len()), (1, 1), "{:?}", recipes.problems);
        let in_tag = |item: &Identifier, tag: &Identifier| tag.path() == "modifiable/durability" && item.path() == "pickaxe";
        let slime = Some((item("minecraft:slime_ball"), 3));
        assert!(recipes.find(&item("tconstruct:pickaxe"), &[slime.clone(), None], &in_tag).is_some());
        assert!(recipes.find(&item("minecraft:stick"), &[slime.clone()], &in_tag).is_none(), "the tool needs durability");
        assert!(recipes.find(&item("tconstruct:pickaxe"), &[slime, Some((item("minecraft:dirt"), 1))], &in_tag).is_none());
        assert!(recipes.find(&item("tconstruct:pickaxe"), &[None, None], &in_tag).is_none());
    }
}
