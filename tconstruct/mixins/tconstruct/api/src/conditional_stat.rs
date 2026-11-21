//! ConditionalStatModule and ConditionalPowerModule, from SlimeKnights (MIT; see LICENSE): ranged-tool stats and projectile power
//! that depend on what the holder and the target are doing. The conditions are the entity state conditions `melee_damage.rs` already
//! asks (airborne, an effect, in water, full health, a mob type) as pseudo entity type tags, and the entity variables (health, biome
//! temperature, an effect's level, rain, armor coverage) are read from PlutoLib's `EntityState`. A module whose condition or variable needs
//! data this port does not hold stays unbound instead of being half-installed. No game hook reads these yet: nothing in the port draws a
//! bow or fires a projectile, so the states a shot would carry are not reported.
use crate::{formula::Formula, melee_damage::TargetCondition, mining_speed::{StatFormula, ToolView}, modifier_entries::Entry, tool_stats::Stat};
use pluto_lib::{Identifier, Nbt, NbtCompound};
use serde_json::{Map, Value};

/// ConditionalStatFormula.VARIABLES and PowerFormula.VARIABLES.
const STAT_NAMES: [&str; 3] = ["level", "value", "multiplier"];
const POWER_NAMES: [&str; 3] = ["level", "damage", "multiplier"];

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ConditionalStat { stat: Stat, holder: TargetCondition, formula: StatFormula }

impl ConditionalStat {
    /// `None` when the module names a stat, a holder condition or a variable this port does not hold.
    pub fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        let id = Identifier::parse(fields.get("stat").and_then(Value::as_str).ok_or("stat missing")?).map_err(|e| e.to_string())?;
        let Some(stat) = Stat::from_id(&id) else { return Ok(None); };
        // The holder is the one wearing or wielding the tool: the `attacker` role of the state tags.
        let Some(holder) = TargetCondition::decode_for("attacker", fields.get("entity"))? else { return Ok(None); };
        Ok(StatFormula::decode_in(fields, &STAT_NAMES, crate::mining_speed::EntityScope::Holder)?.map(|formula| Self { stat, holder, formula }))
    }
    /// A percent module runs later in the original (priority 75).
    pub fn percent(&self) -> bool { self.formula.percent() }
    /// ConditionalStatModifierHook.modifyStat: `holder_in_tag` answers the holder's pseudo state tags; `multiplier` is the tool's
    /// multiplier for the stat.
    pub fn apply(&self, entry: &Entry, condition: bool, tool: &ToolView, holder_in_tag: &dyn Fn(&Identifier) -> bool, stat: Stat, value: f32, multiplier: f32) -> f32 {
        if self.stat != stat || !condition || !self.holder.matches(holder_in_tag) { return value; }
        self.formula.eval(entry, tool, value, multiplier, 0.0)
    }
}

/// PowerVariable, limited to what the projectile's own data can answer.
#[derive(Clone, Debug, PartialEq)]
enum PowerVariable {
    Constant(f32),
    /// PersistentDataPowerVariable: a float of the projectile's data (0 when absent), the fallback with no projectile.
    ProjectileData { key: String, fallback: f32 },
    /// EntityPowerVariable: a fact of the shooter or the target, `fallback` when it was not reported.
    Entity { fact: crate::entity_facts::EntityFact, whose: crate::entity_facts::Whose, fallback: f32 },
}

