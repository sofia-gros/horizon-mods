//! Global loot modifiers (`mantle:add_entry`, `mantle:replace_item` and the original's own `tconstruct:modifier_hook`), from
//! Mantle's AddEntryLootModifier and ReplaceItemLootModifier and the original's GlobalLootModifiersProvider (MIT; see LICENSE):
//! extra rules that run over the stacks a block, a mob or a use is about to drop, in the order the global list
//! (`data/forge/loot_modifiers/global_loot_modifiers.json`, imported with the modifiers) names them. Forge applies only the
//! listed modifiers, in list order, so the list is mirrored here as ORDER and the domain tests hold it against the imported
//! list file. The one listed modifier with no work of its own here is `tconstruct:modifier_hook`: in the original it runs the
//! tools' loot modules (severing, the mining capacity), which this port runs as their own loot event callbacks instead, so the
//! file is decoded (a malformed one is a load problem) and then left alone.
//!
//! The evaluation context is plain data and closures the loot hook fills in, so the engine itself stays testable: the loot
//! event names no block state, so whether the broken block is in a tag is answered by the hook from the break it saw, and the
//! chrysophilite value is the breaker's attribute as the hook computes it from the worn armor (the module that would publish
//! the attribute is not bound, for want of the host's attribute data).
use pluto_lib::{Identifier, ItemSlot, ResourceManagerHelper, ResourceType, ServerLifecycleEvents};
use serde_json::Value;
use std::sync::{Arc, RwLock};

pub(crate) const FOLDER: &str = "loot_modifiers";
const ADD_ENTRY: &str = "mantle:add_entry";
const REPLACE_ITEM: &str = "mantle:replace_item";
const MODIFIER_HOOK: &str = "tconstruct:modifier_hook";

/// The global list's entries, in order: the modifiers run in this order, and a modifier file the list does not name never
/// runs. Mirrors the imported `global_loot_modifiers.json`; the domain tests compare the two.
pub(crate) const ORDER: &[&str] = &[
    "tconstruct:lustrous/platinum",
    "tconstruct:wither_bone",
    "tconstruct:lustrous/tungsten",
    "tconstruct:lustrous/nickel",
    "tconstruct:modifier_hook",
    "tconstruct:lustrous/uranium",
    "tconstruct:lustrous/iron",
    "tconstruct:lustrous/copper",
    "tconstruct:lustrous/aluminum",
    "tconstruct:lustrous/chromium",
    "tconstruct:lustrous/tin",
    "tconstruct:lustrous/cadmium",
    "tconstruct:lustrous/netherite_scrap",
    "tconstruct:lustrous/lead",
    "tconstruct:lustrous/zinc",
    "tconstruct:tasty_bacon",
    "tconstruct:lustrous/silver",
    "tconstruct:lustrous/cobalt",
    "tconstruct:lustrous/gold",
    "tconstruct:chrysophilite_modifier",
    "tconstruct:lustrous/osmium",
];

/// What a modifier is tested and run against: the drops' own facts, plus the lookups only the game side can answer.
pub(crate) struct Context<'a> {
    /// The loot table that rolled (`minecraft:entities/wither_skeleton`); empty when none did.
    pub table: &'a str,
    /// The drops came from a block (a break, or a use on one).
    pub has_block: bool,
    /// The drops came from an entity (a death, or a use on one).
    pub has_entity: bool,
    /// The entity type the drops came from, when one did.
    pub entity: Option<&'a Identifier>,
    /// The broken block is in the block tag (Mantle's BlockTagLootCondition).
    pub block_in_tag: &'a dyn Fn(&Identifier) -> bool,
    /// The entity type is in the entity type tag.
    pub entity_in_tag: &'a dyn Fn(&Identifier) -> bool,
    /// An item is in the item tag.
    pub item_in_tag: &'a dyn Fn(&Identifier, &Identifier) -> bool,
    /// The item tag holds at least one item (Mantle's TagFilledLootCondition).
    pub tag_filled: &'a dyn Fn(&Identifier) -> bool,
    /// The item a tag prefers (Mantle's TagPreference): the first of its items the game has, when it has one.
    pub tag_preference: &'a dyn Fn(&Identifier) -> Option<Identifier>,
    /// The level of a modifier on the tool that made the drops, as HasModifierLootCondition reads it: 0 unless the tool is a
    /// modifiable item carrying the modifier.
    pub tool_level: &'a dyn Fn(&Identifier) -> i32,
    /// The looting level the tool publishes, which the looting function grows the count by.
    pub tool_looting: i32,
    /// The breaker's chrysophilite attribute value; 0 when no worn piece carries the modifier.
    pub chrysophilite: f32,
    /// A config condition property (`wither_bone_drop`), when the config has loaded.
    pub config: &'a dyn Fn(&str) -> Option<bool>,
    /// The radius of the explosion that broke the block, when one did.
    pub explosion: Option<f32>,
}

/// An item or an item tag, as the data writes an ingredient.
#[derive(Clone, Debug, PartialEq)]
enum Ingredient {
    Item(Identifier),
    Tag(Identifier),
    Any(Vec<Ingredient>),
}

