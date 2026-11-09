//! Modifier modules that change a tool's stats: `StatBoostModule` (`tconstruct:stat_boost`) and the pieces it needs (`LevelingValue`,
//! `StatOperation`, the `tool` and `modifier_level` conditions).
use std::any::Any;
use std::sync::Arc;

use serde_json::{Map, Value};

use super::{ModifierEntry, ModifierModule, ModuleLoader};
use crate::predicate;
use crate::tools::stat::{ModifierStatsBuilder, ToolStatId, ToolStats};

/// `LevelingValue`: a flat amount plus an amount for each level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LevelingValue {
    pub flat: f32,
    pub each_level: f32,
}

impl LevelingValue {
    /// `LevelingValue.LOADABLE`: `flat` and `each_level`, both 0 when missing.
    pub fn from_json(object: &Map<String, Value>) -> Result<Self, String> {
        let number = |key: &str| match object.get(key) {
            None => Ok(0.0),
            Some(value) => value.as_f64().map(|v| v as f32).ok_or_else(|| format!("'{key}' must be a number")),
        };
        Ok(Self { flat: number("flat")?, each_level: number("each_level")? })
    }

    /// `compute(level)`.
    pub fn compute(&self, level: f32) -> f32 {
        self.flat + self.each_level * level
    }
}

/// `StatBoostModule.StatOperation`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatOperation {
    Add,
    Percent,
    MultiplyBase,
    MultiplyConditional,
    MultiplyAll,
}

impl StatOperation {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "add" => Some(Self::Add),
            "percent" => Some(Self::Percent),
            "multiply_base" => Some(Self::MultiplyBase),
            "multiply_conditional" => Some(Self::MultiplyConditional),
            "multiply_all" => Some(Self::MultiplyAll),
            _ => None,
        }
    }

    /// `apply`.
    pub fn apply(self, builder: &mut ModifierStatsBuilder<'_>, stat: &ToolStatId, value: f32) {
        let value = f64::from(value);
        match self {
            Self::Add => builder.add(stat, value),
            Self::Percent => builder.percent(stat, value),
            Self::MultiplyBase => builder.multiply(stat, 1.0 + value),
            Self::MultiplyConditional => builder.add_multiplier(stat, 1.0 + value),
            Self::MultiplyAll => builder.multiply_all(stat, 1.0 + value),
        }
    }
}

/// What a module may ask about the tool it works on (`IToolContext`).
pub trait ToolContext {
    /// Whether the tool's item is in the item tag.
    fn item_in_tag(&self, tag: &str) -> bool;
}

/// The error a loader gives for a shape of its data it does not carry yet; the registry then keeps the module as raw data.
pub const UNSUPPORTED: &str = "unsupported condition";

/// A loader that falls back to `raw` for data shapes it does not carry.
pub fn with_raw_fallback(loader: ModuleLoader, raw: ModuleLoader) -> ModuleLoader {
    Arc::new(move |json| match loader(json) {
        Err(error) if error == UNSUPPORTED => raw(json),
        other => other,
    })
}

/// `ModifierCondition` as `stat_boost` uses it: an optional `tool` predicate and an optional `modifier_level` range.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Condition {
    pub tool: Option<Value>,
    pub min_level: Option<i32>,
    pub max_level: Option<i32>,
}

impl Condition {
    pub fn from_json(object: &Map<String, Value>) -> Result<Self, String> {
        let mut condition = Self { tool: object.get("tool").cloned(), ..Self::default() };
        if let Some(range) = object.get("modifier_level") {
            if let Some(exact) = range.as_i64() {
                // a single number is that level exactly
                condition.min_level = Some(exact as i32);
                condition.max_level = Some(exact as i32);
                return Ok(condition);
            }
            let range = range.as_object().ok_or("'modifier_level' must be a number or an object with 'min' and 'max'")?;
            if range.contains_key("type") {
                // a predicate over the level rather than a range: the module is kept as raw data
                return Err(UNSUPPORTED.to_owned());
            }
            condition.min_level = range.get("min").and_then(Value::as_i64).map(|v| v as i32);
            condition.max_level = range.get("max").and_then(Value::as_i64).map(|v| v as i32);
        }
        Ok(condition)
    }

