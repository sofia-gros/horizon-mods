//! Data-driven modifier modules with a ModifierCondition (SlimeKnights, MIT; see LICENSE):
//! volatile_flag, volatile_int, modifier_slot, rarity and stat_boost. Every other module the
//! original ships is recorded as unbound by the caller; a module of these kinds with a hook
//! binding or stat Tinkers cannot honor is also unbound, never half-installed.
use crate::{
    formula::Formula,
    mining_speed::StatFormula,
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
    /// SetStatModule for the harvest tier: the tier becomes the larger of the current one and this.
    SetTier { tier: Identifier, condition: Condition },
    /// ModifierTraitModule: a nested modifier this one brings along.
    Trait { modifier: Identifier, level: i32, fixed_level: bool, condition: Condition },
    /// ReduceToolDamageModule: probabilistically cancels a fraction of tool damage.
    ReduceDamage { cause: CausePredicate, formula: LevelFormula, condition: Condition },
    /// RepairModule: scales the durability a repair restores.
    Repair { formula: LevelFormula, condition: Condition },
    /// ModifierRequirementsModule: a modifier added through a recipe needs this of the tool.
    Requirements { requirement: Predicate, min: i32, max: i32, translation_key: String },
    /// SwappableSlotModule: slots of the type a swappable recipe stored under `key` (default: the modifier's ID).
    SwappableSlot { key: Option<Identifier>, count: i32, condition: Condition },
    /// SwappableSlotModule.BonusSlot: extra slots of `bonus` while the stored slot type is `matching`.
    BonusSlot { key: Option<Identifier>, matching: String, bonus: String, count: i32, condition: Condition },
    /// SwappableToolTraitsModule: the definition's traits for `hook`, while the stored value equals `matching`
    /// (any value when `matching` is empty).
    SwappableTraits { key: Option<Identifier>, matching: String, hook: Identifier },
    /// CreativeSlotModifier: the slots stored in the tool's `tconstruct:creative` persistent compound.
    CreativeSlots,
    /// StatOverrideModifier: the stats stored under `tconstruct:override_bonus` (added, dropping the base) and
    /// `tconstruct:override_multiplier` (multiplied). Only the float stats are supported.
    StatOverride,
    /// ConditionalMiningSpeedModule: changes the dig speed of the blocks it names.
    MiningSpeed { module: Box<crate::mining_speed::MiningSpeed>, condition: Condition },
    /// ConditionalMeleeDamageModule without target or attacker conditions: changes the damage of a melee hit.
    MeleeDamage { module: Box<crate::melee_damage::MeleeDamage>, condition: Condition },
    /// KnockbackModule: adds knockback to a melee hit, filtered on the target's entity type tags.
    Knockback { module: Box<crate::knockback::Knockback>, condition: Condition },
    /// MobEffectModule.Weapon: a melee hit that may put a mob effect on the target.
    WeaponEffect { module: Box<crate::weapon_effect::WeaponEffect>, condition: Condition },
    /// BlockDamageSourceModule: armor that blocks damage of the sources it names.
    BlockDamage { source: crate::protection::SourceCondition, condition: Condition },
    /// EffectImmunityModule: armor that makes its wearer immune to a mob effect.
    EffectImmunity { module: Box<crate::effect_immunity::EffectImmunity>, condition: Condition },
    /// MaxArmorAttributeModule: an attribute worked from the largest level among the worn pieces.
    MaxArmorAttribute { module: Box<crate::max_armor::MaxArmorAttribute>, condition: Condition },
    /// ArmorLevelModule: the modifier's level added up over the worn pieces under a key.
    ArmorLevel { module: Box<crate::max_armor::ArmorLevel> },
    /// ThornsModule: armor that hurts the mob that hit its wearer.
    Thorns { module: Box<crate::thorns::Thorns>, condition: Condition },
    /// MobEffectModule.ArmorCounter: armor that may put a mob effect on the mob that hit its wearer.
    CounterEffect { module: Box<crate::weapon_effect::WeaponEffect>, condition: Condition },
    /// AttributeModule for a vanilla attribute: published as an attribute modifier on the stack for each listed slot.
    /// ToolActionTransformModule: a right click that turns the clicked block into its transformed state.
    ToolAction { action: crate::tool_actions::ToolAction, sound: String, require_ground: bool, condition: Condition },
    /// ConstantEnchantmentModule: the tool counts as carrying this enchantment at this level (published as the stack's enchantments).
    Enchantment { name: String, flat: f32, each: f32, condition: Condition },
    Protection { module: crate::protection::Protection, condition: Condition },
    /// An attribute module for `tconstruct:generic.protection_cap`, which the host has no attribute for: it raises the cap on summed protection.
    ProtectionCap { formula: StatFormula, condition: Condition },
    Attribute { attribute: String, operation: &'static str, formula: StatFormula, slots: Vec<String>, condition: Condition },
}
const CREATIVE_KEY: &str = "tconstruct:creative";
const OVERRIDE_BONUS: &str = "tconstruct:override_bonus";
const OVERRIDE_MULTIPLIER: &str = "tconstruct:override_multiplier";

/// SlotType.getIfPresent: a swappable recipe only stores a created slot type, so any stored name is one.
fn is_known_slot(name: &str) -> bool { !name.is_empty() }

/// ToolDataNBT.addSlots: ignores 0 and removes the key when the total returns to 0.
fn add_slots(data: &mut NbtCompound, name: &str, count: i32) {
    if count == 0 { return; }
    let total = nbt_int(data.get(name)).wrapping_add(count);
    if total == 0 { data.remove(name); } else { data.insert(name.to_owned(), Nbt::Int(total)); }
}

/// A ModifierFormula of either form. `Simple` is flat + each_level * level, which is the scaled
/// level handed to the module's fallback; `PostFix` runs a formula over named arguments.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum LevelFormula { Simple { flat: f32, each: f32 }, PostFix(Formula) }

impl LevelFormula {
    /// `names` are the argument names after the level: ["value","multiplier"] or ["factor"].
    fn decode(fields: &Map<String, Value>, names: &[&str]) -> Result<Option<Self>, String> {
        if fields.contains_key("variables") { return Ok(None); } // data-defined variables are not implemented
        if let Some(formula) = fields.get("formula") {
            let mut all = vec!["level"];
            all.extend_from_slice(names);
            return Ok(Some(Self::PostFix(Formula::decode(formula, &all)?)));
        }
        Ok(Some(Self::Simple { flat: float(fields, "flat")?, each: float(fields, "each_level")? }))
    }
    /// processLevel: the scaled level for simple formulas, the plain effective level for post fix.
    fn level(&self, entry: &Entry) -> f32 {
        match self { Self::Simple { flat, each } => flat + each * entry.effective_level(), Self::PostFix(_) => entry.effective_level() }
    }
}

/// ModifierPredicate over the modifier that caused tool damage (the subset the original ships).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum CausePredicate { Any, Tag(Identifier), Single(Identifier), Not(Box<CausePredicate>) }

impl CausePredicate {
    pub(crate) fn decode(value: Option<&Value>) -> Result<Option<Self>, String> {
        let Some(value) = value.filter(|value| !value.is_null()) else { return Ok(Some(Self::Any)); };
        let object = value.as_object().ok_or("cause must be an object")?;
        Self::typed(object, object.get("type").and_then(Value::as_str).ok_or("cause type missing")?)
    }
    fn typed(object: &Map<String, Value>, kind: &str) -> Result<Option<Self>, String> {
        Ok(Some(match kind {
            "tconstruct:tag" => Self::Tag(identifier(object, "tag")?),
            "tconstruct:single" => Self::Single(identifier(object, "modifier")?),
            "mantle:inverted" => {
                let inner = object.get("inverted_type").and_then(Value::as_str).ok_or("inverted_type missing")?;
                let Some(inner) = Self::typed(object, inner)? else { return Ok(None); };
                Self::Not(Box::new(inner))
            }
            _ => return Ok(None),
        }))
    }
    /// `cause` is None for ordinary tool use (ModifierId.EMPTY), which no tag or single match.
    pub(crate) fn matches(&self, cause: Option<&Identifier>, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        match self {
            Self::Any => true,
            Self::Tag(tag) => cause.is_some_and(|cause| in_tag(cause, tag)),
            Self::Single(modifier) => cause == Some(modifier),
            Self::Not(inner) => !inner.matches(cause, in_tag),
        }
    }
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
/// ModuleWithKey.FIELD: an optional persistent-data key, the modifier's ID when absent.
fn optional_key(fields: &Map<String, Value>) -> Result<Option<Identifier>, String> {
    match fields.get("key").filter(|value| !value.is_null()) { None => Ok(None), Some(_) => identifier(fields, "key").map(Some) }
}

/// A SlotType name: lowercase letters, digits and underscores (the original regex also allows empty).
fn slot_name(fields: &Map<String, Value>, key: &str) -> Result<String, String> {
    let name = fields.get(key).and_then(Value::as_str).ok_or_else(|| format!("{key} must be a slot name"))?;
    if !name.bytes().all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'_')) { return Err(format!("invalid modifier slot name {name}")); }
    Ok(name.to_owned())
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
        for key in ["tool", "requirement"] {
            let Some(predicate) = fields.get(key).filter(|value| !value.is_null()) else { continue; };
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
                // The older count field is the per-level amount when each_level is absent.
                let each = if fields.contains_key("each_level") { short(fields, "each_level")? } else { short(fields, "count")? };
                Self::Slot { name: name.to_owned(), flat: short(fields, "flat")?, each, condition: Condition::decode(fields)? }
            }
            "tconstruct:trait" if only("tconstruct:modifier_traits") => {
                let level = match fields.get("level").filter(|value| !value.is_null()) {
                    None => 1,
                    Some(Value::Number(number)) => crate::materials::number_int(&number.to_string()),
                    Some(_) => return Err("level must be a number".into()),
                };
                if level < 1 { return Err("trait level must be at least 1".into()); }
                let Some(Value::Bool(fixed_level)) = fields.get("fixed_level") else { return Err("fixed_level must be a boolean".into()); };
                Self::Trait { modifier: identifier(fields, "name")?, level, fixed_level: *fixed_level, condition: Condition::decode(fields)? }
            }
            // The tooltip binding is presentation; the original binds both by default.
            "tconstruct:reduce_tool_damage" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:tool_damage" | "tconstruct:tooltip"))
                && !fields.get("entity").is_some_and(|value| !value.is_null()) => {
                let Some(cause) = CausePredicate::decode(fields.get("cause"))? else { return Ok(None); };
                let Some(formula) = LevelFormula::decode(fields, &["value", "multiplier"])? else { return Ok(None); };
                Self::ReduceDamage { cause, formula, condition: Condition::decode(fields)? }
            }
            "tconstruct:requirements" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:validate_upgrade" | "tconstruct:requirements")) => {
                // A requirement with a predicate type this port lacks is unbound (the early check above).
                let requirement = Predicate::decode(fields.get("requirement").ok_or("requirement missing")?)?;
                let (min, max) = crate::modifiers::range(fields.get("modifier_level"))?;
                let translation_key = fields.get("translation_key").and_then(Value::as_str).ok_or("translation_key must be a string")?.to_owned();
                Self::Requirements { requirement, min, max, translation_key }
            }
            "tconstruct:repair" if only("tconstruct:repair_factor") => {
                let Some(formula) = LevelFormula::decode(fields, &["factor"])? else { return Ok(None); };
                Self::Repair { formula, condition: Condition::decode(fields)? }
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
            "tconstruct:set_stat" if only("tconstruct:tool_stats") => {
                // Only the harvest tier is a settable stat the shipped data uses; others stay unbound.
                if identifier(fields, "stat")?.to_string() != "tconstruct:harvest_tier" { return Ok(None); }
                let tier = identifier(fields, "value")?;
                if !matches!(tier.path(), "wood" | "gold" | "stone" | "iron" | "diamond" | "netherite") { return Err(format!("unknown tool tier {tier}")); }
                Self::SetTier { tier, condition: Condition::decode(fields)? }
            }
            // Display name is presentation and removal is run by the remove paths; neither changes this binding.
            "tconstruct:swappable_slot" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:volatile_data" | "tconstruct:display_name" | "tconstruct:remove")) => {
                Self::SwappableSlot { key: optional_key(fields)?, count: short(fields, "slots")?, condition: Condition::decode(fields)? }
            }
            "tconstruct:swappable_bonus_slot" if only("tconstruct:volatile_data") => Self::BonusSlot {
                key: optional_key(fields)?, matching: slot_name(fields, "match")?, bonus: slot_name(fields, "bonus")?,
                count: short(fields, "slots")?, condition: Condition::decode(fields)?,
            },
            "tconstruct:swappable_tool_traits" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:modifier_traits" | "tconstruct:display_name" | "tconstruct:remove")) => {
                let hook = identifier(fields, "hook")?;
                // ToolTraitHook.supportsHook: the main traits hook and the two alternates the definitions bind.
                if !matches!(hook.to_string().as_str(), "tconstruct:tool_traits" | "tconstruct:rebalanced_trait" | "tconstruct:trim_trait") { return Ok(None); }
                let matching = match fields.get("match") { None | Some(Value::Null) => String::new(), Some(Value::String(text)) => text.clone(), Some(_) => return Err("match must be a string".into()) };
                Self::SwappableTraits { key: optional_key(fields)?, matching, hook }
            }
            // The code-defined creative_slot modifier's module; it binds only volatile data and removal.
            "tconstruct:conditional_mining_speed" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:break_speed" | "tconstruct:tooltip")) => {
                match crate::mining_speed::MiningSpeed::decode(fields)? {
                    Some(module) => Self::MiningSpeed { module: Box::new(module), condition: Condition::decode(fields)? },
                    None => return Ok(None),
                }
            }
            "tconstruct:conditional_melee_damage" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:melee_damage" | "tconstruct:monster_melee_damage" | "tconstruct:tooltip")) => {
                match crate::melee_damage::MeleeDamage::decode(fields)? {
                    Some(module) => Self::MeleeDamage { module: Box::new(module), condition: Condition::decode(fields)? },
                    None => return Ok(None),
                }
            }
            "tconstruct:weapon_mob_effect" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:melee_hit" | "tconstruct:monster_melee_hit" | "tconstruct:projectile_hit")) => {
                match crate::weapon_effect::WeaponEffect::decode(fields)? {
                    Some(module) => Self::WeaponEffect { module: Box::new(module), condition: Condition::decode(fields)? },
                    None => return Ok(None),
                }
            }
            "tconstruct:block_damage" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:damage_block" | "tconstruct:tooltip")) => {
                let source = match fields.get("damage_source") { None | Some(Value::Null) => crate::protection::SourceCondition::Any, Some(value) => match crate::protection::SourceCondition::decode(value)? { Some(source) => source, None => return Ok(None) } };
                Self::BlockDamage { source, condition: Condition::decode(fields)? }
            }
            "tconstruct:effect_immunity" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:equipment_change")) => {
                Self::EffectImmunity { module: Box::new(crate::effect_immunity::EffectImmunity::decode(fields)?), condition: Condition::decode(fields)? }
            }
            "tconstruct:max_armor_attribute" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:equipment_change" | "tconstruct:tooltip")) => {
                match crate::max_armor::MaxArmorAttribute::decode(fields)? {
                    Some(module) => Self::MaxArmorAttribute { module: Box::new(module), condition: Condition::decode(fields)? },
                    None => return Ok(None),
                }
            }
            "tconstruct:armor_level" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:equipment_change")) => {
                Self::ArmorLevel { module: Box::new(crate::max_armor::ArmorLevel::decode(fields)?) }
            }
            "tconstruct:thorns" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:on_attacked")) => {
                match crate::thorns::Thorns::decode(fields)? {
                    Some(module) => Self::Thorns { module: Box::new(module), condition: Condition::decode(fields)? },
                    None => return Ok(None),
                }
            }
            "tconstruct:counter_mob_effect" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:on_attacked")) => {
                match crate::weapon_effect::WeaponEffect::decode_counter(fields)? {
                    Some(module) => Self::CounterEffect { module: Box::new(module), condition: Condition::decode(fields)? },
                    None => return Ok(None),
                }
            }
            "tconstruct:knockback" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:melee_hit" | "tconstruct:tooltip")) => {
                match crate::knockback::Knockback::decode(fields)? {
                    Some(module) => Self::Knockback { module: Box::new(module), condition: Condition::decode(fields)? },
                    None => return Ok(None),
                }
            }
            "tconstruct:tool_action_transform" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:block_interact" | "tconstruct:tool_action" | "tconstruct:aoe_highlight")) => {
                let Some(action) = crate::tool_actions::ToolAction::parse(fields.get("tool_action").and_then(Value::as_str).ok_or("tool_action missing")?) else { return Ok(None); };
                let sound = fields.get("sound").and_then(Value::as_str).ok_or("sound missing")?.to_owned();
                let require_ground = match fields.get("require_ground") { Some(Value::Bool(value)) => *value, None | Some(Value::Null) => false, Some(_) => return Err("require_ground must be boolean".into()) };
                Self::ToolAction { action, sound, require_ground, condition: Condition::decode(fields)? }
            }
            "tconstruct:constant_enchantment" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:enchantments" | "tconstruct:tooltip")) => {
                let name = identifier(fields, "name")?.to_string();
                // `level` is a plain number (a flat level) or a leveling value with flat and each_level parts.
                let (flat, each) = match fields.get("level") {
                    Some(Value::Number(number)) => (number.as_f64().ok_or("level must be a number")? as f32, 0.0),
                    Some(Value::Object(level)) => (float(level, "flat")?, float(level, "each_level")?),
                    _ => return Err("level must be a number or a leveling value".into()),
                };
                Self::Enchantment { name, flat, each, condition: Condition::decode(fields)? }
            }
            "tconstruct:protection" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:protection" | "tconstruct:tooltip")) => {
                let Some(module) = crate::protection::Protection::decode(fields)? else { return Ok(None); };
                Self::Protection { module, condition: Condition::decode(fields)? }
            }
            "tconstruct:attribute" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:attributes" | "tconstruct:equipment_change" | "tconstruct:tooltip")) => {
                // Only vanilla attributes exist in the host; a Tinkers or Forge attribute stays unbound.
                let name = fields.get("attribute").and_then(Value::as_str).ok_or("attribute missing")?;
                if name == "tconstruct:generic.protection_cap" && fields.get("operation").and_then(Value::as_str) == Some("addition") {
                    let Some(formula) = StatFormula::decode(fields, &["level", "value", "multiplier", "base"])? else { return Ok(None); };
                    return Ok(Some(Self::ProtectionCap { formula, condition: Condition::decode(fields)? }));
                }
                let Some(path) = name.strip_prefix("minecraft:") else { return Ok(None); };
                let path = path.strip_prefix("generic.").or_else(|| path.strip_prefix("player.")).unwrap_or(path);
                let operation = match fields.get("operation").and_then(Value::as_str) {
                    Some("addition") => "add_value", Some("multiply_base") => "add_multiplied_base", Some("multiply_total") => "add_multiplied_total",
                    _ => return Ok(None),
                };
                // The original's formula has only `level` by default; a variable naming an input this port lacks leaves it unbound.
                let Some(formula) = StatFormula::decode(fields, &["level", "value", "multiplier", "base"])? else { return Ok(None); };
                let slots = fields.get("slots").and_then(Value::as_array).ok_or("slots must be an array")?.iter()
                    .map(|slot| match slot.as_str() { Some(name @ ("mainhand" | "offhand" | "feet" | "legs" | "chest" | "head")) => Ok(name.to_owned()), _ => Err("unknown equipment slot".to_owned()) })
                    .collect::<Result<Vec<_>, _>>()?;
                Self::Attribute { attribute: format!("minecraft:{path}"), operation, formula, slots, condition: Condition::decode(fields)? }
            }
            "tconstruct:stat_override" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:tool_stats" | "tconstruct:remove")) => Self::StatOverride,
            "tconstruct:creative_slots" if hooks.iter().all(|bound| matches!(bound.to_string().as_str(), "tconstruct:volatile_data" | "tconstruct:remove")) => Self::CreativeSlots,
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
            Self::SwappableSlot { key, count, condition } => {
                if condition.matches(entry, facts) {
                    let name = (facts.persistent_string)(key.as_ref().unwrap_or(&entry.id));
                    if is_known_slot(&name) { add_slots(data, &name, *count); }
                }
            }
            Self::BonusSlot { key, matching, bonus, count, condition } => {
                if condition.matches(entry, facts) {
                    let name = (facts.persistent_string)(key.as_ref().unwrap_or(&entry.id));
                    if !name.is_empty() && *matching == name { add_slots(data, bonus, *count); }
                }
            }
            Self::StatOverride | Self::MiningSpeed { .. } | Self::MeleeDamage { .. } | Self::Knockback { .. } | Self::WeaponEffect { .. } | Self::CounterEffect { .. } | Self::Thorns { .. } | Self::EffectImmunity { .. } | Self::MaxArmorAttribute { .. } | Self::ArmorLevel { .. } | Self::BlockDamage { .. } | Self::Attribute { .. } | Self::ToolAction { .. } | Self::Enchantment { .. } | Self::Protection { .. } | Self::ProtectionCap { .. } => {}
            Self::CreativeSlots => {
                // addSlots for each stored slot type that exists (a valid slot name here).
                for (name, count) in (facts.persistent_numbers)(&Identifier::parse(CREATIVE_KEY).expect("valid key")) {
                    if is_known_slot(&name) { add_slots(data, &name, count as i32); }
                }
            }
            Self::SwappableTraits { .. } | Self::Boost { .. } | Self::SetTier { .. } | Self::Trait { .. } | Self::ReduceDamage { .. } | Self::Repair { .. } | Self::Requirements { .. } => {}
        }
    }

    /// ModifierRemovalHook.onRemoved for a modifier that left the tool: a swappable slot forgets its stored type.
    pub fn on_removed(&self, modifier: &Identifier, persistent: &mut NbtCompound) {
        match self {
            Self::SwappableSlot { key, .. } => { persistent.remove(&key.as_ref().unwrap_or(modifier).to_string()); }
            // Only a module that matches a stored value owns it.
            Self::SwappableTraits { key, matching, .. } if !matching.is_empty() => { persistent.remove(&key.as_ref().unwrap_or(modifier).to_string()); }
            Self::CreativeSlots => { persistent.remove(CREATIVE_KEY); }
            Self::StatOverride => { persistent.remove(OVERRIDE_BONUS); persistent.remove(OVERRIDE_MULTIPLIER); }
            _ => {}
        }
    }

    /// BreakSpeedModifierHook for a conditional mining speed module: the speed after it, with its priority
    /// (percent modules run after the additive ones, at 75).
    pub fn mining_speed(&self, entry: &Entry, facts: &Facts, tool: &crate::mining_speed::ToolView, context: &crate::mining_speed::Context, speed: f32) -> f32 {
        let Self::MiningSpeed { module, condition } = self else { return speed; };
        module.apply(entry, condition.matches(entry, facts), tool, context, speed)
    }
    /// MeleeDamageModifierHook for a conditional melee damage module: the damage after it.
    pub fn melee_damage(&self, entry: &Entry, facts: &Facts, tool: &crate::mining_speed::ToolView, base: f32, damage: f32, target_in_tag: &dyn Fn(&Identifier) -> bool) -> f32 {
        let Self::MeleeDamage { module, condition } = self else { return damage; };
        module.apply(entry, condition.matches(entry, facts), tool, base, damage, target_in_tag)
    }
    /// MeleeHitModifierHook.beforeMeleeHit for a knockback module: the strength after it (a melee hit, never a projectile's).
    pub fn knockback(&self, entry: &Entry, facts: &Facts, tool: &crate::mining_speed::ToolView, strength: f32, target_in_tag: &dyn Fn(&Identifier) -> bool) -> f32 {
        let Self::Knockback { module, condition } = self else { return strength; };
        module.apply(entry, condition.matches(entry, facts), tool, strength, false, target_in_tag)
    }
    /// The effect a weapon effect module puts on the target of a melee hit, if it lands (`rolls` are uniform values in [0, 1)).
    pub fn weapon_effect(&self, entry: &Entry, facts: &Facts, in_tag: &dyn Fn(&Identifier) -> bool, rolls: &mut dyn FnMut() -> f32) -> Option<(bool, crate::weapon_effect::Grant)> {
        let Self::WeaponEffect { module, condition } = self else { return None; };
        module.grant(entry, condition.matches(entry, facts), in_tag, rolls).map(|grant| (module.before_melee(), grant))
    }
    /// The effect an armor counter module puts on the mob that hit the wearer, if it lands (`rolls` are uniform values in [0, 1)).
    pub fn counter_effect(&self, entry: &Entry, facts: &Facts, in_tag: &dyn Fn(&Identifier) -> bool, rolls: &mut dyn FnMut() -> f32) -> Option<(crate::weapon_effect::Whom, crate::weapon_effect::Grant)> {
        let Self::CounterEffect { module, condition } = self else { return None; };
        module.grant(entry, condition.matches(entry, facts), in_tag, rolls).map(|grant| (module.whom(), grant))
    }
    /// The damage a thorns module deals to the mob that hit the wearer, if its chance lands (`rolls` are uniform values in [0, 1)).
    pub fn thorns(&self, entry: &Entry, facts: &Facts, in_tag: &dyn Fn(&Identifier) -> bool, rolls: &mut dyn FnMut() -> f32) -> Option<crate::thorns::Hurt> {
        let Self::Thorns { module, condition } = self else { return None; };
        module.hurt(entry, condition.matches(entry, facts), in_tag, rolls)
    }
    /// Whether a block damage module blocks damage from `source`, when its conditions hold.
    pub fn blocks_damage(&self, entry: &Entry, facts: &Facts, source: &crate::protection::DamageSource) -> bool {
        let Self::BlockDamage { source: blocked, condition } = self else { return false; };
        condition.matches(entry, facts) && blocked.matches(source)
    }
    /// Whether this is a block damage module.
    pub fn is_block_damage(&self) -> bool { matches!(self, Self::BlockDamage { .. }) }
    /// The effect an immunity module makes the wearer immune to, with the count it adds, when its conditions hold.
    pub fn effect_immunity(&self, entry: &Entry, facts: &Facts) -> Option<(Identifier, i32)> {
        let Self::EffectImmunity { module, condition } = self else { return None; };
        module.immunity(entry, condition.matches(entry, facts))
    }
    /// The claim a worn piece makes on an attribute through a max armor attribute module, when its conditions hold.
    pub fn max_armor_candidate(&self, entry: &Entry, facts: &Facts, broken: bool) -> Option<crate::max_armor::Candidate> {
        let Self::MaxArmorAttribute { module, condition } = self else { return None; };
        module.candidate(entry, condition.matches(entry, facts), broken)
    }
    /// The key and levels an armor level module adds for a worn piece.
    pub fn armor_level(&self, entry: &Entry, broken: bool) -> Option<(Identifier, i32)> {
        let Self::ArmorLevel { module } = self else { return None; };
        module.level(entry, broken)
    }
    /// Whether this is a max armor attribute module.
    pub fn is_max_armor_attribute(&self) -> bool { matches!(self, Self::MaxArmorAttribute { .. }) }
    /// Whether this is an armor level module.
    pub fn is_armor_level(&self) -> bool { matches!(self, Self::ArmorLevel { .. }) }
    /// Whether this is an effect immunity module.
    pub fn is_effect_immunity(&self) -> bool { matches!(self, Self::EffectImmunity { .. }) }
    /// Whether this is a thorns module.
    pub fn is_thorns(&self) -> bool { matches!(self, Self::Thorns { .. }) }
    /// Whether this is an armor counter effect module.
    pub fn is_counter_effect(&self) -> bool { matches!(self, Self::CounterEffect { .. }) }
    /// Whether this is a weapon effect module.
    pub fn is_weapon_effect(&self) -> bool { matches!(self, Self::WeaponEffect { .. }) }
    /// Whether this is a knockback module.
    pub fn is_knockback(&self) -> bool { matches!(self, Self::Knockback { .. }) }
    /// What this module adds to the protection cap attribute (0.05 is one point and a quarter of protection).
    pub fn protection_cap(&self, entry: &Entry, facts: &Facts, nbt: &NbtCompound) -> f32 {
        let Self::ProtectionCap { formula, condition } = self else { return 0.0; };
        if !condition.matches(entry, facts) { return 0.0; }
        formula.eval(entry, &crate::mining_speed::ToolView { nbt }, 0.0, 1.0, 0.0)
    }
    /// ProtectionModifierHook: the running protection after this module, over a damage source.
    pub fn protection(&self, entry: &Entry, facts: &Facts, nbt: &NbtCompound, source: &crate::protection::DamageSource, value: f32) -> f32 {
        let Self::Protection { module, condition } = self else { return value; };
        if !condition.matches(entry, facts) || !module.source().matches(source) || !module.attacker_matches(source) { return value; }
        module.formula().eval(entry, &crate::mining_speed::ToolView { nbt }, value, 1.0, 0.0)
    }
    /// The enchantment this module gives the tool and its level, when its conditions hold and the level is positive.
    pub fn enchantment(&self, entry: &Entry, facts: &Facts) -> Option<(String, i32)> {
        let Self::Enchantment { name, flat, each, condition } = self else { return None; };
        let level = (flat + each * entry.effective_level()) as i32;
        (level > 0 && condition.matches(entry, facts)).then(|| (name.clone(), level))
    }
    /// The tool action this module performs on a right click, with its sound and whether the block above must be clear.
    pub fn tool_action(&self, entry: &Entry, facts: &Facts) -> Option<(crate::tool_actions::ToolAction, String, bool)> {
        let Self::ToolAction { action, sound, require_ground, condition } = self else { return None; };
        condition.matches(entry, facts).then(|| (*action, sound.clone(), *require_ground))
    }
    /// AttributesModifierHook for an attribute module: one attribute modifier entry per listed slot among `wanted`.
    pub fn attribute_entries(&self, entry: &Entry, facts: &Facts, nbt: &NbtCompound, wanted: &[&str]) -> Vec<NbtCompound> {
        let Self::Attribute { attribute, operation, formula, slots, condition } = self else { return Vec::new(); };
        if !condition.matches(entry, facts) { return Vec::new(); }
        let amount = f64::from(formula.eval(entry, &crate::mining_speed::ToolView { nbt }, 0.0, 1.0, 0.0));
        let short = attribute.rsplit(':').next().unwrap_or(attribute);
        slots.iter().filter(|slot| wanted.contains(&slot.as_str())).map(|slot| {
            let mut modifier = NbtCompound::new();
            modifier.insert("type".into(), Nbt::String(attribute.clone()));
            modifier.insert("id".into(), Nbt::String(format!("{}/{short}/{slot}", entry.id)));
            modifier.insert("amount".into(), Nbt::Double(amount));
            modifier.insert("operation".into(), Nbt::String((*operation).to_owned()));
            modifier.insert("slot".into(), Nbt::String(slot.clone()));
            modifier
        }).collect()
    }
    /// Whether this is a percent melee damage module (None for other modules).
    pub fn melee_percent(&self) -> Option<bool> {
        match self { Self::MeleeDamage { module, .. } => Some(module.percent()), _ => None }
    }
    /// The module's priority override, if it is a mining speed module.
    pub fn mining_percent(&self) -> Option<bool> {
        match self { Self::MiningSpeed { module, .. } => Some(module.percent()), _ => None }
    }

    /// ModifierTraitModule.addTraits: the entry this module adds for `entry`, if any. A fixed-level
    /// trait joins once (the first time its owner is met); otherwise its level scales with the owner's.
    pub fn trait_entry(&self, entry: &Entry, facts: &Facts, first_encounter: bool) -> Option<Entry> {
        let Self::Trait { modifier, level, fixed_level, condition } = self else { return None; };
        if !condition.matches(entry, facts) { return None; }
        if *fixed_level { return first_encounter.then(|| Entry::new(modifier.clone(), *level)); }
        Some(Entry::new(modifier.clone(), if entry.level == 1 { *level } else { level.wrapping_mul(entry.level) }))
    }
    /// SwappableToolTraitsModule.addTraits: the alternate hook's entries, added at their own levels.
    pub fn swappable_traits(&self, entry: &Entry, facts: &Facts) -> Vec<Entry> {
        let Self::SwappableTraits { key, matching, hook } = self else { return Vec::new(); };
        if !matching.is_empty() && *matching != (facts.persistent_string)(key.as_ref().unwrap_or(&entry.id)) { return Vec::new(); }
        (facts.hook_traits)(hook).into_iter().map(|trait_entry| Entry::new(trait_entry.id, trait_entry.level)).collect()
    }
    pub fn is_trait(&self) -> bool { matches!(self, Self::Trait { .. } | Self::SwappableTraits { .. }) }

    /// ValidateModifierHook (VALIDATE_UPGRADE): the translation key of the error when this
    /// modifier's level is in range and the tool does not meet the requirement.
    pub fn validate_upgrade(&self, entry: &Entry, facts: &Facts) -> Option<&str> {
        let Self::Requirements { requirement, min, max, translation_key } = self else { return None; };
        (*min <= entry.level && entry.level <= *max && !requirement.matches(facts)).then_some(translation_key.as_str())
    }

    /// ToolDamageModifierHook.beforeDamageTool: the amount left after this module's probabilistic
    /// reduction. `random` yields uniform values in [0, 1).
    pub fn reduce_damage(&self, entry: &Entry, facts: &Facts, cause: Option<&Identifier>,
        modifier_in_tag: &dyn Fn(&Identifier, &Identifier) -> bool, amount: i32, random: &mut dyn FnMut() -> f32) -> i32 {
        let Self::ReduceDamage { cause: wanted, formula, condition } = self else { return amount; };
        if !condition.matches(entry, facts) || !wanted.matches(cause, modifier_in_tag) { return amount; }
        // ConditionalStatFormula.apply(.., value 0, multiplier 1); a simple formula's IDENTITY is the scaled level.
        let percentage = match formula {
            LevelFormula::Simple { .. } => formula.level(entry),
            LevelFormula::PostFix(post_fix) => match post_fix.apply(&[formula.level(entry), 0.0, 1.0]) { Ok(value) => value, Err(_) => return amount },
        };
        if percentage >= 1.0 { return 0; }
        if percentage <= 0.0 { return amount; }
        // Each point of damage is cancelled with the given probability.
        (0..amount.max(0)).filter(|_| random() >= percentage).count() as i32
    }

    /// RepairFactorModifierHook.getRepairFactor for this module.
    pub fn repair_factor(&self, entry: &Entry, facts: &Facts, factor: f32) -> f32 {
        let Self::Repair { formula, condition } = self else { return factor; };
        if !condition.matches(entry, facts) { return factor; }
        match formula {
            // FallbackFormula.PERCENT: value * (1 + level), where the value argument is the factor.
            LevelFormula::Simple { .. } => factor * (1.0 + formula.level(entry)),
            LevelFormula::PostFix(post_fix) => post_fix.apply(&[formula.level(entry), factor]).unwrap_or(factor),
        }
    }

    /// The tool-stats hook.
    pub fn apply_stats(&self, entry: &Entry, facts: &Facts, builder: &mut Builder) {
        if let Self::SetTier { tier, condition } = self {
            // ToolTierStat.supports: harvest tools only (no item context means no filter).
            let harvest = facts.item.is_none() || (facts.in_tag)(facts.item.expect("checked"), &Identifier::new("tconstruct", "modifiable/harvest").expect("valid tag"));
            if condition.matches(entry, facts) && harvest { builder.update_tier(tier); }
            return;
        }
        if let Self::StatOverride = self {
            let parse = |name: &str| Identifier::parse(name).ok().and_then(|id| Stat::from_id(&id));
            for (name, value) in (facts.persistent_numbers)(&Identifier::parse(OVERRIDE_BONUS).expect("valid key")) {
                if let Some(stat) = parse(&name) { builder.update(stat, value as f32); }
            }
            for (name, value) in (facts.persistent_numbers)(&Identifier::parse(OVERRIDE_MULTIPLIER).expect("valid key")) {
                if let Some(stat) = parse(&name) { builder.multiply(stat, f64::from(value as f32)); }
            }
            return;
        }
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
        Facts { item, in_tag, level: &|_, _| 0, tag_level: &|_, _| 0, has_persistent_key: &|_| false, persistent_string: &|_| String::new(), persistent_numbers: &|_| Vec::new(), hook_traits: &|_| Vec::new() }
    }
    #[test]
    fn swappable_slots_follow_the_stored_slot_type_and_clear_on_removal() {
        let in_tag = |_: &Identifier, _: &Identifier| true;
        let stored = std::cell::RefCell::new(std::collections::BTreeMap::new());
        stored.borrow_mut().insert("test:swap".to_owned(), "abilities".to_owned());
        let lookup = |key: &Identifier| stored.borrow().get(&key.to_string()).cloned().unwrap_or_default();
        let f = Facts { item: None, in_tag: &in_tag, level: &|_, _| 0, tag_level: &|_, _| 0, has_persistent_key: &|_| false, persistent_string: &lookup, persistent_numbers: &|_| Vec::new(), hook_traits: &|_| Vec::new() };
        let slot = decode(json!({"type":"tconstruct:swappable_slot","slots":1})).unwrap();
        let bonus = decode(json!({"type":"tconstruct:swappable_bonus_slot","match":"abilities","bonus":"upgrades","slots":-1})).unwrap();
        let other = decode(json!({"type":"tconstruct:swappable_bonus_slot","match":"defense","bonus":"upgrades","slots":-1})).unwrap();
        let entry = Entry::new(id("test:swap"), 1);
        let mut data = NbtCompound::new();
        for module in [&slot, &bonus, &other] { module.apply_volatile(&entry, &f, &mut data); }
        assert_eq!(data["abilities"], Nbt::Int(1));
        assert_eq!(data["upgrades"], Nbt::Int(-1), "only the module whose match is stored applies");
        // An explicit key reads another persistent entry; an absent value grants nothing.
        let keyed = decode(json!({"type":"tconstruct:swappable_slot","key":"test:other","slots":2})).unwrap();
        let mut none = NbtCompound::new();
        keyed.apply_volatile(&entry, &f, &mut none);
        assert!(none.is_empty());
        // A bonus that cancels the slot's own type removes the key instead of storing zero.
        let cancel = decode(json!({"type":"tconstruct:swappable_bonus_slot","match":"abilities","bonus":"abilities","slots":-1})).unwrap();
        cancel.apply_volatile(&entry, &f, &mut data);
        assert!(!data.contains_key("abilities"));
        // Only the slot module forgets its stored type when the modifier leaves the tool.
        let mut persistent = NbtCompound::new();
        persistent.insert("test:swap".into(), Nbt::String("abilities".into()));
        bonus.on_removed(&id("test:swap"), &mut persistent);
        assert!(persistent.contains_key("test:swap"));
        slot.on_removed(&id("test:swap"), &mut persistent);
        assert!(persistent.is_empty());
        // Hook bindings this port does not run are unbound; display-name and remove are allowed with volatile data.
        assert!(decode(json!({"type":"tconstruct:swappable_slot","slots":1,"hooks":["tconstruct:tooltip"]})).is_none());
        assert!(decode(json!({"type":"tconstruct:swappable_slot","slots":1,"hooks":["tconstruct:volatile_data"]})).is_some());
    }
    #[test]
    fn swappable_tool_traits_add_the_hook_traits_only_for_the_stored_value() {
        let in_tag = |_: &Identifier, _: &Identifier| true;
        let stored = |key: &Identifier| if key.to_string() == "test:rebalanced" { "traits".to_owned() } else { String::new() };
        let hook_traits = |hook: &Identifier| if hook.to_string() == "tconstruct:rebalanced_trait" {
            vec![crate::material_traits::ModifierEntry { id: id("test:extra"), level: 2, optional: false }]
        } else { Vec::new() };
        let f = Facts { item: None, in_tag: &in_tag, level: &|_, _| 0, tag_level: &|_, _| 0, has_persistent_key: &|_| false, persistent_string: &stored, persistent_numbers: &|_| Vec::new(), hook_traits: &hook_traits };
        let entry = Entry::new(id("test:rebalanced"), 1);
        let matching = decode(json!({"type":"tconstruct:swappable_tool_traits","hook":"tconstruct:rebalanced_trait","match":"traits"})).unwrap();
        let other_value = decode(json!({"type":"tconstruct:swappable_tool_traits","hook":"tconstruct:rebalanced_trait","match":"other"})).unwrap();
        let always = decode(json!({"type":"tconstruct:swappable_tool_traits","hook":"tconstruct:rebalanced_trait"})).unwrap();
        let wrong_hook = decode(json!({"type":"tconstruct:swappable_tool_traits","hook":"tconstruct:trim_trait"})).unwrap();
        let levels = |module: &Module| module.swappable_traits(&entry, &f).iter().map(|added| (added.id.to_string(), added.level)).collect::<Vec<_>>();
        assert_eq!(levels(&matching), vec![("test:extra".to_owned(), 2)]);
        assert!(levels(&other_value).is_empty());
        assert_eq!(levels(&always), vec![("test:extra".to_owned(), 2)], "an empty match applies whatever is stored");
        assert!(levels(&wrong_hook).is_empty());
        assert!(decode(json!({"type":"tconstruct:swappable_tool_traits","hook":"tconstruct:unknown_hook"})).is_none());
        // Only a module with a match owns the stored value and clears it on removal.
        let mut persistent = NbtCompound::new();
        persistent.insert("test:rebalanced".into(), Nbt::String("traits".into()));
        always.on_removed(&id("test:rebalanced"), &mut persistent);
        assert!(persistent.contains_key("test:rebalanced"));
        matching.on_removed(&id("test:rebalanced"), &mut persistent);
        assert!(persistent.is_empty());
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
        // modifier_slot: the older count field is the per-level amount; a total of zero removes the key.
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
    fn damage_reduction_is_probabilistic_per_point_and_repair_scales_the_factor() {
        let in_tag = |_: &Identifier, _: &Identifier| false;
        let f = facts(None, &in_tag);
        let entry = Entry::new(id("test:m"), 2);
        let tags = |cause: &Identifier, tag: &Identifier| cause.path() == "bypasser" && tag.path() == "bypass";
        // Reinforced-style: 0.025 * level * (11 - level) = 0.45 at level 2; each damage point is cancelled when the roll is below it.
        let reduce = decode(json!({"type":"tconstruct:reduce_tool_damage","formula":[0.025,"$level","*",11.0,"$level","-","*"]})).unwrap();
        let mut rolls = [0.1_f32, 0.5, 0.44, 0.9].into_iter();
        let dealt = reduce.reduce_damage(&entry, &f, None, &tags, 4, &mut || rolls.next().unwrap());
        assert_eq!(dealt, 2, "rolls 0.5 and 0.9 are not below 0.45, so two points land");
        // 100% blocks everything without rolling; 0% blocks nothing without rolling.
        let all = decode(json!({"type":"tconstruct:reduce_tool_damage","flat":1.0})).unwrap();
        assert_eq!(all.reduce_damage(&entry, &f, None, &tags, 7, &mut || panic!("no roll expected")), 0);
        let none = decode(json!({"type":"tconstruct:reduce_tool_damage","flat":0.0,"each_level":0.0})).unwrap();
        assert_eq!(none.reduce_damage(&entry, &f, None, &tags, 7, &mut || panic!("no roll expected")), 7);
        // A cause predicate: this reduction ignores damage caused by a bypassing modifier.
        let guarded = decode(json!({"type":"tconstruct:reduce_tool_damage","flat":1.0,
            "cause":{"type":"mantle:inverted","inverted_type":"tconstruct:tag","tag":"test:bypass"}})).unwrap();
        assert_eq!(guarded.reduce_damage(&entry, &f, None, &tags, 3, &mut || 0.0), 0, "ordinary use is reduced");
        assert_eq!(guarded.reduce_damage(&entry, &f, Some(&id("test:bypasser")), &tags, 3, &mut || 0.0), 3, "a bypassing cause is not");
        // Repair: simple form is factor * (1 + scaled level); post fix reads the effective level and the factor.
        let preserved = decode(json!({"type":"tconstruct:repair","each_level":0.15})).unwrap();
        let Some(Module::Repair { .. }) = Some(&preserved) else { panic!() };
        assert!((preserved.repair_factor(&entry, &f, 10.0) - 10.0 * (1.0 + 0.3)).abs() < 1e-5);
        let dense = decode(json!({"type":"tconstruct:repair","formula":["$factor",1.0,0.025,"$level","*",11.0,"$level","-","*","-","*"],"modifier_level":{"max":5}})).unwrap();
        assert!((dense.repair_factor(&entry, &f, 10.0) - 10.0 * (1.0 - 0.025 * 2.0 * 9.0)).abs() < 1e-5);
        assert_eq!(dense.repair_factor(&Entry::new(id("test:m"), 6), &f, 10.0), 10.0, "outside the level range nothing changes");
        // Unsupported shapes are unbound: an entity predicate or data-defined variables.
        assert!(decode(json!({"type":"tconstruct:reduce_tool_damage","flat":1.0,"entity":{"type":"tconstruct:sneaking"}})).is_none());
        assert!(decode(json!({"type":"tconstruct:reduce_tool_damage","formula":["$light"],"variables":{"light":{"type":"tconstruct:entity"}}})).is_none());
        assert!(decode(json!({"type":"tconstruct:reduce_tool_damage","flat":1.0,"cause":{"type":"mantle:and"}})).is_none());
    }
    #[test]
    fn unrepresentable_shapes_are_unbound_and_malformed_modules_reject() {
        assert!(decode(json!({"type":"tconstruct:stat_boost","stat":"tconstruct:tank_capacity","operation":"add","flat":1})).is_none(), "tank stats belong to another domain");
        assert!(decode(json!({"type":"tconstruct:volatile_flag","flag":"a:b","hooks":["tconstruct:projectile_launch"]})).is_none());
        assert!(decode(json!({"type":"tconstruct:stat_boost","stat":"tconstruct:durability","operation":"add","flat":1,"hooks":["tconstruct:volatile_data"]})).is_none());
        assert!(decode(json!({"type":"tconstruct:conditional_melee_damage","attacker":"mantle:sprinting","flat":7.0})).is_some(), "an attacker state condition is bound");
        assert!(decode(json!({"type":"tconstruct:conditional_melee_damage","target":"mantle:on_fire","each_level":2.0})).is_some(), "a target state condition is bound");
        assert!(decode(json!({"type":"tconstruct:conditional_melee_damage","attacker":{"type":"mantle:in_biome","biome":"minecraft:plains"},"flat":7.0})).is_none(), "an attacker condition on the biome stays unbound");
        assert!(decode(json!({"type":"tconstruct:conditional_melee_damage","target":{"type":"mantle:mob_type","mobs":"minecraft:undead"},"each_level":2.0})).is_some(), "a mob type target is bound");
        assert!(decode(json!({"type":"tconstruct:conditional_melee_damage","each_level":2.0})).is_some(), "a condition-free melee module is bound");
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