impl Ingredient {
    fn decode(value: &Value) -> Result<Self, String> {
        if let Some(text) = value.as_str() {
            return Ok(Self::Item(identifier(text)?));
        }
        if let Some(list) = value.as_array() {
            return Ok(Self::Any(list.iter().map(Self::decode).collect::<Result<_, _>>()?));
        }
        let object = value.as_object().ok_or("an ingredient must be an item, a tag or a list of them")?;
        if let Some(item) = object.get("item") {
            return Ok(Self::Item(identifier(item.as_str().ok_or("an ingredient item must be a name")?)?));
        }
        if let Some(tag) = object.get("tag") {
            return Ok(Self::Tag(identifier(tag.as_str().ok_or("an ingredient tag must be a name")?)?));
        }
        Err("an ingredient needs an item or a tag".into())
    }

    fn matches(&self, item: &Identifier, context: &Context) -> bool {
        match self {
            Self::Item(wanted) => item == wanted,
            Self::Tag(tag) => (context.item_in_tag)(item, tag),
            Self::Any(list) => list.iter().any(|ingredient| ingredient.matches(item, context)),
        }
    }
}

/// A condition on the drops themselves, tested before a modifier runs. In the data the kind is the `condition` field.
#[derive(Clone, Debug, PartialEq)]
enum Condition {
    /// `mantle:block_tag`: the broken block is in the block tag.
    BlockTag(Identifier),
    /// `mantle:tag_filled`: the item tag holds at least one item.
    TagFilled(Identifier),
    /// `tconstruct:has_modifier`: the tool carries the modifier (HasModifierLootCondition).
    HasModifier(Identifier),
    /// `tconstruct:has_chrysophilite`: the breaker's chrysophilite attribute is at least 1 (ChrysophiliteLootCondition).
    HasChrysophilite,
    /// `tconstruct:has_context_set`: the named loot context set's parameters are all present (HasLootContextSetCondition).
    /// The loot event distinguishes only the block and entity sets; any other set's parameters cannot be present in it.
    ContextSet(String),
    /// `minecraft:entity_properties` on `this`, with the predicate's type test: an entity type tag, a type, or no test.
    EntityProperties(EntityTest),
    /// `tconstruct:block_or_entity`: the drops came from a block or an entity.
    BlockOrEntity,
    /// `forge:loot_table_id`: the loot table that rolled.
    LootTableId(Identifier),
    /// `tconstruct:config`: a boolean of the Tinkers config.
    Config(String),
}

#[derive(Clone, Debug, PartialEq)]
enum EntityTest {
    Any,
    Type(Identifier),
    Tag(Identifier),
}

impl Condition {
    fn decode(value: &Value) -> Result<Self, String> {
        let object = value.as_object().ok_or("a loot condition must be an object")?;
        let kind = object
            .get("condition")
            .or_else(|| object.get("type"))
            .and_then(Value::as_str)
            .ok_or("a loot condition needs its condition kind")?;
        let field = |name: &str| object.get(name).and_then(Value::as_str).ok_or_else(|| format!("a {kind} condition needs {name}"));
        Ok(match kind {
            "mantle:block_tag" => Self::BlockTag(identifier(field("tag")?)?),
            "mantle:tag_filled" => Self::TagFilled(identifier(field("tag")?)?),
            "tconstruct:has_modifier" => Self::HasModifier(identifier(field("modifier")?)?),
            "tconstruct:has_chrysophilite" => Self::HasChrysophilite,
            "tconstruct:has_context_set" => Self::ContextSet(field("set")?.to_owned()),
            "tconstruct:block_or_entity" => Self::BlockOrEntity,
            "forge:loot_table_id" => Self::LootTableId(identifier(field("loot_table_id")?)?),
            "tconstruct:config" => {
                let prop = field("prop")?.to_lowercase();
                if !crate::config::known(&prop) {
                    return Err(format!("unknown config property {prop}"));
                }
                Self::Config(prop)
            }
            "minecraft:entity_properties" => {
                if field("entity")? != "this" {
                    return Err("only the dropping entity (this) is ported for entity_properties".into());
                }
                let predicate = object.get("predicate").and_then(Value::as_object).ok_or("entity_properties needs a predicate")?;
                if predicate.keys().any(|key| key != "type") {
                    return Err("only the entity type test is ported for entity_properties".into());
                }
                match predicate.get("type").and_then(Value::as_str) {
                    None => Self::EntityProperties(EntityTest::Any),
                    Some(text) if text.starts_with('#') => Self::EntityProperties(EntityTest::Tag(identifier(&text[1..])?)),
                    Some(text) => Self::EntityProperties(EntityTest::Type(identifier(text)?)),
                }
            }
            _ => return Err(format!("unknown loot condition {kind}")),
        })
    }

    fn test(&self, context: &Context) -> bool {
        match self {
            Self::BlockTag(tag) => (context.block_in_tag)(tag),
            Self::TagFilled(tag) => (context.tag_filled)(tag),
            Self::HasModifier(modifier) => (context.tool_level)(modifier) > 0,
            Self::HasChrysophilite => context.chrysophilite >= 1.0,
            Self::ContextSet(set) => match set.as_str() {
                "minecraft:entity" => context.has_entity,
                "minecraft:block" => context.has_block,
                _ => false,
            },
            Self::EntityProperties(test) => match (context.entity, test) {
                (None, _) => false,
                (Some(_), EntityTest::Any) => true,
                (Some(entity), EntityTest::Type(wanted)) => entity == wanted,
                (Some(_), EntityTest::Tag(tag)) => (context.entity_in_tag)(tag),
            },
            Self::BlockOrEntity => context.has_entity || context.has_block,
            Self::LootTableId(wanted) => context.table == wanted.to_string(),
            Self::Config(prop) => (context.config)(prop) == Some(true),
        }
    }
}