    /// `matches(context, modifier)`.
    pub fn matches(&self, context: &dyn ToolContext, entry: &ModifierEntry) -> bool {
        if self.min_level.is_some_and(|min| entry.level < min) || self.max_level.is_some_and(|max| entry.level > max) {
            return false;
        }
        match &self.tool {
            None => true,
            Some(tool) => predicate::test(tool, &|kind, object| match kind {
                "mantle:tag" => Ok(object.get("tag").and_then(Value::as_str).is_some_and(|tag| context.item_in_tag(tag))),
                other => Err(format!("unsupported item predicate {other}")),
            })
            .unwrap_or(false),
        }
    }
}

/// `StatBoostModule`.
#[derive(Clone, Debug, PartialEq)]
pub struct StatBoostModule {
    pub stat: ToolStatId,
    pub operation: StatOperation,
    pub amount: LevelingValue,
    pub condition: Condition,
}

impl StatBoostModule {
    /// `addToolStats(context, modifier, builder)`; `supports` says whether the stat applies to the tool's item.
    pub fn add_tool_stats(&self, context: &dyn ToolContext, entry: &ModifierEntry, stats: &ToolStats, builder: &mut ModifierStatsBuilder<'_>) {
        let Some(stat) = stats.get(&self.stat) else { return };
        if self.condition.matches(context, entry) && stat.supports(&|tag| context.item_in_tag(tag)) {
            self.operation.apply(builder, &self.stat, self.amount.compute(entry.level as f32));
        }
    }
}