impl PowerVariable {
    fn decode(value: &Value) -> Result<Option<Self>, String> {
        if let Some(number) = value.as_f64() { return Ok(Some(Self::Constant(number as f32))); }
        let object = value.as_object().ok_or("variable must be a number or an object")?;
        match object.get("type").and_then(Value::as_str).ok_or("variable type missing")? {
            "tconstruct:persistent_data" => {
                let key = Identifier::parse(object.get("key").and_then(Value::as_str).ok_or("key missing")?).map_err(|e| e.to_string())?.to_string();
                let fallback = object.get("fallback").and_then(Value::as_f64).ok_or("fallback missing")? as f32;
                Ok(Some(Self::ProjectileData { key, fallback }))
            }
            "tconstruct:entity" => {
                use crate::entity_facts::{EntityFact, Whose};
                let whose = match object.get("which").and_then(Value::as_str).ok_or("which missing")? { "attacker" => Whose::Attacker, "target" => Whose::Target, _ => return Ok(None) };
                let fallback = object.get("fallback").and_then(Value::as_f64).ok_or("fallback missing")? as f32;
                let role = if whose == Whose::Attacker { "attacker" } else { "target" };
                let fact = match object.get("entity_type").ok_or("entity_type missing")? {
                    Value::String(kind) => EntityFact::decode(kind, object, role)?,
                    number => EntityFact::decode_value(number, role)?,
                };
                Ok(fact.map(|fact| Self::Entity { fact, whose, fallback }))
            }
            _ => Ok(None),
        }
    }
    fn value(&self, projectile: Option<&NbtCompound>, entities: &crate::entity_facts::Entities) -> f32 {
        match self {
            Self::Constant(value) => *value,
            Self::Entity { fact, whose, fallback } => fact.value(entities, *whose).unwrap_or(*fallback),
            Self::ProjectileData { key, fallback } => match projectile {
                Some(data) => data.get(key).map_or(0.0, number),
                None => *fallback,
            },
        }
    }
}

fn number(value: &Nbt) -> f32 {
    match value { Nbt::Float(v) => *v, Nbt::Double(v) => *v as f32, Nbt::Byte(v) => f32::from(*v), Nbt::Short(v) => f32::from(*v), Nbt::Int(v) => *v as f32, Nbt::Long(v) => *v as f32, _ => 0.0 }
}

#[derive(Clone, Debug, PartialEq)]
enum PowerShape { Boost { flat: f32, each: f32 }, Percent { flat: f32, each: f32 }, PostFix(Formula, Vec<PowerVariable>) }

/// Who held the launcher and what the arrow hit, for one evaluation of a power module.
pub(crate) struct PowerContext<'a> {
    pub holder_in_tag: &'a dyn Fn(&Identifier) -> bool,
    pub target_in_tag: &'a dyn Fn(&Identifier) -> bool,
    /// The holder (as the attacker) and the target as PlutoLib reported them, for the entity variables.
    pub entities: crate::entity_facts::Entities<'a>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ConditionalPower { target: TargetCondition, holder: TargetCondition, percent: bool, shape: PowerShape, min: i32, max: i32 }

/// The projectile data keys: the tool's projectile damage multiplier, kept apart for the launcher and for ammo.
pub(crate) const AMMO_MULTIPLIER: &str = "tconstruct:projectile_damage_ammo_multiplier";
pub(crate) const BOW_MULTIPLIER: &str = "tconstruct:projectile_damage_bow_multiplier";

fn float(fields: &Map<String, Value>, key: &str) -> Result<f32, String> {
    match fields.get(key) {
        None | Some(Value::Null) => Ok(0.0),
        Some(value) => value.as_f64().map(|v| v as f32).ok_or_else(|| format!("{key} must be a number")),
    }
}