/// A condition on the loot as a modifier leaves it, tested by an add-entry modifier after its entry is in: if one fails, the
/// entry is taken back out. In the data the kind is the `type` field, and `mantle:inverted` wraps another in its `condition`.
#[derive(Clone, Debug, PartialEq)]
enum PostCondition {
    /// `mantle:contains_item`: the loot holds a stack the ingredient matches.
    ContainsItem(Ingredient),
    /// `mantle:inverted`: the wrapped condition does not hold.
    Inverted(Box<PostCondition>),
}

impl PostCondition {
    fn decode(value: &Value) -> Result<Self, String> {
        let object = value.as_object().ok_or("a loot post condition must be an object")?;
        let kind = object
            .get("type")
            .or_else(|| object.get("condition"))
            .and_then(Value::as_str)
            .ok_or("a loot post condition needs its type")?;
        Ok(match kind {
            "mantle:contains_item" => Self::ContainsItem(Ingredient::decode(object.get("ingredient").ok_or("contains_item needs an ingredient")?)?),
            "mantle:inverted" => Self::Inverted(Box::new(Self::decode(object.get("condition").ok_or("inverted needs a condition")?)?)),
            _ => return Err(format!("unknown loot post condition {kind}")),
        })
    }

    fn test(&self, loot: &[ItemSlot], context: &Context) -> bool {
        match self {
            Self::ContainsItem(ingredient) => loot.iter().any(|slot| slot.count > 0 && ingredient.matches(&slot.item, context)),
            Self::Inverted(inner) => !inner.test(loot, context),
        }
    }
}

/// Where an add-entry modifier's stack comes from.
#[derive(Clone, Debug, PartialEq)]
enum Entry {
    /// `minecraft:item`: the named item.
    Item(Identifier),
    /// `mantle:tag_preference`: the item the tag prefers.
    TagPreference(Identifier),
}

impl Entry {
    fn decode(value: &Value) -> Result<Self, String> {
        let object = value.as_object().ok_or("a loot entry must be an object")?;
        let kind = object.get("type").and_then(Value::as_str).ok_or("a loot entry needs its type")?;
        let field = |name: &str| object.get(name).and_then(Value::as_str).ok_or_else(|| format!("a {kind} entry needs {name}"));
        Ok(match kind {
            "minecraft:item" => Self::Item(identifier(field("name")?)?),
            "mantle:tag_preference" => Self::TagPreference(identifier(field("tag")?)?),
            _ => return Err(format!("unknown loot entry {kind}")),
        })
    }

    fn create(&self, context: &Context) -> Option<Identifier> {
        match self {
            Self::Item(item) => Some(item.clone()),
            Self::TagPreference(tag) => (context.tag_preference)(tag),
        }
    }
}

/// A number the data rolls: a constant, or a uniform roll between two bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
enum NumberProvider {
    Constant(f32),
    Uniform { min: f32, max: f32 },
}

impl NumberProvider {
    fn decode(value: &Value) -> Result<Self, String> {
        if let Some(number) = value.as_f64() {
            return Ok(Self::Constant(number as f32));
        }
        let object = value.as_object().ok_or("a number provider must be a number or an object")?;
        match object.get("type").and_then(Value::as_str) {
            Some("minecraft:constant") => Ok(Self::Constant(number(object.get("value").ok_or("a constant provider needs a value")?)?)),
            Some("minecraft:uniform") | None => Ok(Self::Uniform {
                min: number(object.get("min").ok_or("a uniform provider needs a min")?)?,
                max: number(object.get("max").ok_or("a uniform provider needs a max")?)?,
            }),
            Some(kind) => Err(format!("unknown number provider {kind}")),
        }
    }

    fn float(&self, random: &mut dyn FnMut() -> f32) -> f32 {
        match self {
            Self::Constant(value) => *value,
            Self::Uniform { min, max } => min + (max - min) * random(),
        }
    }

    /// The game's own integer roll of a provider: a uniform roll is an integer of the inclusive range the bounds name (the
    /// game's number providers evaluate the bounds as integers and roll `max - min + 1` values), so uniform -2 to 1 gives
    /// each of -2, -1, 0 and 1 a quarter of the rolls, as the original's data provider notes for its bacon modifier.
    fn int(&self, random: &mut dyn FnMut() -> f32) -> i32 {
        match self {
            Self::Constant(value) => *value as i32,
            Self::Uniform { min, max } => {
                let (low, high) = (min.ceil() as i32, max.floor() as i32);
                if low >= high {
                    low
                } else {
                    low + (random() * (high - low + 1) as f32) as i32
                }
            }
        }
    }
}

/// How a bonus function grows a count with a level (the game's ApplyBonusCount formulas).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Formula {
    /// `minecraft:ore_drops`: the count times one plus a roll of `level + 2`, less one, floored at no growth.
    OreDrops,
    /// `minecraft:uniform_bonus_count`: the count plus a roll of `bonus_multiplier * level + 1`.
    UniformBonusCount(f32),
    /// `minecraft:binomial_with_bonus_count`: the count plus the successes of `extra * level` rolls at `probability` (the
    /// formula's documented shape; no shipped modifier uses it).
    BinomialWithBonusCount { extra: i32, probability: f32 },
}

