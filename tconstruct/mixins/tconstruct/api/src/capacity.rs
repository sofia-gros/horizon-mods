//! CapacityBarModule and DurabilityShieldModule, from SlimeKnights (MIT; see LICENSE): a data-defined bar whose amount lives in the
//! tool's persistent data under the modifier's ID, with a capacity of `flat + each_level * level` scaled by a stat multiplier, and the
//! shield that soaks tool wear from it before any other reduction (barkskin, frostshield, stoneshield). The durability bar color and the
//! incremental name are presentation work that is not ported. The sources that fill a bar are ported where PlutoLib reports the event with
//! a stack it can edit: damage to capacity (armor taking damage), melee capacity (a weapon hit), mining capacity (a block the tool broke,
//! before and after), loot to capacity (the loot of a block the tool broke) and launch capacity (an arrow a player's bow or crossbow shot:
//! the shot event carries both hands' stacks and takes the edit). A sling's or a thrown projectile's launch is not reported.
//! The edits land as `EntityAction::EditStackData` (see `persistent_edits`).
use crate::{
    modifier_entries::Entry,
    melee_damage::TargetCondition,
    modifier_modules::CausePredicate,
    modifiers::Modifier,
    protection::{DamageSource, SourceCondition},
    thorns::Leveling,
    material_recipes::Ingredient,
    overslime,
    tool_stack::{saved_modifiers, Environment, TAG_MULTIPLIERS, TAG_PERSISTENT},
    tool_stats::Stat,
};
use pluto_lib::{Identifier, ItemSlot, Nbt, NbtCompound, StackDataEdit};
use serde_json::{Map, Value};

/// A LevelingInt: whole numbers within a short, `flat + each_level * level` truncated to an int.
#[derive(Clone, Copy, Debug, PartialEq)]
struct LevelingInt { flat: i32, each: i32 }

impl LevelingInt {
    fn decode(fields: &Map<String, Value>, default_flat: i32) -> Result<Self, String> {
        let short = |key: &str, default: i32| match fields.get(key).filter(|value| !value.is_null()) {
            None => Ok(default),
            Some(Value::Number(number)) => {
                let value = crate::materials::number_int(&number.to_string());
                if (-32768..=32767).contains(&value) { Ok(value) } else { Err(format!("{key} must fit in a short")) }
            }
            Some(_) => Err(format!("{key} must be a number")),
        };
        Ok(Self { flat: short("flat", default_flat)?, each: short("each_level", 0)? })
    }
    fn compute(self, level: f32) -> i32 { (self.flat as f32 + self.each as f32 * level) as i32 }
}

/// The persistent key of a bar: its modifier's ID.
fn key(modifier: &Identifier) -> String { modifier.to_string() }

/// PersistentDataCapacityBar.getAmount.
pub(crate) fn amount(nbt: &NbtCompound, modifier: &Identifier) -> i32 {
    match nbt.get(TAG_PERSISTENT) { Some(Nbt::Compound(data)) => crate::modifier_entries::nbt_int(data.get(&key(modifier))), _ => 0 }
}

/// PersistentDataCapacityBar.setAmount: zero or less removes the key; otherwise capped at `capacity`.
pub(crate) fn set_amount(nbt: &mut NbtCompound, modifier: &Identifier, capacity: i32, amount: i32) {
    let mut persistent = match nbt.get(TAG_PERSISTENT) { Some(Nbt::Compound(data)) => data.clone(), _ => NbtCompound::new() };
    if amount <= 0 { persistent.remove(&key(modifier)); } else { persistent.insert(key(modifier), Nbt::Int(amount.min(capacity))); }
    nbt.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
}

/// tool.getMultiplier: the stat's multiplier, 1 when the tool has none.
fn multiplier(nbt: &NbtCompound, stat: Stat) -> f32 {
    match nbt.get(TAG_MULTIPLIERS) {
        Some(Nbt::Compound(data)) => match data.get(&format!("tconstruct:{}", stat.path())) {
            Some(Nbt::Float(value)) => *value,
            Some(other) => crate::modifier_entries::nbt_int(Some(other)) as f32,
            None => 1.0,
        },
        _ => 1.0,
    }
}

/// `tconstruct:capacity_bar`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Bar { capacity: LevelingInt, multiplier: Option<Stat> }

