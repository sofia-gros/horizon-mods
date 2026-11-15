//! Data-driven modifier modules with a ModifierCondition (SlimeKnights, MIT; see LICENSE):
//! volatile_flag, volatile_int, modifier_slot, rarity and stat_boost. Every other module the
//! original ships is recorded as unbound by the caller; a module of these kinds with a hook
//! binding or stat Tinkers cannot honor is also unbound, never half-installed.
use crate::{
    modifier_entries::{nbt_int, Entry},
    tool_predicate::{Facts, Predicate},
    tool_stats::{Builder, Stat},
};
use pluto_lib::{Identifier, Nbt, NbtCompound};
use serde_json::{Map, Value};

/// ModifierCondition: the modifier level range and the tool predicate, both of which must hold.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Condition { tool: Predicate, min: i32, max: i32 }

impl Condition {
    fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        let tool = match fields.get("tool").filter(|value| !value.is_null()) {
            None => Predicate::Any,
            Some(value) => Predicate::decode(value)?,
        };
        let (min, max) = crate::modifiers::range(fields.get("modifier_level"))?;
        Ok(Self { tool, min, max })
    }
    fn matches(&self, entry: &Entry, facts: &Facts) -> bool {
        self.min <= entry.level && entry.level <= self.max && self.tool.matches(facts)
    }
}

/// StatBoostModule.StatOperation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Operation { Add, Percent, MultiplyBase, MultiplyConditional, MultiplyAll }

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Module {
    Flag { id: Identifier, condition: Condition },
    Rarity(i32),
    Int { id: Identifier, flat: i32, each: i32, condition: Condition },
    Slot { name: String, flat: i32, each: i32, condition: Condition },
    Boost { stat: Stat, operation: Operation, flat: f32, each: f32, condition: Condition },
}

fn short(fields: &Map<String, Value>, key: &str) -> Result<i32, String> {
    match fields.get(key).filter(|value| !value.is_null()) {
        None => Ok(0),
        Some(Value::Number(number)) => {
            let value = crate::materials::number_int(&number.to_string());
            if (-32768..=32767).contains(&value) { Ok(value) } else { Err(format!("{key} must fit in a short")) }
        }
        Some(_) => Err(format!("{key} must be a number")),
    }
}
fn float(fields: &Map<String, Value>, key: &str) -> Result<f32, String> {
    match fields.get(key).filter(|value| !value.is_null()) {
        None => Ok(0.0),
        Some(Value::Number(number)) => number.as_f64().map(|value| value as f32).ok_or_else(|| format!("invalid {key}")),
        Some(_) => Err(format!("{key} must be a number")),
    }
}
fn identifier(fields: &Map<String, Value>, key: &str) -> Result<Identifier, String> {
    Identifier::parse(fields.get(key).and_then(Value::as_str).ok_or_else(|| format!("{key} must be an ID string"))?).map_err(|e| e.to_string())
}