impl Formula {
    fn decode(value: &Value, parameters: Option<&Value>) -> Result<Self, String> {
        let (kind, inline) = match value {
            Value::String(text) => (text.as_str(), None),
            Value::Object(object) => (
                object.get("id").and_then(Value::as_str).ok_or("a bonus formula object needs its id")?,
                object.get("parameters"),
            ),
            _ => return Err("a bonus formula must be a name or an object".into()),
        };
        let parameters = parameters.or(inline);
        let field = |name: &str| parameters.and_then(|object| object.get(name)).ok_or_else(|| format!("the {kind} formula needs {name}"));
        Ok(match kind {
            "minecraft:ore_drops" => Self::OreDrops,
            "minecraft:uniform_bonus_count" => Self::UniformBonusCount(number(field("bonus_multiplier")?)?),
            "minecraft:binomial_with_bonus_count" => Self::BinomialWithBonusCount {
                extra: number(field("extra")?)? as i32,
                probability: number(field("probability")?)?,
            },
            _ => return Err(format!("unknown bonus formula {kind}")),
        })
    }

    fn new_count(&self, random: &mut dyn FnMut() -> f32, count: i32, level: i32) -> i32 {
        match self {
            // nextInt(level + 2) - 1, floored at 0, is the multiplier less one.
            Self::OreDrops => count * (((random() * (level + 2) as f32) as i32 - 1).max(0) + 1),
            Self::UniformBonusCount(multiplier) => count + (random() * (multiplier * level as f32 + 1.0)) as i32,
            Self::BinomialWithBonusCount { extra, probability } => {
                count + (0..extra * level).filter(|_| random() < *probability).count() as i32
            }
        }
    }
}

/// One function of a modifier, run over the count of the stack it works on (an added entry, or a replaced stack). Counts are
/// tracked signed through the chain, as the game's stacks go empty at zero or below and only drop at the end if still positive.
#[derive(Clone, Debug, PartialEq)]
enum Function {
    /// `minecraft:set_count`: the count becomes the provider's roll, or grows by it when `add` is set.
    SetCount { count: NumberProvider, add: bool },
    /// `tconstruct:modifier_bonus`: the formula at the tool's level of the modifier (ModifierBonusLootFunction).
    ModifierBonus { modifier: Identifier, formula: Formula, include_base: bool },
    /// `tconstruct:chrysophilite_bonus`: the formula at the breaker's chrysophilite attribute (ChrysophiliteBonusFunction).
    ChrysophiliteBonus { formula: Formula, include_base: bool },
    /// `minecraft:looting_enchant`: the count grows by the looting level times the provider's roll, capped at `limit` when
    /// the data gives one (0 is no cap).
    LootingEnchant { count: NumberProvider, limit: i32 },
    /// `minecraft:explosion_decay`: each item of the stack survives the explosion independently at one in the radius.
    ExplosionDecay,
}

impl Function {
    fn decode(value: &Value) -> Result<Self, String> {
        let object = value.as_object().ok_or("a loot function must be an object")?;
        if object.contains_key("conditions") {
            return Err("loot function conditions are not ported".into());
        }
        let kind = object.get("function").and_then(Value::as_str).ok_or("a loot function needs its function kind")?;
        Ok(match kind {
            "minecraft:set_count" => Self::SetCount {
                count: NumberProvider::decode(object.get("count").ok_or("set_count needs a count")?)?,
                add: object.get("add").and_then(Value::as_bool).unwrap_or(false),
            },
            "minecraft:looting_enchant" => Self::LootingEnchant {
                count: NumberProvider::decode(object.get("count").ok_or("looting_enchant needs a count")?)?,
                limit: object.get("limit").and_then(Value::as_i64).unwrap_or(0) as i32,
            },
            "minecraft:explosion_decay" => Self::ExplosionDecay,
            "tconstruct:modifier_bonus" => Self::ModifierBonus {
                modifier: identifier(object.get("modifier").and_then(Value::as_str).ok_or("modifier_bonus needs a modifier")?)?,
                formula: Formula::decode(object.get("formula").ok_or("modifier_bonus needs a formula")?, object.get("parameters"))?,
                include_base: object.get("include_base").and_then(Value::as_bool).unwrap_or(true),
            },
            "tconstruct:chrysophilite_bonus" => Self::ChrysophiliteBonus {
                formula: Formula::decode(object.get("formula").ok_or("chrysophilite_bonus needs a formula")?, object.get("parameters"))?,
                include_base: object.get("include_base").and_then(Value::as_bool).unwrap_or(true),
            },
            _ => return Err(format!("unknown loot function {kind}")),
        })
    }

    fn run(&self, count: i32, context: &Context, random: &mut dyn FnMut() -> f32) -> i32 {
        match self {
            Self::SetCount { count: provider, add } => {
                let rolled = provider.int(random);
                if *add { count + rolled } else { rolled }
            }
            Self::ModifierBonus { modifier, formula, include_base } => {
                let mut level = (context.tool_level)(modifier);
                if !include_base {
                    level -= 1;
                }
                if level > 0 { formula.new_count(random, count, level) } else { count }
            }
            Self::ChrysophiliteBonus { formula, include_base } => {
                let mut level = context.chrysophilite as i32;
                if !include_base {
                    level -= 1;
                }
                if level > 0 { formula.new_count(random, count, level) } else { count }
            }
            Self::LootingEnchant { count: provider, limit } => {
                if context.tool_looting == 0 {
                    return count;
                }
                let grown = count + (context.tool_looting as f32 * provider.float(random)).round() as i32;
                if *limit > 0 { grown.min(*limit) } else { grown }
            }
            Self::ExplosionDecay => match context.explosion {
                Some(radius) if radius > 0.0 => (0..count).filter(|_| random() < 1.0 / radius).count() as i32,
                _ => count,
            },
        }
    }
}