impl Bar {
    /// `None` when the multiplier names a stat this port does not hold.
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        let multiplier = match fields.get("multiplier").filter(|value| !value.is_null()) {
            None => None,
            Some(Value::String(text)) => match Stat::from_id(&Identifier::parse(text).map_err(|e| e.to_string())?) { Some(stat) => Some(stat), None => return Ok(None) },
            Some(_) => return Err("multiplier must be a stat ID".into()),
        };
        Ok(Some(Self { capacity: LevelingInt::decode(fields, 0)?, multiplier }))
    }

    /// CapacityBarModule.getCapacity: the level's capacity, scaled by the multiplier stat.
    pub(crate) fn capacity(&self, nbt: &NbtCompound, entry: &Entry) -> i32 {
        let capacity = self.capacity.compute(entry.effective_level());
        match self.multiplier { Some(stat) => (capacity as f32 * multiplier(nbt, stat)) as i32, None => capacity }
    }
}

/// `tconstruct:durability_shield`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Shield { cost: LevelingInt, cause: CausePredicate }

impl Shield {
    /// `None` when the cause predicate is a kind this port does not hold.
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        match fields.get("color") {
            Some(Value::String(text)) if text.len() == 6 && text.bytes().all(|byte| byte.is_ascii_hexdigit()) => {}
            _ => return Err("color must be six hexadecimal digits".into()),
        }
        // The default cause is anything outside the tag of modifiers that bypass extra durability bars.
        let cause = match fields.get("cause").filter(|value| !value.is_null()) {
            None => CausePredicate::Not(Box::new(CausePredicate::Tag(Identifier::new("tconstruct", "durability/bypass_extra_bar").expect("valid tag")))),
            Some(value) => match CausePredicate::decode(Some(value))? { Some(cause) => cause, None => return Ok(None) },
        };
        let cost = match fields.get("cost").filter(|value| !value.is_null()) {
            None => LevelingInt { flat: 1, each: 0 },
            Some(Value::Object(cost)) => LevelingInt::decode(cost, 0)?,
            Some(_) => return Err("cost must be an object".into()),
        };
        Ok(Some(Self { cost, cause }))
    }

    /// DurabilityShieldModule.beforeDamageTool for a bar holding `shield` of `capacity`: the damage left once the bar has soaked what it can.
    /// With a cost above one the bar's remainder is a chance to soak one more point (`random` is uniform in [0, 1)).
    fn soak(&self, nbt: &mut NbtCompound, modifier: &Entry, capacity: i32, mut damage: i32, random: &mut dyn FnMut() -> f32) -> i32 {
        let cost = self.cost.compute(modifier.effective_level());
        let shield = amount(nbt, &modifier.id);
        if shield <= 0 || cost <= 0 { return damage; }
        let can_reduce = shield / cost;
        if can_reduce >= damage {
            set_amount(nbt, &modifier.id, capacity, shield.wrapping_sub(damage.wrapping_mul(cost)));
            return 0;
        }
        damage -= can_reduce;
        if cost > 1 {
            let chance = shield % cost;
            if chance > 0 && chance > (random() * cost as f32) as i32 { damage -= 1; }
        }
        set_amount(nbt, &modifier.id, capacity, 0);
        damage
    }
}

/// The bar a capacity source fills: the overslime bar or the capacity bar of a modifier on the tool (CapacitySourceModule.barModifier).
pub(crate) enum Target<'a> { Overslime, Bar { entry: &'a Entry, bar: &'a Bar } }