impl ConditionalPower {
    /// `None` when the module names a condition or a variable this port does not hold.
    pub fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        let Some(target) = TargetCondition::decode_for("target", fields.get("target"))? else { return Ok(None); };
        let Some(holder) = TargetCondition::decode_for("attacker", fields.get("holder"))? else { return Ok(None); };
        let percent = match fields.get("percent") { None | Some(Value::Null) => false, Some(Value::Bool(v)) => *v, Some(_) => return Err("percent must be boolean".into()) };
        let shape = if let Some(variables) = fields.get("variables") {
            let formula = fields.get("formula").ok_or("Cannot set variables when not using formula")?;
            let mut names: Vec<&str> = POWER_NAMES.to_vec();
            let mut decoded = Vec::new();
            for (name, value) in variables.as_object().ok_or("variables must be an object")? {
                if POWER_NAMES.contains(&name.as_str()) { return Err(format!("Variable {name} is already defined for this module")); }
                match PowerVariable::decode(value)? { Some(variable) => decoded.push(variable), None => return Ok(None) }
                names.push(name);
            }
            PowerShape::PostFix(Formula::decode(formula, &names)?, decoded)
        } else if let Some(formula) = fields.get("formula") {
            PowerShape::PostFix(Formula::decode(formula, &POWER_NAMES)?, Vec::new())
        } else if percent {
            PowerShape::Percent { flat: float(fields, "flat")?, each: float(fields, "each_level")? }
        } else {
            PowerShape::Boost { flat: float(fields, "flat")?, each: float(fields, "each_level")? }
        };
        let (min, max) = crate::modifiers::range(fields.get("modifier_level"))?;
        Ok(Some(Self { target, holder, percent, shape, min, max }))
    }
    /// A percent module runs later in the original (priority 75).
    pub fn percent(&self) -> bool { self.percent }
    /// ModifierEntry.VALID_LEVEL: whether the modifier's level is in the module's range.
    pub fn level_matches(&self, entry: &Entry) -> bool { self.min <= entry.level && entry.level <= self.max }

    fn eval(&self, entry: &Entry, projectile: Option<&NbtCompound>, damage: f32, multiplier: f32, entities: &crate::entity_facts::Entities) -> f32 {
        let level = entry.effective_level();
        match &self.shape {
            PowerShape::Boost { flat, each } => damage + (flat + each * level) * multiplier,
            PowerShape::Percent { flat, each } => damage * (1.0 + (flat + each * level)),
            PowerShape::PostFix(formula, variables) => {
                let mut arguments = vec![level, damage, multiplier];
                arguments.extend(variables.iter().map(|variable| variable.value(projectile, entities)));
                formula.apply(&arguments).unwrap_or(damage)
            }
        }
    }

    /// ProjectileShootModifierHook: the tool's projectile damage multiplier goes into the projectile's data, under the key for ammo or for a launcher.
    pub fn shoot(&self, is_ammo: bool, tool_multiplier: f32, projectile: &mut NbtCompound) {
        projectile.insert((if is_ammo { AMMO_MULTIPLIER } else { BOW_MULTIPLIER }).into(), Nbt::Float(tool_multiplier));
    }

    /// ProjectileHitModifierHook: the arrow's base damage after the module. A modifier boosts once per projectile: the first hit
    /// marks it as run, even when its conditions did not hold.
    pub fn hit(&self, entry: &Entry, projectile: &mut NbtCompound, context: &PowerContext, base_damage: f32) -> f32 {
        let key = entry.id.to_string();
        if !self.level_matches(entry) || matches!(projectile.get(&key), Some(Nbt::Byte(flag)) if *flag != 0) { return base_damage; }
        projectile.insert(key, Nbt::Byte(1));
        if !self.target.matches(context.target_in_tag) || !self.holder.matches(context.holder_in_tag) { return base_damage; }
        let mut multiplier = 1.0;
        for key in [AMMO_MULTIPLIER, BOW_MULTIPLIER] {
            if let Some(value) = projectile.get(key) { multiplier *= number(value); }
        }
        self.eval(entry, Some(projectile), base_damage, multiplier, &context.entities)
    }

    /// SlingForceModifierHook: the force after the module (no projectile, so a projectile variable reads its fallback).
    pub fn sling(&self, entry: &Entry, supported: bool, context: &PowerContext, force: f32, multiplier: f32, tool_multiplier: f32) -> f32 {
        if !self.level_matches(entry) || !supported || !self.holder.matches(context.holder_in_tag) || !self.target.matches(context.target_in_tag) { return force; }
        self.eval(entry, None, force, multiplier * tool_multiplier, &context.entities)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn fields(value: Value) -> Map<String, Value> { value.as_object().unwrap().clone() }
    fn entry(level: i32) -> Entry { Entry::new(Identifier::parse("tconstruct:holy").unwrap(), level) }

    #[test]
    fn entity_variables_read_the_reported_state_and_others_stay_unbound() {
        let water = fields(json!({"formula":["$water",2.0,"/","$level","*","$multiplier","*","$damage","+"],"percent":false,"variables":{"water":{"type":"tconstruct:entity","entity_type":"tconstruct:water","fallback":2.0,"which":"target"}}}));
        let module = ConditionalPower::decode(&water).unwrap().expect("a water variable is read from the target");
        let mut arrow = NbtCompound::new();
        module.shoot(false, 1.0, &mut arrow);
        let soaked = pluto_lib::EntityState { in_water: true, ..Default::default() };
        let context = PowerContext { holder_in_tag: &|_| false, target_in_tag: &|_| false, entities: crate::entity_facts::Entities { attacker: None, target: Some(&soaked) } };
        // 2 / 2 * level 2 * multiplier 1 + damage 3.
        assert_eq!(module.hit(&entry(2), &mut arrow, &context, 3.0), 5.0);
        let mut other = NbtCompound::new();
        module.shoot(false, 1.0, &mut other);
        let unknown = PowerContext { holder_in_tag: &|_| false, target_in_tag: &|_| false, entities: Default::default() };
        // No target reported: the variable's fallback of 2 applies.
        assert_eq!(module.hit(&entry(2), &mut other, &unknown, 3.0), 5.0);
        let light = fields(json!({"formula":["$light","$damage","+"],"percent":false,"variables":{"light":{"type":"tconstruct:entity","entity_type":"tconstruct:equipment_count","slots":["head","chest"],"fallback":0.0,"which":"target"}}}));
        let module = ConditionalPower::decode(&light).unwrap().expect("an equipment count is read from the target");
        let mut arrow = NbtCompound::new();
        module.shoot(false, 1.0, &mut arrow);
        let geared = pluto_lib::EntityState { equipment: Some(vec!["head".to_string()]), ..Default::default() };
        let context = PowerContext { holder_in_tag: &|_| false, target_in_tag: &|_| false, entities: crate::entity_facts::Entities { attacker: None, target: Some(&geared) } };
        // One of the two counted slots holds an item: 1 + damage 3.
        assert_eq!(module.hit(&entry(2), &mut arrow, &context, 3.0), 4.0);
        // The loader requires the slots list, as in the original: without it the variable does not load.
        assert!(ConditionalPower::decode(&fields(json!({"formula":["$light","$damage","+"],"percent":false,"variables":{"light":{"type":"tconstruct:entity","entity_type":"tconstruct:equipment_count","fallback":0.0,"which":"target"}}}))).is_err());
        assert!(ConditionalPower::decode(&fields(json!({"each_level":0.25,"target":"tconstruct:full_health"}))).unwrap().is_some());
        assert!(ConditionalStat::decode(&fields(json!({"stat":"tconstruct:projectile_damage","each_level":0.5,"entity":"tconstruct:full_health"}))).unwrap().is_some());
    }

    #[test]
    fn a_boost_applies_once_per_projectile_with_the_launch_multiplier() {
        let module = ConditionalPower::decode(&fields(json!({"each_level":0.75,"target":{"type":"mantle:mob_type","mobs":"minecraft:undead"}}))).unwrap().unwrap();
        let mut arrow = NbtCompound::new();
        module.shoot(false, 2.0, &mut arrow);
        let undead = |tag: &Identifier| tag.to_string() == "minecraft:undead";
        let context = PowerContext { holder_in_tag: &|_| false, target_in_tag: &undead, entities: Default::default() };
        // 2 + 0.75 * level 2 * multiplier 2.
        assert_eq!(module.hit(&entry(2), &mut arrow, &context, 2.0), 5.0);
        assert_eq!(module.hit(&entry(2), &mut arrow, &context, 5.0), 5.0, "a second target gets no second boost");
        // A first hit on a target that does not match spends the boost.
        let mut other = NbtCompound::new();
        module.shoot(false, 2.0, &mut other);
        let none = PowerContext { holder_in_tag: &|_| false, target_in_tag: &|_| false, entities: Default::default() };
        assert_eq!(module.hit(&entry(2), &mut other, &none, 2.0), 2.0);
        assert_eq!(module.hit(&entry(2), &mut other, &context, 2.0), 2.0);
    }
}