/// One global loot modifier, decoded.
#[derive(Clone, Debug, PartialEq)]
enum Modifier {
    /// `mantle:add_entry`: when the conditions hold, the entry's stack (after its functions) joins the loot, unless a post
    /// condition then fails against the loot with the entry in it.
    AddEntry { conditions: Vec<Condition>, entry: Entry, functions: Vec<Function>, post: Vec<PostCondition> },
    /// `mantle:replace_item`: every stack the original ingredient matches becomes the replacement, count kept (or the
    /// replacement's own count), with the functions run over it.
    ReplaceItem { conditions: Vec<Condition>, original: Ingredient, replacement: Identifier, replacement_count: Option<i32>, functions: Vec<Function> },
    /// `tconstruct:modifier_hook`: decoded, and deliberately not run (see the module note).
    ModifierHook { conditions: Vec<Condition> },
}

impl Modifier {
    fn decode(value: &Value) -> Result<Self, String> {
        let object = value.as_object().ok_or("a loot modifier must be an object")?;
        let kind = object.get("type").and_then(Value::as_str).ok_or("a loot modifier needs its type")?;
        let conditions = decode_list(object.get("conditions"), Condition::decode)?;
        let functions = decode_list(object.get("functions"), Function::decode)?;
        Ok(match kind {
            ADD_ENTRY => Self::AddEntry {
                conditions,
                entry: Entry::decode(object.get("entry").ok_or("add_entry needs an entry")?)?,
                functions,
                post: decode_list(object.get("post_conditions"), PostCondition::decode)?,
            },
            REPLACE_ITEM => {
                let (replacement, replacement_count) = match object.get("replacement").ok_or("replace_item needs a replacement")? {
                    Value::String(text) => (identifier(text)?, None),
                    Value::Object(output) => (
                        identifier(output.get("item").and_then(Value::as_str).ok_or("a replacement object needs its item")?)?,
                        output.get("count").and_then(Value::as_i64).map(|count| count as i32),
                    ),
                    _ => return Err("a replacement must be an item name or an object".into()),
                };
                Self::ReplaceItem {
                    conditions,
                    original: Ingredient::decode(object.get("original").ok_or("replace_item needs an original")?)?,
                    replacement,
                    replacement_count,
                    functions,
                }
            }
            MODIFIER_HOOK => Self::ModifierHook { conditions },
            _ => return Err(format!("unknown loot modifier type {kind}")),
        })
    }

    fn apply(&self, context: &Context, loot: &mut Vec<ItemSlot>, random: &mut dyn FnMut() -> f32) {
        match self {
            Self::AddEntry { conditions, entry, functions, post } => {
                if !conditions.iter().all(|condition| condition.test(context)) {
                    return;
                }
                let Some(item) = entry.create(context) else { return };
                let mut count: i32 = 1;
                for function in functions {
                    count = function.run(count, context, random);
                }
                if count <= 0 {
                    return;
                }
                let stack = ItemSlot::new(item, count as u32);
                if !post.is_empty() {
                    let mut candidate = loot.clone();
                    candidate.push(stack.clone());
                    if !post.iter().all(|condition| condition.test(&candidate, context)) {
                        return;
                    }
                }
                loot.push(stack);
            }
            Self::ReplaceItem { conditions, original, replacement, replacement_count, functions } => {
                if !conditions.iter().all(|condition| condition.test(context)) {
                    return;
                }
                for slot in loot.iter_mut() {
                    if slot.count == 0 || !original.matches(&slot.item, context) {
                        continue;
                    }
                    slot.item = replacement.clone();
                    let mut count = replacement_count.unwrap_or(slot.count as i32);
                    for function in functions {
                        count = function.run(count, context, random);
                    }
                    slot.count = count.max(0) as u32;
                }
            }
            Self::ModifierHook { .. } => {}
        }
    }
}

/// The decoded modifiers, in the global list's order.
#[derive(Clone, Debug, Default)]
pub(crate) struct Modifiers {
    ordered: Vec<(Identifier, Modifier)>,
    pub problems: Vec<String>,
    pub loaded: bool,
}

impl Modifiers {
    pub fn load(files: &[(Identifier, Value)]) -> Self {
        let mut result = Self {
            loaded: true,
            ..Self::default()
        };
        let mut decoded: Vec<(Identifier, Modifier)> = Vec::new();
        let mut sorted: Vec<&(Identifier, Value)> = files.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (id, value) in sorted {
            match Modifier::decode(value) {
                Ok(modifier) => decoded.push((id.clone(), modifier)),
                Err(error) => result.problems.push(format!("{id}: {error}")),
            }
        }
        for name in ORDER {
            let wanted = Identifier::parse(name).expect("a written global loot modifier name");
            match decoded.iter().position(|(found, _)| *found == wanted) {
                Some(at) => {
                    let (_, modifier) = decoded.remove(at);
                    result.ordered.push((wanted, modifier));
                }
                None => result.problems.push(format!("{wanted}: listed in the global loot modifier list but no file was loaded")),
            }
        }
        result
    }