impl<'a> Target<'a> {
    /// The bar of `owner` (the source's own modifier `own` when none); None when that modifier is not on the tool or holds no bar. The
    /// original would also fill the bar of an absent owner at level 0; only bars of modifiers on the tool are filled here.
    pub(crate) fn find(saved: &'a [(Entry, std::sync::Arc<Modifier>)], own: &'a Entry, owner: Option<&Identifier>) -> Option<Self> {
        let id = owner.unwrap_or(&own.id);
        if *id == overslime::id() { return Some(Self::Overslime); }
        saved.iter().find(|(entry, _)| entry.id == *id).and_then(|(entry, modifier)| modifier.capacity_bar().map(|bar| Self::Bar { entry, bar }))
    }
    fn capacity(&self, nbt: &NbtCompound) -> i32 { match self { Self::Overslime => overslime::capacity(nbt), Self::Bar { entry, bar } => bar.capacity(nbt, entry) } }
    fn amount(&self, nbt: &NbtCompound) -> i32 { match self { Self::Overslime => overslime::amount(nbt), Self::Bar { entry, .. } => amount(nbt, &entry.id) } }
    /// BarHook.setAmount: zero or less empties the bar, otherwise the amount is capped at the capacity.
    fn set(&self, nbt: &mut NbtCompound, value: i32) {
        match self { Self::Overslime => overslime::set_amount(nbt, value), Self::Bar { entry, bar } => { let capacity = bar.capacity(nbt, entry); set_amount(nbt, &entry.id, capacity, value) } }
    }
    /// CapacitySourceModule.apply with an amount of 1: a positive grant adds it, a negative one removes it and 0 empties the bar.
    pub(crate) fn grant(&self, nbt: &mut NbtCompound, env: &Environment, grant: i32) {
        match (self, grant) {
            (_, 0) => self.set(nbt, 0),
            (Self::Overslime, grant) if grant > 0 => overslime::add_amount(nbt, env, grant),
            (_, grant) => { let current = self.amount(nbt); self.set(nbt, current.wrapping_add(grant)) }
        }
    }
}

/// `tconstruct:melee_capacity`: adds to a bar whenever the tool deals melee damage. `run_before` only orders the grant against the hit's own
/// damage modules, which run in one pass here.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MeleeFill { target: TargetCondition, attacker: TargetCondition, grant: LevelingInt, owner: Option<Identifier> }

fn owner_of(fields: &Map<String, Value>) -> Result<Option<Identifier>, String> {
    match fields.get("owner").filter(|value| !value.is_null()) {
        None => Ok(None),
        Some(Value::String(text)) => Identifier::parse(text).map(Some).map_err(|e| e.to_string()),
        Some(_) => Err("owner must be a modifier ID".into()),
    }
}

impl MeleeFill {
    /// `None` when a condition names state this port does not hold.
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        let Some(target) = TargetCondition::decode_for("target", fields.get("target"))? else { return Ok(None); };
        let Some(attacker) = TargetCondition::decode_for("attacker", fields.get("attacker"))? else { return Ok(None); };
        let grant = match fields.get("grant").filter(|value| !value.is_null()) {
            None => LevelingInt { flat: 0, each: 0 },
            Some(Value::Object(grant)) => LevelingInt::decode(grant, 0)?,
            Some(_) => return Err("grant must be an object".into()),
        };
        Ok(Some(Self { target, attacker, grant, owner: owner_of(fields)? }))
    }

    /// MeleeCapacityModule.apply: the bar's owner and the grant when the hit's target and attacker pass their conditions.
    pub(crate) fn grant_for(&self, entry: &Entry, target_in_tag: &dyn Fn(&Identifier) -> bool, attacker_in_tag: &dyn Fn(&Identifier) -> bool) -> Option<(Option<&Identifier>, i32)> {
        (self.attacker.matches(attacker_in_tag) && self.target.matches(target_in_tag)).then(|| (self.owner.as_ref(), self.grant.compute(entry.effective_level())))
    }
}

/// `tconstruct:mining_capacity`: adds to (or drains) a bar whenever the tool breaks a block, before the break (`run_before`) or after it. The
/// original's grant is scaled by the number of blocks the break harvested; an area tool is not ported, so that is the one block.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MiningFill { grant: LevelingInt, before: bool, owner: Option<Identifier> }

impl MiningFill {
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        let grant = match fields.get("grant").filter(|value| !value.is_null()) {
            None => LevelingInt { flat: 0, each: 0 },
            Some(Value::Object(grant)) => LevelingInt::decode(grant, 0)?,
            Some(_) => return Err("grant must be an object".into()),
        };
        let before = match fields.get("run_before").filter(|value| !value.is_null()) {
            None => false,
            Some(Value::Bool(flag)) => *flag,
            Some(_) => return Err("run_before must be a boolean".into()),
        };
        Ok(Self { grant, before, owner: owner_of(fields)? })
    }

    /// MiningCapacityModule.startHarvest (`before` true) or finishHarvest (false) for one harvested block: the bar's owner and the grant when
    /// the module runs in that phase.
    pub(crate) fn grant_for(&self, entry: &Entry, before: bool) -> Option<(Option<&Identifier>, i32)> {
        (self.before == before).then(|| (self.owner.as_ref(), self.grant.compute(entry.effective_level())))
    }
}