impl Module {
    /// `Ok(None)` means a known kind this port cannot honor in this form (unbound);
    /// `Err` means a module it does implement is malformed.
    pub fn decode(kind: &str, fields: &Map<String, Value>, hooks: &[Identifier]) -> Result<Option<Self>, String> {
        let only = |hook: &str| hooks.iter().all(|bound| bound.to_string() == hook);
        // A tool predicate type this port lacks leaves the module unbound; a malformed one still rejects.
        if let Some(predicate) = fields.get("tool").filter(|value| !value.is_null()) {
            if let Err(error) = Predicate::decode(predicate) {
                if error.starts_with("unimplemented tool predicate") || error.starts_with("only single-modifier") { return Ok(None); }
            }
        }
        Ok(Some(match kind {
            "tconstruct:rarity" if only("tconstruct:volatile_data") => {
                // RarityModule has no condition: its loader reads only the rarity.
                Self::Rarity(match fields.get("rarity").and_then(Value::as_str) {
                    Some("common") => 0, Some("uncommon") => 1, Some("rare") => 2, Some("epic") => 3,
                    _ => return Err("rarity must be common, uncommon, rare or epic".into()),
                })
            }
            "tconstruct:volatile_flag" if only("tconstruct:volatile_data") => Self::Flag { id: identifier(fields, "flag")?, condition: Condition::decode(fields)? },
            "tconstruct:volatile_int" if only("tconstruct:volatile_data") => Self::Int {
                id: identifier(fields, "flag")?, flat: short(fields, "flat")?, each: short(fields, "each_level")?, condition: Condition::decode(fields)?,
            },
            "tconstruct:modifier_slot" if only("tconstruct:volatile_data") => {
                let name = fields.get("name").and_then(Value::as_str).ok_or("name must be a slot name")?;
                if !name.bytes().all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'_')) { return Err(format!("invalid modifier slot name {name}")); }
                // The legacy count field is the per-level amount when each_level is absent.
                let each = if fields.contains_key("each_level") { short(fields, "each_level")? } else { short(fields, "count")? };
                Self::Slot { name: name.to_owned(), flat: short(fields, "flat")?, each, condition: Condition::decode(fields)? }
            }
            "tconstruct:stat_boost" if only("tconstruct:tool_stats") => {
                let stat_id = identifier(fields, "stat")?;
                // A stat of another domain (tanks, food) is not representable here.
                let Some(stat) = Stat::from_id(&stat_id) else { return Ok(None); };
                let operation = match fields.get("operation").and_then(Value::as_str) {
                    Some("add") => Operation::Add, Some("percent") => Operation::Percent,
                    Some("multiply_base") => Operation::MultiplyBase,
                    Some("multiply_conditional") => Operation::MultiplyConditional,
                    Some("multiply_all") => Operation::MultiplyAll,
                    _ => return Err("operation must be add, percent, multiply_base, multiply_conditional or multiply_all".into()),
                };
                Self::Boost { stat, operation, flat: float(fields, "flat")?, each: float(fields, "each_level")?, condition: Condition::decode(fields)? }
            }
            _ => return Ok(None),
        }))
    }

    /// The volatile-data hook. Modules of other hooks do nothing here.
    pub fn apply_volatile(&self, entry: &Entry, facts: &Facts, data: &mut NbtCompound) {
        match self {
            Self::Flag { id, condition } => { if condition.matches(entry, facts) { data.insert(id.to_string(), Nbt::Byte(1)); } }
            Self::Rarity(ordinal) => {
                let current = nbt_int(data.get("tconstruct:rarity"));
                if *ordinal > current { data.insert("tconstruct:rarity".into(), Nbt::Int(*ordinal)); }
            }
            Self::Int { id, flat, each, condition } => {
                if condition.matches(entry, facts) {
                    // LevelingInt.compute(float): (int)(flat + eachLevel * level), added with Java wrapping.
                    let amount = (*flat as f32 + *each as f32 * entry.effective_level()) as i32;
                    let previous = nbt_int(data.get(&id.to_string()));
                    data.insert(id.to_string(), Nbt::Int(previous.wrapping_add(amount)));
                }
            }
            Self::Slot { name, flat, each, condition } => {
                if condition.matches(entry, facts) {
                    // computeForLevel: nothing for a non-positive level; addSlots ignores 0 and removes a total of 0.
                    let level = entry.effective_level();
                    let count = if level > 0.0 { (*flat as f32 + *each as f32 * level) as i32 } else { 0 };
                    if count != 0 {
                        let total = nbt_int(data.get(name)).wrapping_add(count);
                        if total == 0 { data.remove(name); } else { data.insert(name.clone(), Nbt::Int(total)); }
                    }
                }
            }
            Self::Boost { .. } => {}
        }
    }

    /// The tool-stats hook.
    pub fn apply_stats(&self, entry: &Entry, facts: &Facts, builder: &mut Builder) {
        let Self::Boost { stat, operation, flat, each, condition } = self else { return; };
        if !condition.matches(entry, facts) || !supports(*stat, facts) { return; }
        let value = f64::from(*flat + *each * entry.effective_level());
        match operation {
            Operation::Add => builder.add(*stat, value),
            Operation::Percent => builder.percent(*stat, value),
            Operation::MultiplyBase => builder.multiply(*stat, 1.0 + value),
            Operation::MultiplyConditional => builder.multiplier_only(*stat, 1.0 + value),
            Operation::MultiplyAll => builder.multiply_all(*stat, 1.0 + value),
        }
    }
}