    pub fn len(&self) -> usize {
        self.ordered.len()
    }

    /// Runs every listed modifier over the loot, in the global list's order.
    pub fn apply(&self, context: &Context, loot: &mut Vec<ItemSlot>, random: &mut dyn FnMut() -> f32) {
        for (_, modifier) in &self.ordered {
            modifier.apply(context, loot, random);
        }
    }
}

/// Reload listener, shaped like the other data listeners.
pub(crate) fn register() -> Result<Arc<RwLock<Modifiers>>, String> {
    let state = Arc::new(RwLock::new(Modifiers::default()));
    let output = state.clone();
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "loot_modifiers").unwrap(),
        FOLDER,
        move |files| {
            let mut state = output.write().map_err(|_| "loot modifiers poisoned")?;
            *state = Modifiers::load(files);
            for problem in &state.problems {
                eprintln!("Tinkers: {problem}");
            }
            Ok(())
        },
    )?;
    let stopped = state.clone();
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| {
        *stopped.write().expect("loot modifiers poisoned") = Modifiers::default()
    });
    Ok(state)
}

fn identifier(text: &str) -> Result<Identifier, String> {
    Identifier::parse(text).map_err(|_| format!("invalid identifier {text}"))
}

fn number(value: &Value) -> Result<f32, String> {
    value.as_f64().map(|number| number as f32).ok_or_else(|| "expected a number".to_string())
}