impl ModifierModule for StatBoostModule {
    fn kind(&self) -> &str {
        "tconstruct:stat_boost"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The loader of `tconstruct:stat_boost`; the stat must be a numeric one the registry knows.
pub fn stat_boost_loader(stats: Arc<ToolStats>) -> ModuleLoader {
    Arc::new(move |json| {
        let name = json.get("stat").and_then(Value::as_str).ok_or("stat_boost needs a 'stat'")?;
        let id = ToolStatId::try_parse(name).ok_or_else(|| format!("bad stat id '{name}'"))?;
        let stat = stats.get(&id).ok_or_else(|| format!("Unknown stat type {name}"))?;
        if !stat.is_numeric() {
            return Err(format!("Invalid tool stat {name}, must be a numeric stat"));
        }
        let operation = json.get("operation").and_then(Value::as_str).and_then(StatOperation::from_name).ok_or("stat_boost needs a valid 'operation'")?;
        Ok(Arc::new(StatBoostModule { stat: id, operation, amount: LevelingValue::from_json(json)?, condition: Condition::from_json(json)? }) as Arc<dyn ModifierModule>)
    })
}

/// `ModifierTraitModule` (`tconstruct:trait`): while the tool has this modifier, it also has another one. With `fixed_level` the trait
/// is added once at its own level; otherwise it is added at its level times this modifier's level.
#[derive(Clone, Debug, PartialEq)]
pub struct ModifierTraitModule {
    pub modifier: ModifierEntry,
    pub fixed_level: bool,
    pub condition: Condition,
}

impl ModifierTraitModule {
    /// `addTraits`: the entry to add, if the module applies. `first_encounter` is whether this modifier was met for the first time.
    pub fn trait_for(&self, context: &dyn ToolContext, own: &ModifierEntry, first_encounter: bool) -> Option<ModifierEntry> {
        if !self.condition.matches(context, own) {
            return None;
        }
        if self.fixed_level {
            first_encounter.then(|| self.modifier.clone())
        } else if own.level == 1 {
            Some(self.modifier.clone())
        } else {
            Some(ModifierEntry { modifier: self.modifier.modifier.clone(), level: self.modifier.level * own.level })
        }
    }
}

impl ModifierModule for ModifierTraitModule {
    fn kind(&self) -> &str {
        "tconstruct:trait"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The loader of `tconstruct:trait`.
pub fn trait_loader() -> ModuleLoader {
    Arc::new(|json| {
        let modifier = ModifierEntry::from_json(&Value::Object(json.clone()))?;
        let fixed_level = json.get("fixed_level").and_then(Value::as_bool).ok_or("trait needs 'fixed_level'")?;
        Ok(Arc::new(ModifierTraitModule { modifier, fixed_level, condition: Condition::from_json(json)? }) as Arc<dyn ModifierModule>)
    })
}

/// `ToolDataNBT`: the volatile data of a tool, worked out again on every rebuild: flags, whole numbers and modifier slots.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VolatileData {
    pub flags: std::collections::BTreeSet<String>,
    pub ints: std::collections::BTreeMap<String, i32>,
    pub slots: std::collections::BTreeMap<String, i32>,
}

impl VolatileData {
    /// `putBoolean(flag, true)`.
    pub fn set_flag(&mut self, flag: &str) {
        self.flags.insert(flag.to_owned());
    }

    /// `getInt(key)`: 0 when missing.
    pub fn int(&self, key: &str) -> i32 {
        self.ints.get(key).copied().unwrap_or(0)
    }

    /// `addSlots(type, count)`.
    pub fn add_slots(&mut self, slot: &str, count: i32) {
        *self.slots.entry(slot.to_owned()).or_insert(0) += count;
    }
}

fn leveling_int(object: &Map<String, Value>) -> Result<(i32, i32), String> {
    let number = |key: &str| match object.get(key) {
        None => Ok(0),
        Some(value) => value.as_i64().map(|v| v as i32).ok_or_else(|| format!("'{key}' must be a whole number")),
    };
    Ok((number("flat")?, number("each_level")?))
}

/// `VolatileFlagModule` (`tconstruct:volatile_flag`): sets a flag while the modifier is on the tool.
#[derive(Clone, Debug, PartialEq)]
pub struct VolatileFlagModule {
    pub flag: String,
    pub condition: Condition,
}

/// `VolatileIntModule` (`tconstruct:volatile_int`): adds `flat + each_level * level` to a whole number.
#[derive(Clone, Debug, PartialEq)]
pub struct VolatileIntModule {
    pub key: String,
    pub flat: i32,
    pub each_level: i32,
    pub condition: Condition,
}

/// `ModifierSlotModule` (`tconstruct:modifier_slot`): adds slots of a type for a level above 0 (`LevelingInt.computeForLevel`).
#[derive(Clone, Debug, PartialEq)]
pub struct ModifierSlotModule {
    pub slot: String,
    pub flat: i32,
    pub each_level: i32,
    pub condition: Condition,
}

/// `RarityModule` (`tconstruct:rarity`): raises the tool's rarity to at least this one (kept as the rarity's index).
#[derive(Clone, Debug, PartialEq)]
pub struct RarityModule {
    pub rarity: crate::materials::Rarity,
}

/// The key `RarityModule.RARITY` keeps the rarity under.
pub const RARITY_KEY: &str = "tconstruct:rarity";

/// The index of a rarity (`Rarity.ordinal`).
pub fn rarity_index(rarity: crate::materials::Rarity) -> i32 {
    use crate::materials::Rarity::{Common, Epic, Rare, Uncommon};
    match rarity {
        Common => 0,
        Uncommon => 1,
        Rare => 2,
        Epic => 3,
    }
}

/// The volatile-data hook of the modules above.
pub fn add_volatile_data(module: &dyn ModifierModule, context: &dyn ToolContext, entry: &ModifierEntry, data: &mut VolatileData) {
    let any = module.as_any();
    if let Some(m) = any.downcast_ref::<VolatileFlagModule>() {
        if m.condition.matches(context, entry) {
            data.set_flag(&m.flag);
        }
    } else if let Some(m) = any.downcast_ref::<VolatileIntModule>() {
        if m.condition.matches(context, entry) {
            let total = data.int(&m.key) + m.flat + m.each_level * entry.level;
            data.ints.insert(m.key.clone(), total);
        }
    } else if let Some(m) = any.downcast_ref::<ModifierSlotModule>() {
        if m.condition.matches(context, entry) && entry.level > 0 {
            data.add_slots(&m.slot, m.flat + m.each_level * entry.level);
        }
    } else if let Some(m) = any.downcast_ref::<RarityModule>() {
        let index = rarity_index(m.rarity);
        if index > data.int(RARITY_KEY) {
            data.ints.insert(RARITY_KEY.to_owned(), index);
        }
    }
}

macro_rules! impl_module {
    ($type:ty, $kind:expr, $priority:expr) => {
        impl ModifierModule for $type {
            fn kind(&self) -> &str {
                $kind
            }
            fn priority(&self) -> Option<i32> {
                $priority
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
        }
    };
}

impl_module!(VolatileFlagModule, "tconstruct:volatile_flag", None);
impl_module!(VolatileIntModule, "tconstruct:volatile_int", None);
impl_module!(ModifierSlotModule, "tconstruct:modifier_slot", Some(50));
impl_module!(RarityModule, "tconstruct:rarity", None);

/// The loaders of the volatile-data modules.
pub fn volatile_loaders() -> Vec<(&'static str, ModuleLoader)> {
    let flag: ModuleLoader = Arc::new(|json| {
        let flag = json.get("flag").and_then(Value::as_str).ok_or("volatile_flag needs a 'flag'")?.to_owned();
        Ok(Arc::new(VolatileFlagModule { flag, condition: Condition::from_json(json)? }) as Arc<dyn ModifierModule>)
    });
    let int: ModuleLoader = Arc::new(|json| {
        let key = json.get("flag").and_then(Value::as_str).ok_or("volatile_int needs a 'flag'")?.to_owned();
        let (flat, each_level) = leveling_int(json)?;
        Ok(Arc::new(VolatileIntModule { key, flat, each_level, condition: Condition::from_json(json)? }) as Arc<dyn ModifierModule>)
    });
    let slot: ModuleLoader = Arc::new(|json| {
        let slot = json.get("name").and_then(Value::as_str).ok_or("modifier_slot needs a 'name'")?.to_owned();
        let (flat, each_level) = leveling_int(json)?;
        Ok(Arc::new(ModifierSlotModule { slot, flat, each_level, condition: Condition::from_json(json)? }) as Arc<dyn ModifierModule>)
    });
    let rarity: ModuleLoader = Arc::new(|json| {
        let rarity = json.get("rarity").and_then(Value::as_str).and_then(crate::materials::Rarity::from_name).ok_or("rarity needs a 'rarity'")?;
        Ok(Arc::new(RarityModule { rarity }) as Arc<dyn ModifierModule>)
    });
    vec![("tconstruct:volatile_flag", flag), ("tconstruct:volatile_int", int), ("tconstruct:modifier_slot", slot), ("tconstruct:rarity", rarity)]
}

#[cfg(test)]
mod volatile_tests {
    use super::*;

    struct Anything;
    impl ToolContext for Anything {
        fn item_in_tag(&self, _: &str) -> bool {
            true
        }
    }

    fn entry(level: i32) -> ModifierEntry {
        ModifierEntry { modifier: crate::modifiers::ModifierId::try_parse("tconstruct:test").unwrap(), level }
    }

    #[test]
    fn flags_ints_slots_and_rarity_accumulate() {
        let mut data = VolatileData::default();
        let condition = Condition::default();
        add_volatile_data(&VolatileFlagModule { flag: "tconstruct:ballista".into(), condition: condition.clone() }, &Anything, &entry(1), &mut data);
        add_volatile_data(&VolatileIntModule { key: "tconstruct:expanded".into(), flat: 1, each_level: 2, condition: condition.clone() }, &Anything, &entry(3), &mut data);
        add_volatile_data(&VolatileIntModule { key: "tconstruct:expanded".into(), flat: 0, each_level: 1, condition: condition.clone() }, &Anything, &entry(1), &mut data);
        add_volatile_data(&ModifierSlotModule { slot: "upgrades".into(), flat: 0, each_level: 1, condition: condition.clone() }, &Anything, &entry(2), &mut data);
        add_volatile_data(&RarityModule { rarity: crate::materials::Rarity::Rare }, &Anything, &entry(1), &mut data);
        add_volatile_data(&RarityModule { rarity: crate::materials::Rarity::Uncommon }, &Anything, &entry(1), &mut data);
        assert!(data.flags.contains("tconstruct:ballista"));
        assert_eq!(data.int("tconstruct:expanded"), 8, "(1 + 2 x 3) + 1");
        assert_eq!(data.slots["upgrades"], 2);
        assert_eq!(data.int(RARITY_KEY), 2, "the rarer one stays");
    }

    #[test]
    fn a_condition_keeps_a_module_from_applying() {
        let mut data = VolatileData::default();
        let condition = Condition { min_level: Some(2), ..Condition::default() };
        add_volatile_data(&VolatileFlagModule { flag: "tconstruct:piglin_neutral".into(), condition }, &Anything, &entry(1), &mut data);
        assert!(data.flags.is_empty());
    }
}

/// `AttributeModifier.Operation`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttributeOperation {
    Addition,
    MultiplyBase,
    MultiplyTotal,
}

impl AttributeOperation {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "addition" => Some(Self::Addition),
            "multiply_base" => Some(Self::MultiplyBase),
            "multiply_total" => Some(Self::MultiplyTotal),
            _ => None,
        }
    }
}

/// One attribute change a modifier gives for an equipment slot.
#[derive(Clone, Debug, PartialEq)]
pub struct AttributeChange {
    pub attribute: crate::resource::ResourceId,
    /// `unique.slot`, the name the change is told apart by.
    pub name: String,
    pub operation: AttributeOperation,
    pub amount: f32,
}

/// The equipment slots, as `EquipmentSlot.getName`.
pub const EQUIPMENT_SLOTS: [&str; 6] = ["mainhand", "offhand", "feet", "legs", "chest", "head"];

/// `AttributeModule` (`tconstruct:attribute`): an attribute changed by `flat + each_level x level` in the slots the tool is worn or held in.
/// The formula form (`formula` and `variables`) needs the variable loaders and stays raw data.
#[derive(Clone, Debug, PartialEq)]
pub struct AttributeModule {
    pub unique: String,
    pub attribute: crate::resource::ResourceId,
    pub operation: AttributeOperation,
    pub amount: LevelingValue,
    pub slots: Vec<String>,
    pub condition: Condition,
}

impl AttributeModule {
    /// `addAttributes`: the change for `slot`, if the module covers it and its condition holds.
    pub fn attributes(&self, context: &dyn ToolContext, entry: &ModifierEntry, slot: &str) -> Option<AttributeChange> {
        if !self.slots.iter().any(|s| s == slot) || !self.condition.matches(context, entry) {
            return None;
        }
        Some(AttributeChange { attribute: self.attribute.clone(), name: format!("{}.{}", self.unique, slot), operation: self.operation, amount: self.amount.compute(entry.level as f32) })
    }
}

impl ModifierModule for AttributeModule {
    fn kind(&self) -> &str {
        "tconstruct:attribute"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The loader of `tconstruct:attribute`. `unique` is the module's name for its changes; the original defaults it from the modifier,
/// here from the attribute when the data does not say.
pub fn attribute_loader(raw: ModuleLoader) -> ModuleLoader {
    Arc::new(move |json| {
        if json.contains_key("formula") {
            return raw(json);
        }
        let attribute = json.get("attribute").and_then(Value::as_str).and_then(crate::resource::ResourceId::try_parse).ok_or("attribute needs an 'attribute'")?;
        let operation = json.get("operation").and_then(Value::as_str).and_then(AttributeOperation::from_name).ok_or("attribute needs a valid 'operation'")?;
        let slots = json
            .get("slots")
            .and_then(Value::as_array)
            .ok_or("attribute needs 'slots'")?
            .iter()
            .map(|s| s.as_str().filter(|s| EQUIPMENT_SLOTS.contains(s)).map(str::to_owned).ok_or("a slot must be an equipment slot"))
            .collect::<Result<Vec<_>, _>>()?;
        let unique = json.get("unique").and_then(Value::as_str).map_or_else(|| attribute.to_string(), str::to_owned);
        Ok(Arc::new(AttributeModule { unique, attribute, operation, amount: LevelingValue::from_json(json)?, slots, condition: Condition::from_json(json)? }) as Arc<dyn ModifierModule>)
    })
}

#[cfg(test)]
mod attribute_tests {
    use super::*;

    struct Anything;
    impl ToolContext for Anything {
        fn item_in_tag(&self, _: &str) -> bool {
            true
        }
    }

    #[test]
    fn an_attribute_module_covers_its_slots_by_level() {
        let json = serde_json::json!({"type": "tconstruct:attribute", "attribute": "tconstruct:generic.good_effect_duration_multiplier", "flat": 0.15, "each_level": 0.1, "operation": "multiply_base", "slots": ["head", "mainhand"]});
        let raw: ModuleLoader = Arc::new(|_| Err("raw".to_owned()));
        let module = attribute_loader(raw)(json.as_object().unwrap()).unwrap();
        let module = module.as_any().downcast_ref::<AttributeModule>().unwrap();
        let entry = ModifierEntry { modifier: crate::modifiers::ModifierId::try_parse("tconstruct:test").unwrap(), level: 2 };
        let change = module.attributes(&Anything, &entry, "head").unwrap();
        assert!((change.amount - 0.35).abs() < 1e-6 && change.operation == AttributeOperation::MultiplyBase);
        assert!(module.attributes(&Anything, &entry, "feet").is_none());
    }

    #[test]
    fn the_formula_form_stays_raw() {
        let json = serde_json::json!({"type": "tconstruct:attribute", "formula": ["$level"], "variables": {}});
        let raw: ModuleLoader = Arc::new(|_| Err("raw".to_owned()));
        assert_eq!(attribute_loader(raw)(json.as_object().unwrap()).err().as_deref(), Some("raw"));
    }
}

/// `ProtectionModule` (`tconstruct:protection`) in its flat form: armor protection of `flat + each_level x level` against the damage
/// sources the `damage_source` predicate accepts. The `wearing_entity` and `attacker` predicates and the formula form need the
/// entity predicates and variable loaders and stay raw data.
#[derive(Clone, Debug, PartialEq)]
pub struct ProtectionModule {
    pub damage_source: Option<Value>,
    pub amount: LevelingValue,
    pub condition: Condition,
}

/// What the game tells about a damage source: whether it is in a damage type tag.
pub trait DamageSourceContext {
    fn in_tag(&self, tag: &str) -> bool;
}

impl ProtectionModule {
    /// `getProtectionModifier`: `value` plus this module's protection when the tool and the damage source match.
    pub fn apply(&self, tool: &dyn ToolContext, source: &dyn DamageSourceContext, entry: &ModifierEntry, value: f32) -> f32 {
        if !self.condition.matches(tool, entry) {
            return value;
        }
        let accepts = match &self.damage_source {
            // Mantle's default is `can_protect`: the damage is not one that goes through armor and enchantments
            None => can_protect(source),
            Some(predicate) => predicate::test(predicate, &|kind, object| match kind {
                "mantle:can_protect" => Ok(can_protect(source)),
                "mantle:tag" => Ok(object.get("tag").and_then(Value::as_str).is_some_and(|tag| source.in_tag(tag))),
                other => Err(format!("unsupported damage source predicate {other}")),
            })
            .unwrap_or(false),
        };
        if accepts { value + self.amount.compute(entry.level as f32) } else { value }
    }
}

fn can_protect(source: &dyn DamageSourceContext) -> bool {
    !source.in_tag("minecraft:bypasses_invulnerability") && !source.in_tag("minecraft:bypasses_enchantments")
}

impl ModifierModule for ProtectionModule {
    fn kind(&self) -> &str {
        "tconstruct:protection"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The loader of `tconstruct:protection`.
pub fn protection_loader() -> ModuleLoader {
    Arc::new(|json| {
        if json.contains_key("formula") || json.contains_key("wearing_entity") || json.contains_key("attacker") {
            return Err(UNSUPPORTED.to_owned());
        }
        Ok(Arc::new(ProtectionModule { damage_source: json.get("damage_source").cloned(), amount: LevelingValue::from_json(json)?, condition: Condition::from_json(json)? }) as Arc<dyn ModifierModule>)
    })
}

#[cfg(test)]
mod protection_tests {
    use super::*;

    struct Anything;
    impl ToolContext for Anything {
        fn item_in_tag(&self, _: &str) -> bool {
            true
        }
    }
    struct Source(&'static [&'static str]);
    impl DamageSourceContext for Source {
        fn in_tag(&self, tag: &str) -> bool {
            self.0.contains(&tag)
        }
    }

    fn module() -> ProtectionModule {
        let json = serde_json::json!({"type": "tconstruct:protection", "each_level": 3.0, "damage_source": {"type": "mantle:and", "predicates": ["mantle:can_protect", {"type": "mantle:tag", "tag": "tconstruct:protection/blast"}]}});
        let module = protection_loader()(json.as_object().unwrap()).unwrap();
        module.as_any().downcast_ref::<ProtectionModule>().unwrap().clone()
    }

    #[test]
    fn protection_adds_by_level_for_the_sources_it_names() {
        let entry = ModifierEntry { modifier: crate::modifiers::ModifierId::try_parse("tconstruct:blast_protection").unwrap(), level: 2 };
        assert_eq!(module().apply(&Anything, &Source(&["tconstruct:protection/blast"]), &entry, 4.0), 10.0);
        assert_eq!(module().apply(&Anything, &Source(&["minecraft:is_fire"]), &entry, 4.0), 4.0);
        assert_eq!(module().apply(&Anything, &Source(&["tconstruct:protection/blast", "minecraft:bypasses_enchantments"]), &entry, 4.0), 4.0, "damage that bypasses enchantments cannot be protected against");
    }

    #[test]
    fn the_entity_and_formula_forms_stay_raw() {
        let json = serde_json::json!({"type": "tconstruct:protection", "attacker": "tconstruct:airborne", "flat": 2.5});
        assert_eq!(protection_loader()(json.as_object().unwrap()).err().as_deref(), Some(UNSUPPORTED));
    }
}