/// INumericToolStat.supports(item): the item tags a stat applies to, any of them. With no item
/// context (previews outside a tool stack) nothing is filtered.
fn supports(stat: Stat, facts: &Facts) -> bool {
    let Some(item) = facts.item else { return true; };
    let tags: &[&str] = match stat {
        Stat::Durability => &["modifiable/durability"],
        Stat::UseItemSpeed | Stat::BlockAmount | Stat::BlockAngle => &["modifiable/held"],
        Stat::AttackDamage => &["modifiable/melee"],
        Stat::AttackSpeed => &["modifiable/melee/weapon"],
        Stat::MiningSpeed => &["modifiable/harvest"],
        Stat::Armor | Stat::ArmorToughness | Stat::KnockbackResistance => &["modifiable/armor"],
        Stat::DrawSpeed => &["modifiable/ranged"],
        Stat::Velocity | Stat::Accuracy => &["modifiable/ranged", "modifiable/ammo"],
        Stat::ProjectileDamage => &["modifiable/ranged/launcher", "modifiable/ammo"],
        Stat::SeaLuck | Stat::Lure => &["modifiable/fishing_rods"],
        // Water inertia and overslime declare no tag.
        Stat::WaterInertia | Stat::Overslime => return true,
    };
    tags.iter().any(|tag| (facts.in_tag)(item, &Identifier::new("tconstruct", tag).expect("valid tag id")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn decode(value: Value) -> Option<Module> {
        let fields = value.as_object().unwrap();
        let hooks: Vec<Identifier> = fields.get("hooks").and_then(Value::as_array).into_iter().flatten().map(|h| id(h.as_str().unwrap())).collect();
        Module::decode(fields["type"].as_str().unwrap(), fields, &hooks).unwrap()
    }
    fn facts<'a>(item: Option<&'a Identifier>, in_tag: &'a dyn Fn(&Identifier, &Identifier) -> bool) -> Facts<'a> {
        Facts { item, in_tag, level: &|_, _| 0, has_persistent_key: &|_| false }
    }
    #[test]
    fn leveling_arithmetic_and_slot_rules_follow_the_original() {
        let in_tag = |_: &Identifier, _: &Identifier| true;
        let f = facts(None, &in_tag);
        let mut data = NbtCompound::new();
        // volatile_int: (int)(flat + each * level), added to what is there.
        let int = decode(json!({"type":"tconstruct:volatile_int","flag":"test:n","flat":1,"each_level":2}));
        int.as_ref().unwrap().apply_volatile(&Entry::new(id("test:m"), 3), &f, &mut data);
        int.as_ref().unwrap().apply_volatile(&Entry::new(id("test:m"), 3), &f, &mut data);
        assert_eq!(data["test:n"], Nbt::Int(14));
        // modifier_slot: legacy count is the per-level amount; a total of zero removes the key.
        let plus = decode(json!({"type":"tconstruct:modifier_slot","name":"upgrades","count":1})).unwrap();
        let minus = decode(json!({"type":"tconstruct:modifier_slot","name":"upgrades","each_level":-1})).unwrap();
        plus.apply_volatile(&Entry::new(id("test:m"), 2), &f, &mut data);
        assert_eq!(data["upgrades"], Nbt::Int(2));
        minus.apply_volatile(&Entry::new(id("test:m"), 2), &f, &mut data);
        assert!(!data.contains_key("upgrades"));
        // A condition on the modifier level gates the module.
        let gated = decode(json!({"type":"tconstruct:volatile_flag","flag":"test:f","modifier_level":{"min":2}})).unwrap();
        gated.apply_volatile(&Entry::new(id("test:m"), 1), &f, &mut data);
        assert!(!data.contains_key("test:f"));
        gated.apply_volatile(&Entry::new(id("test:m"), 2), &f, &mut data);
        assert_eq!(data["test:f"], Nbt::Byte(1));
    }
    #[test]
    fn stat_boosts_apply_each_operation_only_to_supported_conditioned_tools() {
        let pickaxe = id("tconstruct:pickaxe");
        let in_tag = |item: &Identifier, tag: &Identifier| item.path() == "pickaxe" && matches!(tag.path(), "modifiable/harvest" | "modifiable/durability");
        let f = facts(Some(&pickaxe), &in_tag);
        let boost = |stat: &str, operation: &str, flat: f64, each: f64| decode(json!({"type":"tconstruct:stat_boost","stat":stat,"operation":operation,"flat":flat,"each_level":each})).unwrap();
        let mut builder = Builder::default();
        let entry = Entry::new(id("test:m"), 2);
        boost("tconstruct:durability", "add", 10.0, 5.0).apply_stats(&entry, &f, &mut builder);
        boost("tconstruct:durability", "multiply_base", 0.0, 0.5).apply_stats(&entry, &f, &mut builder);
        boost("tconstruct:mining_speed", "multiply_all", 0.0, 0.25).apply_stats(&entry, &f, &mut builder);
        // Not supported by this item's tags: armor stats do nothing on a pickaxe.
        boost("tconstruct:armor", "add", 5.0, 0.0).apply_stats(&entry, &f, &mut builder);
        assert_eq!(builder.get(Stat::Durability), (1.0 + 20.0) * 2.0, "base 1 + add (10 + 5*2), then x(1 + 0.5*2)");
        assert_eq!(builder.get(Stat::Armor), Stat::Armor.default_value());
        assert_eq!(builder.multiplier(Stat::MiningSpeed), 1.5, "multiply_all also records the global multiplier");
        // multiply_conditional only records the multiplier, leaving the stat itself alone.
        let mut conditional = Builder::default();
        boost("tconstruct:mining_speed", "multiply_conditional", 0.0, 0.5).apply_stats(&entry, &f, &mut conditional);
        assert_eq!(conditional.multiplier(Stat::MiningSpeed), 2.0);
        assert_eq!(conditional.get(Stat::MiningSpeed), Stat::MiningSpeed.default_value());
        // A tool predicate condition that fails skips the boost.
        let armor_only = decode(json!({"type":"tconstruct:stat_boost","stat":"tconstruct:durability","operation":"add","flat":99,
            "tool":{"type":"mantle:tag","tag":"tconstruct:modifiable/armor"}})).unwrap();
        let mut skipped = Builder::default();
        armor_only.apply_stats(&entry, &f, &mut skipped);
        assert_eq!(skipped.get(Stat::Durability), Stat::Durability.default_value());
    }
    #[test]
    fn unrepresentable_shapes_are_unbound_and_malformed_modules_reject() {
        assert!(decode(json!({"type":"tconstruct:stat_boost","stat":"tconstruct:tank_capacity","operation":"add","flat":1})).is_none(), "tank stats belong to another domain");
        assert!(decode(json!({"type":"tconstruct:volatile_flag","flag":"a:b","hooks":["tconstruct:projectile_launch"]})).is_none());
        assert!(decode(json!({"type":"tconstruct:stat_boost","stat":"tconstruct:durability","operation":"add","flat":1,"hooks":["tconstruct:volatile_data"]})).is_none());
        assert!(decode(json!({"type":"tconstruct:conditional_melee_damage"})).is_none());
        for (kind, fields) in [("tconstruct:stat_boost", json!({"stat":"tconstruct:durability","operation":"explode"})),
            ("tconstruct:modifier_slot", json!({"name":"Bad Name"})), ("tconstruct:volatile_int", json!({"flag":"a:b","flat":99999})),
            ("tconstruct:rarity", json!({"rarity":"mythic"})), ("tconstruct:volatile_flag", json!({"flag":"a:b","modifier_level":0})),
            ("tconstruct:volatile_flag", json!({"flag":"a:b","tool":{}}))] {
            assert!(Module::decode(kind, fields.as_object().unwrap(), &[]).is_err(), "{kind} {fields}");
        }
        // A well-formed predicate of a type this port lacks leaves just that module unbound.
        assert!(decode(json!({"type":"tconstruct:volatile_flag","flag":"a:b","tool":{"type":"tconstruct:has_hook","hook":"x:y"}})).is_none());
    }
}