fn decode_list<T>(value: Option<&Value>, decode: fn(&Value) -> Result<T, String>) -> Result<Vec<T>, String> {
    match value {
        None => Ok(Vec::new()),
        Some(Value::Array(list)) => list.iter().map(decode).collect(),
        Some(_) => Err("expected a list".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn id(text: &str) -> Identifier {
        Identifier::parse(text).unwrap()
    }

    /// A context whose lookups are all empty, for tests to override field by field.
    fn bare<'a>(
        block_in_tag: &'a dyn Fn(&Identifier) -> bool,
        entity_in_tag: &'a dyn Fn(&Identifier) -> bool,
        item_in_tag: &'a dyn Fn(&Identifier, &Identifier) -> bool,
        tag_filled: &'a dyn Fn(&Identifier) -> bool,
        tag_preference: &'a dyn Fn(&Identifier) -> Option<Identifier>,
        tool_level: &'a dyn Fn(&Identifier) -> i32,
        config: &'a dyn Fn(&str) -> Option<bool>,
    ) -> Context<'a> {
        Context {
            table: "",
            has_block: false,
            has_entity: false,
            entity: None,
            block_in_tag,
            entity_in_tag,
            item_in_tag,
            tag_filled,
            tag_preference,
            tool_level,
            tool_looting: 0,
            chrysophilite: 0.0,
            config,
            explosion: None,
        }
    }

    fn no_bool(_: &Identifier) -> bool {
        false
    }
    fn no_item(_: &Identifier, _: &Identifier) -> bool {
        false
    }
    fn no_preference(_: &Identifier) -> Option<Identifier> {
        None
    }
    fn no_level(_: &Identifier) -> i32 {
        0
    }
    fn no_config(_: &str) -> Option<bool> {
        None
    }

    fn iron() -> Value {
        json!({
            "type": "mantle:add_entry",
            "conditions": [
                {"condition": "mantle:block_tag", "tag": "forge:ores/iron"},
                {"condition": "tconstruct:has_modifier", "modifier": "tconstruct:lustrous"}
            ],
            "entry": {"type": "mantle:tag_preference", "tag": "forge:nuggets/iron"},
            "functions": [
                {"add": false, "count": {"type": "minecraft:uniform", "max": 4, "min": 2}, "function": "minecraft:set_count"},
                {"formula": "minecraft:ore_drops", "function": "tconstruct:modifier_bonus", "include_base": false, "modifier": "tconstruct:lustrous"},
                {"function": "minecraft:explosion_decay"}
            ],
            "post_conditions": [
                {"type": "mantle:inverted", "condition": {"type": "mantle:contains_item", "ingredient": {"tag": "forge:ores/iron"}}}
            ]
        })
    }

    #[test]
    fn shipped_shapes_decode() {
        // The compat shape: a tag_filled condition between the block tag and the modifier test.
        let aluminum = json!({
            "type": "mantle:add_entry",
            "conditions": [
                {"condition": "mantle:block_tag", "tag": "forge:ores/aluminum"},
                {"condition": "mantle:tag_filled", "tag": "forge:nuggets/aluminum"},
                {"condition": "tconstruct:has_modifier", "modifier": "tconstruct:lustrous"}
            ],
            "entry": {"type": "mantle:tag_preference", "tag": "forge:nuggets/aluminum"},
            "functions": [],
            "post_conditions": []
        });
        assert!(Modifier::decode(&aluminum).is_ok());
        assert!(Modifier::decode(&iron()).is_ok());
        let tasty = json!({
            "type": "mantle:add_entry",
            "conditions": [
                {"condition": "tconstruct:has_context_set", "set": "minecraft:entity"},
                {"condition": "minecraft:entity_properties", "entity": "this", "predicate": {"type": "#tconstruct:bacon_producer"}},
                {"condition": "tconstruct:has_modifier", "modifier": "tconstruct:tasty"}
            ],
            "entry": {"type": "minecraft:item", "name": "tconstruct:bacon"},
            "functions": [
                {"add": false, "count": {"type": "minecraft:uniform", "max": 1, "min": -2}, "function": "minecraft:set_count"},
                {"count": {"type": "minecraft:uniform", "max": 1, "min": 0}, "function": "minecraft:looting_enchant"}
            ],
            "post_conditions": []
        });
        assert!(Modifier::decode(&tasty).is_ok());
        let wither = json!({
            "type": "mantle:replace_item",
            "conditions": [
                {"condition": "forge:loot_table_id", "loot_table_id": "minecraft:entities/wither_skeleton"},
                {"condition": "tconstruct:config", "prop": "wither_bone_drop"}
            ],
            "functions": [],
            "original": {"item": "minecraft:bone"},
            "replacement": "tconstruct:necrotic_bone"
        });
        assert!(Modifier::decode(&wither).is_ok());
        let hook = json!({"type": "tconstruct:modifier_hook", "conditions": [{"condition": "tconstruct:block_or_entity"}]});
        assert!(Modifier::decode(&hook).is_ok());
        // An unreviewed kind is a load problem, never a silent skip.
        assert!(Modifier::decode(&json!({"type": "mantle:add_item"})).is_err());
        assert!(Modifier::decode(&json!({"type": "mantle:add_entry", "conditions": [{"condition": "minecraft:random_chance"}], "entry": {"type": "minecraft:item", "name": "minecraft:stone"}})).is_err());
    }

    #[test]
    fn uniform_integers_cover_the_inclusive_range() {
        let bacon = NumberProvider::Uniform { min: -2.0, max: 1.0 };
        assert_eq!(bacon.int(&mut || 0.0), -2);
        assert_eq!(bacon.int(&mut || 0.25), -1);
        assert_eq!(bacon.int(&mut || 0.5), 0);
        assert_eq!(bacon.int(&mut || 0.99), 1);
        let nuggets = NumberProvider::Uniform { min: 2.0, max: 4.0 };
        assert_eq!(nuggets.int(&mut || 0.0), 2);
        assert_eq!(nuggets.int(&mut || 0.99), 4);
    }

    #[test]
    fn lustrous_adds_nuggets_unless_the_ore_itself_dropped() {
        let modifier = Modifier::decode(&iron()).unwrap();
        let block = |tag: &Identifier| *tag == id("forge:ores/iron");
        let ore_member = |item: &Identifier, tag: &Identifier| *item == id("minecraft:iron_ore") && *tag == id("forge:ores/iron");
        let level = |modifier: &Identifier| if *modifier == id("tconstruct:lustrous") { 1 } else { 0 };
        let preference = |tag: &Identifier| (*tag == id("forge:nuggets/iron")).then(|| id("minecraft:iron_nugget"));
        let mut context = bare(&block, &no_bool, &ore_member, &no_bool, &preference, &level, &no_config);
        context.has_block = true;
        // Set count rolls its lowest (2); the bonus at level 1 with the base excluded leaves it alone.
        let mut loot = vec![ItemSlot::new(id("minecraft:iron_ore"), 1)];
        // The ore in the loot is what a silk touch break drops: the post condition takes the nuggets back out.
        modifier.apply(&context, &mut loot, &mut || 0.0);
        assert_eq!(loot.len(), 1);
        let mut loot = Vec::new();
        modifier.apply(&context, &mut loot, &mut || 0.0);
        assert_eq!(loot.len(), 1);
        assert_eq!(loot[0].item, id("minecraft:iron_nugget"));
        assert_eq!(loot[0].count, 2);
        // No lustrous on the tool: nothing.
        let mut context = bare(&block, &no_bool, &no_item, &no_bool, &preference, &no_level, &no_config);
        context.has_block = true;
        let mut loot = Vec::new();
        modifier.apply(&context, &mut loot, &mut || 0.0);
        assert!(loot.is_empty());
    }

    #[test]
    fn tasty_bacon_rolls_and_looting_grows_it() {
        let tasty = json!({
            "type": "mantle:add_entry",
            "conditions": [
                {"condition": "tconstruct:has_context_set", "set": "minecraft:entity"},
                {"condition": "minecraft:entity_properties", "entity": "this", "predicate": {"type": "#tconstruct:bacon_producer"}},
                {"condition": "tconstruct:has_modifier", "modifier": "tconstruct:tasty"}
            ],
            "entry": {"type": "minecraft:item", "name": "tconstruct:bacon"},
            "functions": [
                {"add": false, "count": {"type": "minecraft:uniform", "max": 1, "min": -2}, "function": "minecraft:set_count"},
                {"count": {"type": "minecraft:uniform", "max": 1, "min": 0}, "function": "minecraft:looting_enchant"}
            ],
            "post_conditions": []
        });
        let modifier = Modifier::decode(&tasty).unwrap();
        let pig = id("minecraft:pig");
        let in_tag = |tag: &Identifier| *tag == id("tconstruct:bacon_producer");
        let level = |modifier: &Identifier| if *modifier == id("tconstruct:tasty") { 1 } else { 0 };
        let mut context = bare(&no_bool, &in_tag, &no_item, &no_bool, &no_preference, &level, &no_config);
        context.has_entity = true;
        context.entity = Some(&pig);
        // The top of the set count range is the one roll of four that drops a bacon.
        let mut loot = Vec::new();
        modifier.apply(&context, &mut loot, &mut || 0.99);
        assert_eq!(loot.len(), 1);
        assert_eq!(loot[0].count, 1);
        // The bottom of the range drops nothing.
        let mut loot = Vec::new();
        modifier.apply(&context, &mut loot, &mut || 0.0);
        assert!(loot.is_empty());
        // Looting 2 with a full looting roll grows the one bacon by two.
        context.tool_looting = 2;
        let mut loot = Vec::new();
        modifier.apply(&context, &mut loot, &mut || 0.99);
        assert_eq!(loot[0].count, 3);
    }

    #[test]
    fn wither_bone_replacement_needs_the_table_and_the_config() {
        let wither = json!({
            "type": "mantle:replace_item",
            "conditions": [
                {"condition": "forge:loot_table_id", "loot_table_id": "minecraft:entities/wither_skeleton"},
                {"condition": "tconstruct:config", "prop": "wither_bone_drop"}
            ],
            "functions": [],
            "original": {"item": "minecraft:bone"},
            "replacement": "tconstruct:necrotic_bone"
        });
        let modifier = Modifier::decode(&wither).unwrap();
        let on = |prop: &str| (prop == "wither_bone_drop").then_some(true);
        let mut context = bare(&no_bool, &no_bool, &no_item, &no_bool, &no_preference, &no_level, &on);
        context.table = "minecraft:entities/wither_skeleton";
        let mut loot = vec![ItemSlot::new(id("minecraft:bone"), 3), ItemSlot::new(id("minecraft:coal"), 1)];
        modifier.apply(&context, &mut loot, &mut || 0.0);
        assert_eq!(loot[0].item, id("tconstruct:necrotic_bone"));
        assert_eq!(loot[0].count, 3);
        assert_eq!(loot[1].item, id("minecraft:coal"));
        // Config off: the bones stay bones.
        let off = |_: &str| Some(false);
        let mut context = bare(&no_bool, &no_bool, &no_item, &no_bool, &no_preference, &no_level, &off);
        context.table = "minecraft:entities/wither_skeleton";
        let mut loot = vec![ItemSlot::new(id("minecraft:bone"), 1)];
        modifier.apply(&context, &mut loot, &mut || 0.0);
        assert_eq!(loot[0].item, id("minecraft:bone"));
    }

    #[test]
    fn chrysophilite_bonus_uses_the_attribute_as_its_level() {
        let modifier = Modifier::decode(&json!({
            "type": "mantle:add_entry",
            "conditions": [
                {"condition": "mantle:block_tag", "tag": "tconstruct:chrysophilite_ores"},
                {"condition": "tconstruct:has_chrysophilite"}
            ],
            "entry": {"type": "minecraft:item", "name": "minecraft:gold_nugget"},
            "functions": [
                {"add": false, "count": {"type": "minecraft:uniform", "max": 6, "min": 2}, "function": "minecraft:set_count"},
                {"formula": "minecraft:ore_drops", "function": "tconstruct:chrysophilite_bonus", "include_base": false},
                {"function": "minecraft:explosion_decay"}
            ],
            "post_conditions": [
                {"type": "mantle:inverted", "condition": {"type": "mantle:contains_item", "ingredient": {"tag": "tconstruct:chrysophilite_ores"}}}
            ]
        }))
        .unwrap();
        let block = |tag: &Identifier| *tag == id("tconstruct:chrysophilite_ores");
        let mut context = bare(&block, &no_bool, &no_item, &no_bool, &no_preference, &no_level, &no_config);
        context.has_block = true;
        // Attribute 3: the bonus level is 2 with the base excluded; set count rolls 2, ore drops rolls its top (3-1=2 over).
        context.chrysophilite = 3.0;
        let mut rolls = [0.0, 0.99].into_iter();
        let mut loot = Vec::new();
        modifier.apply(&context, &mut loot, &mut || rolls.next().unwrap_or(0.0));
        assert_eq!(loot.len(), 1);
        assert_eq!(loot[0].count, 6);
        // No chrysophilite worn: the condition alone stops it.
        context.chrysophilite = 0.0;
        let mut loot = Vec::new();
        modifier.apply(&context, &mut loot, &mut || 0.0);
        assert!(loot.is_empty());
    }

    #[test]
    fn explosion_decay_thins_a_stack_only_in_an_explosion() {
        let decay = Function::ExplosionDecay;
        let mut context = bare(&no_bool, &no_bool, &no_item, &no_bool, &no_preference, &no_level, &no_config);
        assert_eq!(decay.run(4, &context, &mut || 0.0), 4);
        context.explosion = Some(2.0);
        assert_eq!(decay.run(4, &context, &mut || 0.4), 4);
        assert_eq!(decay.run(4, &context, &mut || 0.6), 0);
    }

    #[test]
    fn only_listed_modifiers_run_in_list_order() {
        let files = [
            (id("tconstruct:tasty_bacon"), json!({"type": "tconstruct:modifier_hook", "conditions": []})),
            (id("tconstruct:lustrous/iron"), iron()),
            (id("tconstruct:unlisted"), json!({"type": "tconstruct:modifier_hook", "conditions": []})),
        ];
        let modifiers = Modifiers::load(&files);
        assert_eq!(modifiers.len(), 2);
        // Every other listed name is reported missing; the unlisted file is not a problem and never runs.
        assert_eq!(modifiers.problems.len(), ORDER.len() - 2);
        assert!(modifiers.problems.iter().all(|problem| !problem.contains("unlisted")));
    }
}