/// `tconstruct:launch_capacity`: adds to (or drains) a bar whenever the tool launches a projectile (LaunchCapacityModule: the same grant on
/// the launch, shot and thrown hooks, once for each projectile). The grant defaults to zero, which empties the bar (war charge).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LaunchFill { grant: LevelingInt, owner: Option<Identifier> }

impl LaunchFill {
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        let grant = match fields.get("grant").filter(|value| !value.is_null()) {
            None => LevelingInt { flat: 0, each: 0 },
            Some(Value::Object(grant)) => LevelingInt::decode(grant, 0)?,
            Some(_) => return Err("grant must be an object".into()),
        };
        Ok(Self { grant, owner: owner_of(fields)? })
    }

    /// LaunchCapacityModule.onProjectileShoot: the bar's owner and the grant for one projectile launched.
    pub(crate) fn grant_for(&self, entry: &Entry) -> (Option<&Identifier>, i32) { (self.owner.as_ref(), self.grant.compute(entry.effective_level())) }
}

/// `tconstruct:loot_to_capacity`: eats the loot of a mined block that matches `consume` into a bar (stoneshield eats stone). Each eaten item
/// adds `amount` to the bar; with a chance below 1 every item of a stack is eaten on its own roll.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LootFill { consume: Ingredient, amount: i32, chance: Leveling, owner: Option<Identifier> }

impl LootFill {
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        let consume = Ingredient::decode(fields.get("consume").filter(|value| !value.is_null()).ok_or("consume is required")?)?;
        let amount = match fields.get("amount") {
            Some(Value::Number(number)) => crate::materials::number_int(&number.to_string()),
            _ => return Err("amount must be a number".into()),
        };
        if amount < 1 { return Err("amount must be at least 1".into()); }
        if !fields.get("chance").is_some_and(Value::is_object) { return Err("chance must be an object".into()); }
        Ok(Self { consume, amount, chance: Leveling::decode(fields, "chance", Leveling::ZERO)?, owner: owner_of(fields)? })
    }

    pub(crate) fn owner(&self) -> Option<&Identifier> { self.owner.as_ref() }

    /// LootToCapacityModule.processLoot: removes the eaten items from `loot` and returns how much the bar gains (0 when nothing was eaten).
    /// `in_tag(item, tag)` answers item tag membership; `random` is uniform in [0, 1).
    pub(crate) fn eat(&self, entry: &Entry, loot: &mut Vec<ItemSlot>, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool, random: &mut dyn FnMut() -> f32) -> i32 {
        let chance = self.chance.compute(entry.effective_level());
        let mut added: i32 = 0;
        loot.retain_mut(|stack| {
            if !self.consume.matches(&stack.item, in_tag) { return true; }
            if chance >= 1.0 {
                added = added.wrapping_add(stack.count as i32);
                return false;
            }
            let eaten = (0..stack.count).filter(|_| random() < chance).count() as u32;
            added = added.wrapping_add(eaten as i32);
            stack.count -= eaten;
            stack.count > 0
        });
        added.wrapping_mul(self.amount)
    }
}

/// `tconstruct:damage_to_capacity`: armor that turns the damage its wearer takes into bar capacity; once the bar is full the wearer takes it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DamageFill { source: SourceCondition, multiplier: Leveling, reduce_damage: bool, owner: Option<Identifier> }

impl DamageFill {
    /// `None` when the source predicate needs state this port does not hold.
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        let Some(source) = fields.get("source").filter(|value| !value.is_null()) else { return Err("source is required".into()); };
        let Some(source) = SourceCondition::decode(source)? else { return Ok(None); };
        let multiplier = match fields.get("multiplier") { Some(Value::Object(_)) => Leveling::decode(fields, "multiplier", Leveling::ZERO)?, _ => return Err("multiplier must be an object".into()) };
        let reduce_damage = match fields.get("reduce_damage") { Some(Value::Bool(flag)) => *flag, _ => return Err("reduce_damage must be a boolean".into()) };
        Ok(Some(Self { source, multiplier, reduce_damage, owner: owner_of(fields)? }))
    }

    pub(crate) fn matches(&self, source: &DamageSource) -> bool { self.source.matches(source) }
    pub(crate) fn owner(&self) -> Option<&Identifier> { self.owner.as_ref() }

    /// DamageToCapacityModule.modifyDamageTaken for a matching source: the damage after the bar took what it can of it.
    pub(crate) fn absorb(&self, nbt: &mut NbtCompound, target: &Target, entry: &Entry, amount: f32) -> f32 {
        let (capacity, current) = (target.capacity(nbt), target.amount(nbt));
        // Once the bar is full the damage is taken directly.
        if current >= capacity { return amount; }
        let wanted = (amount * self.multiplier.compute(entry.effective_level())).ceil() as i32;
        let added = (capacity - current).min(wanted);
        target.set(nbt, current.wrapping_add(added));
        if self.reduce_damage { amount - added as f32 } else { amount }
    }
}

/// The edits that turn `before` into `after` where bars live: the changed, added and removed keys of the persistent data.
pub(crate) fn persistent_edits(before: &NbtCompound, after: &NbtCompound) -> Vec<StackDataEdit> {
    let data = |nbt: &NbtCompound| match nbt.get(TAG_PERSISTENT) { Some(Nbt::Compound(data)) => data.clone(), _ => NbtCompound::new() };
    let (was, now) = (data(before), data(after));
    let keys: std::collections::BTreeSet<&String> = was.keys().chain(now.keys()).collect();
    keys.into_iter().filter(|key| was.get(*key) != now.get(*key))
        .map(|key| StackDataEdit { path: vec![TAG_PERSISTENT.to_string(), key.clone()], value: now.get(key).cloned() }).collect()
}

/// The shield part of tool wear over every bar of the tool, in priority order (highest first, ties in modifier order), before any other
/// reduction: each data-defined shield soaks from its own bar unless the damaging modifier is outside its `cause`, and overslime takes its
/// turn at its own priority. Stops at zero. `cause` is the damaging modifier, None for ordinary use.
pub(crate) fn absorb(nbt: &mut NbtCompound, env: &Environment, cause: Option<&Identifier>, mut damage: i32, random: &mut dyn FnMut() -> f32) -> i32 {
    if damage <= 0 { return damage; }
    let saved = saved_modifiers(nbt, env.modifiers);
    let mut order: Vec<usize> = (0..saved.len()).collect();
    order.sort_by_key(|&index| (std::cmp::Reverse(saved[index].1.priority), index));
    let in_tag = |modifier: &Identifier, tag: &Identifier| env.modifiers.reverse_tags.get(modifier).is_some_and(|tags| tags.contains(tag));
    for index in order {
        let (entry, modifier) = &saved[index];
        if entry.id == overslime::id() {
            damage = overslime::absorb(nbt, env, cause, damage);
        } else if let Some(bar) = modifier.capacity_bar() {
            for shield in modifier.shields() {
                if shield.cause.matches(cause, &in_tag) {
                    let capacity = bar.capacity(nbt, entry);
                    damage = shield.soak(nbt, entry, capacity, damage, random);
                }
            }
        }
        if damage <= 0 { return 0; }
    }
    damage
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn entry(level: i32) -> Entry { Entry::new(Identifier::parse("tconstruct:stoneshield").unwrap(), level) }
    fn bar(value: Value) -> Bar { Bar::decode(value.as_object().unwrap()).unwrap().unwrap() }
    fn shield(value: Value) -> Shield { Shield::decode(value.as_object().unwrap()).unwrap().unwrap() }
    fn with_amount(amount: i32) -> NbtCompound {
        let mut nbt = NbtCompound::new();
        set_amount(&mut nbt, &entry(1).id, i32::MAX, amount);
        nbt
    }
    fn multipliers(durability: f32) -> NbtCompound {
        let mut data = NbtCompound::new();
        data.insert("tconstruct:durability".into(), Nbt::Float(durability));
        let mut nbt = NbtCompound::new();
        nbt.insert(TAG_MULTIPLIERS.into(), Nbt::Compound(data));
        nbt
    }

    #[test]
    fn the_capacity_scales_with_the_level_and_the_multiplier_stat() {
        let stone = bar(json!({"each_level":100,"multiplier":"tconstruct:durability"}));
        assert_eq!(stone.capacity(&multipliers(1.5), &entry(2)), 300);
        assert_eq!(stone.capacity(&NbtCompound::new(), &entry(2)), 200, "no multiplier is 1");
        let flat = bar(json!({"flat":25}));
        assert_eq!(flat.capacity(&multipliers(3.0), &entry(1)), 25, "no multiplier stat, no scaling");
        assert!(Bar::decode(json!({"flat":25,"multiplier":"tconstruct:fluid_capacity"}).as_object().unwrap()).unwrap().is_none(), "a stat this port lacks stays unbound");
        assert!(Bar::decode(json!({"flat":70000}).as_object().unwrap()).is_err(), "the numbers are shorts");
    }

    #[test]
    fn the_bar_amount_is_capped_and_removed_at_zero() {
        let id = entry(1).id;
        let mut nbt = NbtCompound::new();
        set_amount(&mut nbt, &id, 50, 80);
        assert_eq!(amount(&nbt, &id), 50);
        set_amount(&mut nbt, &id, 50, 0);
        let Some(Nbt::Compound(persistent)) = nbt.get(TAG_PERSISTENT) else { panic!("persistent data stays") };
        assert!(persistent.is_empty(), "zero removes the key");
    }

    #[test]
    fn a_shield_soaks_wear_until_it_runs_dry() {
        let module = shield(json!({"color":"7F7F7F"}));
        let mut nbt = with_amount(10);
        assert_eq!(module.soak(&mut nbt, &entry(1), 100, 4, &mut || 0.0), 0);
        assert_eq!(amount(&nbt, &entry(1).id), 6);
        assert_eq!(module.soak(&mut nbt, &entry(1), 100, 9, &mut || 0.0), 3, "what the bar cannot hold gets through");
        assert_eq!(amount(&nbt, &entry(1).id), 0);
    }

    #[test]
    fn a_cost_above_one_spends_more_of_the_bar_and_the_remainder_is_a_chance() {
        let module = shield(json!({"color":"7F7F7F","cost":{"flat":3}}));
        let mut nbt = with_amount(10);
        assert_eq!(module.soak(&mut nbt, &entry(1), 100, 2, &mut || 0.0), 0);
        assert_eq!(amount(&nbt, &entry(1).id), 4, "two points cost six");
        // 4 / 3 = one point soaked, the remainder 1 beats a roll of 0 out of 3.
        let mut lucky = with_amount(4);
        assert_eq!(module.soak(&mut lucky, &entry(1), 100, 5, &mut || 0.0), 3);
        let mut unlucky = with_amount(4);
        assert_eq!(module.soak(&mut unlucky, &entry(1), 100, 5, &mut || 0.9), 4);
        assert_eq!(amount(&unlucky, &entry(1).id), 0);
    }

    #[test]
    fn the_cause_decides_which_modifiers_wear_the_shield_ignores() {
        let default = shield(json!({"color":"7F7F7F"}));
        let bypass = Identifier::parse("tconstruct:durability/bypass_extra_bar").unwrap();
        let in_tag = |modifier: &Identifier, tag: &Identifier| modifier.path() == "reinforced" && *tag == bypass;
        assert!(default.cause.matches(None, &in_tag), "ordinary wear is soaked");
        assert!(default.cause.matches(Some(&Identifier::parse("tconstruct:other").unwrap()), &in_tag));
        assert!(!default.cause.matches(Some(&Identifier::parse("tconstruct:reinforced").unwrap()), &in_tag), "a bypassing modifier is not");
        assert!(Shield::decode(json!({"color":"7F7F7F","cause":{"type":"mantle:and","predicates":[]}}).as_object().unwrap()).unwrap().is_none(), "an unknown predicate stays unbound");
        assert!(Shield::decode(json!({}).as_object().unwrap()).is_err(), "the color is required");
    }

    #[test]
    fn mining_capacity_runs_in_its_phase_and_scales_with_the_level() {
        let drain = MiningFill::decode(json!({"grant":{"each_level":-1},"owner":"tconstruct:overslime","run_before":true}).as_object().unwrap()).unwrap();
        assert_eq!(drain.grant_for(&entry(3), true), Some((Some(&overslime::id()), -3)));
        assert_eq!(drain.grant_for(&entry(3), false), None, "a before module is silent after the break");
        let fill = MiningFill::decode(json!({"grant":{"flat":1}}).as_object().unwrap()).unwrap();
        assert_eq!(fill.grant_for(&entry(1), false), Some((None, 1)));
        assert_eq!(fill.grant_for(&entry(1), true), None);
        assert_eq!(MiningFill::decode(json!({}).as_object().unwrap()).unwrap().grant_for(&entry(2), false), Some((None, 0)), "no grant is zero, which empties the bar");
        assert!(MiningFill::decode(json!({"run_before":"yes"}).as_object().unwrap()).is_err());
    }

    fn stone_tag(item: &Identifier, tag: &Identifier) -> bool { tag.path() == "stoneshields" && matches!(item.path(), "cobblestone" | "netherrack") }
    fn slot(item: &str, count: u32) -> ItemSlot { ItemSlot::new(Identifier::parse(item).unwrap(), count) }
    fn eater(chance: Value) -> LootFill {
        LootFill::decode(json!({"amount":3,"chance":chance,"consume":{"tag":"tconstruct:stoneshields"}}).as_object().unwrap()).unwrap()
    }

    #[test]
    fn loot_to_capacity_eats_every_matching_item_at_full_chance_and_scales_the_gain_by_the_amount() {
        let mut loot = vec![slot("minecraft:cobblestone", 4), slot("minecraft:diamond", 1), slot("minecraft:netherrack", 2)];
        let gain = eater(json!({"each_level":0.5})).eat(&entry(2), &mut loot, &stone_tag, &mut || 0.99);
        assert_eq!(gain, 18, "six items at three each");
        assert_eq!(loot.iter().map(|stack| stack.item.to_string()).collect::<Vec<_>>(), ["minecraft:diamond"], "only what the tag does not name is left");
    }

    #[test]
    fn loot_to_capacity_rolls_for_each_item_below_full_chance_and_keeps_the_rest_of_a_stack() {
        let module = eater(json!({"each_level":0.2}));
        // level 1 is a 20% chance: rolls 0.1 eat, 0.5 keep.
        let mut rolls = [0.1, 0.5, 0.1, 0.5].into_iter();
        let mut loot = vec![slot("minecraft:cobblestone", 4)];
        assert_eq!(module.eat(&entry(1), &mut loot, &stone_tag, &mut || rolls.next().unwrap()), 6);
        assert_eq!(loot[0].count, 2);
        let mut all = vec![slot("minecraft:cobblestone", 2)];
        assert_eq!(module.eat(&entry(1), &mut all, &stone_tag, &mut || 0.0), 6);
        assert!(all.is_empty(), "a stack eaten whole is removed");
        let mut none = vec![slot("minecraft:cobblestone", 2)];
        assert_eq!(module.eat(&entry(1), &mut none, &stone_tag, &mut || 0.9), 0);
        assert_eq!(none[0].count, 2);
    }

    #[test]
    fn launch_capacity_grants_per_projectile_and_no_grant_empties_the_bar() {
        let drain = LaunchFill::decode(json!({"grant":{"each_level":-1},"owner":"tconstruct:overslime"}).as_object().unwrap()).unwrap();
        assert_eq!(drain.grant_for(&entry(3)), (Some(&overslime::id()), -3));
        assert_eq!(LaunchFill::decode(json!({}).as_object().unwrap()).unwrap().grant_for(&entry(2)), (None, 0), "war charge: no grant is zero, which empties the bar");
        assert!(LaunchFill::decode(json!({"grant":1}).as_object().unwrap()).is_err());
        assert!(LaunchFill::decode(json!({"owner":5}).as_object().unwrap()).is_err());
    }

    #[test]
    fn loot_to_capacity_needs_its_ingredient_amount_and_chance() {
        let decode = |value: Value| LootFill::decode(value.as_object().unwrap());
        assert!(decode(json!({"amount":3,"chance":{"flat":1.0}})).is_err());
        assert!(decode(json!({"consume":{"tag":"a:b"},"amount":0,"chance":{"flat":1.0}})).is_err(), "the amount starts at one");
        assert!(decode(json!({"consume":{"tag":"a:b"},"amount":1})).is_err());
        assert!(decode(json!({"consume":{"tag":"a:b"},"amount":1,"chance":{"flat":1.0},"owner":"tconstruct:overslime"})).unwrap().owner().is_some());
    }
}
