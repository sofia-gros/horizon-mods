//! The fluid effects (SlimeKnights Tinkers' Construct and Mantle, MIT; see LICENSE): the data of
//! `tinkering/fluid_effects`, decoded and run with the original's FluidEffectManager semantics.
//!
//! A file names a fluid ingredient (an amount of a fluid or a fluid tag per level), the block effects and the
//! entity effects it runs, and Forge load conditions that switch the whole file off. [`Effects::find`] is the
//! manager's lookup: the first entry whose ingredient matches the fluid, as the original's per-fluid cache
//! resolves it. Running an entry scales the requested level by the fluid on hand, runs every effect against
//! the context, and charges the ceiling of the amount per level times the largest scale any effect used
//! (`FluidEffects.apply`).
//!
//! Effects run against [`WorldAccess`] and [`EntityTarget`], the port's stand-ins for the original's level and
//! entities: an effect that the game would only simulate is asked with [`FluidAction::Simulate`] and changes
//! nothing. Consumers (the fluid cannon, spilling and splashing, the projectile) build the contexts; the
//! contact behaviour of the fluid blocks themselves is in `fluids.rs`, on PlutoLib's entity-inside hook.
//!
//! Two reaches the original takes for granted are still the consumer's to supply, so the effects that need
//! them run only where a context supplies them: potion contents (a fluid's NBT read as a potion, through
//! [`WorldAccess::potion_effects`]) and the vanilla interaction pipelines behind `interact`
//! ([`WorldAccess::interact_block`], [`WorldAccess::interact_entity`]).
#![allow(dead_code)]
use crate::{
    fluid_ingredient::{id_of, int_field, only_keys, FluidIngredient, FluidTags, InTag, ItemOutput, Source},
    fluid_recipes,
    materials::Conditions,
};
use pluto_lib::{
    BlockPos, BlockState, Direction, FluidStack, Identifier, Nbt, NbtCompound, ResourceManagerHelper, ResourceType,
    ServerLifecycleEvents,
};
use serde_json::{Map, Value};
use std::sync::{Arc, RwLock};

/// The data folder of the fluid effects, as the reload listener reads it.
pub(crate) const FOLDER: &str = "tinkering/fluid_effects";

/* The manager's arithmetic (FluidEffects.apply, EffectLevel) */

/// Whether an effect run may change the world (FluidAction).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FluidAction {
    Simulate,
    Execute,
}

impl FluidAction {
    pub(crate) fn execute(self) -> bool { self == Self::Execute }
    pub(crate) fn simulate(self) -> bool { self == Self::Simulate }
}

/// How much of an effect applies: the scale the fluid on hand buys, and the level that was asked for
/// (EffectLevel). Conservative effects may raise a target up to `max` by summing with what it already has.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct EffectLevel {
    pub value: f32,
    pub max: f32,
}

impl EffectLevel {
    pub(crate) fn new(value: f32, max: f32) -> Self { Self { value, max } }
    /// The level reached by adding this level to what the target already has, capped at the maximum.
    pub(crate) fn effective(self, existing: f32) -> f32 { (self.value + existing).min(self.max) }
    /// The part of this level a cap of `max` lets through.
    pub(crate) fn compute_used(self, max: f32) -> f32 { self.value.min(max) }
    /// Flat effects (they do not scale) require a full level.
    pub(crate) fn is_full(self) -> bool { self.value >= 1.0 }
    /// Both parts divided (ScalingFluidEffect runs its inner effect at a divided level).
    pub(crate) fn divide(self, divisor: f32) -> Self {
        if divisor == 1.0 { return self; }
        Self { value: self.value / divisor, max: self.max / divisor }
    }
    /// The value less what an earlier target used (AreaMobEffectFluidEffect with a summed group cost).
    pub(crate) fn subtract(self, amount: f32) -> Self {
        if amount == 0.0 { return self; }
        Self { value: self.value - amount, max: self.max }
    }
}

/// A value with a flat part and a part per level (LevelingValue), as an object with either part or a plain
/// number for the flat part.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LevelingValue {
    pub flat: f32,
    pub each_level: f32,
}

impl LevelingValue {
    pub(crate) fn flat(value: f32) -> Self { Self { flat: value, each_level: 0.0 } }
    pub(crate) fn is_flat(self) -> bool { self.each_level == 0.0 }
    /// `flat + each_level * level`.
    pub(crate) fn compute(self, level: f32) -> f32 { self.flat + self.each_level * level }
    /// For fluid effects, where the level may be under 1: below 1 the flat part scales with the level too.
    pub(crate) fn compute_for_scale(self, level: f32) -> f32 {
        if level < 1.0 { (self.flat + self.each_level) * level } else { self.compute(level) }
    }
    pub(crate) fn decode(value: &Value) -> Result<Self, String> {
        match value {
            Value::Number(number) => Ok(Self::flat(number.as_f64().ok_or("leveling value must be a number")? as f32)),
            Value::Object(fields) => {
                only_keys(fields, "leveling value", &["flat", "each_level"])?;
                let part = |key: &str| match fields.get(key) {
                    None | Some(Value::Null) => Ok(0.0),
                    Some(Value::Number(number)) => number.as_f64().map(|value| value as f32).ok_or_else(|| format!("{key} must be a number")),
                    Some(_) => Err(format!("{key} must be a number")),
                };
                Ok(Self { flat: part("flat")?, each_level: part("each_level")? })
            }
            _ => Err("leveling value must be a number or an object".into()),
        }
    }
}

/// A whole-number leveling value (LevelingInt): a plain number is the amount for each level, as the port
/// reads leveling ints elsewhere; an object carries `flat` and `each_level` parts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LevelingInt {
    pub flat: i32,
    pub each_level: i32,
}

impl LevelingInt {
    pub(crate) fn flat(value: i32) -> Self { Self { flat: value, each_level: 0 } }
    pub(crate) fn is_flat(self) -> bool { self.each_level == 0 }
    /// `flat + each_level * level`, truncated to a whole number.
    pub(crate) fn compute(self, level: f32) -> i32 { (self.flat as f32 + self.each_level as f32 * level) as i32 }
    pub(crate) fn decode(value: &Value) -> Result<Self, String> {
        match value {
            Value::Number(_) => Ok(Self { flat: 0, each_level: int_of(value, "leveling int")? }),
            Value::Object(fields) => {
                only_keys(fields, "leveling int", &["flat", "each_level"])?;
                let part = |key: &str| match fields.get(key) {
                    None | Some(Value::Null) => Ok(0),
                    Some(number) => int_of(number, key),
                };
                Ok(Self { flat: part("flat")?, each_level: part("each_level")? })
            }
            _ => Err("leveling int must be a number or an object".into()),
        }
    }
}

fn int_of(value: &Value, what: &str) -> Result<i32, String> {
    value.as_i64().filter(|value| i32::try_from(*value).is_ok()).map(|value| value as i32)
        .ok_or_else(|| format!("{what} must be a whole number"))
}

/// How a timed effect combines with what the target already has (TimeAction).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TimeAction {
    Add,
    Set,
}

impl TimeAction {
    fn decode(value: &Value) -> Result<Self, String> {
        match value.as_str() {
            Some("add") => Ok(Self::Add),
            Some("set") => Ok(Self::Set),
            _ => Err("time action must be \"add\" or \"set\"".into()),
        }
    }
}

/// How an area effect charges for several targets (GroupCost).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GroupCost {
    Sum,
    Max,
}

impl GroupCost {
    fn decode(value: &Value) -> Result<Self, String> {
        match value.as_str() {
            Some("sum") => Ok(Self::Sum),
            Some("max") => Ok(Self::Max),
            _ => Err("group cost must be \"sum\" or \"max\"".into()),
        }
    }
}

/// What an explosion does to blocks (Explosion.BlockInteraction).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BlockInteraction {
    Keep,
    Destroy,
    DestroyWithDecay,
    None,
}

impl BlockInteraction {
    fn decode(value: &Value) -> Result<Self, String> {
        match value.as_str() {
            Some("keep") => Ok(Self::Keep),
            Some("destroy") => Ok(Self::Destroy),
            Some("destroy_with_decay") => Ok(Self::DestroyWithDecay),
            Some("none") => Ok(Self::None),
            _ => Err("block interaction must be keep, destroy, destroy_with_decay or none".into()),
        }
    }
}

/* Stacks and effects on entities */

/// An item stack as the effects handle one: the fallback stack of a context, a hand, a cure item, a drop.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Stack {
    pub item: Identifier,
    pub count: i32,
    pub nbt: Option<Value>,
}

impl Stack {
    pub(crate) fn empty() -> Self {
        Self { item: Identifier::new("minecraft", "air").expect("valid id"), count: 0, nbt: None }
    }
    pub(crate) fn of(item: Identifier, count: i32) -> Self { Self { item, count, nbt: None } }
    pub(crate) fn is_empty(&self) -> bool { self.count <= 0 }
}

/// A mob effect as it is about to be given: the amplifier is the level less one, the duration in ticks
/// (-1 is infinite), and the items that cure it when the definition names them (MobEffectInstance).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NewEffect {
    pub effect: Identifier,
    pub duration: i32,
    pub amplifier: i32,
    pub curative_items: Option<Vec<Stack>>,
}

/// A mob effect a target already has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ActiveEffect {
    pub amplifier: i32,
    pub duration: i32,
}

/// One effect of a potion, as [`WorldAccess::potion_effects`] reads it from a fluid's NBT (PotionUtils).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PotionEffect {
    pub effect: Identifier,
    pub duration: i32,
    pub amplifier: i32,
    pub instant: bool,
}

/* NBT predicates (TagPredicate) */

/// An NBT template matched against a fluid's tag (TagPredicate): every entry of the template must be found
/// in the tag, compounds and lists matching in part (NbtUtils.compareNbt with partial matching).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TagPredicate(Option<NbtCompound>);

impl TagPredicate {
    pub(crate) fn any() -> Self { Self(None) }
    pub(crate) fn test(&self, tag: Option<&NbtCompound>) -> bool {
        match &self.0 {
            None => true,
            Some(template) => tag.is_some_and(|tag| nbt_matches(&Nbt::Compound(template.clone()), &Nbt::Compound(tag.clone()))),
        }
    }
    fn decode(value: &Value) -> Result<Self, String> {
        match value {
            Value::Object(_) => Ok(Self(Some(nbt_compound(value)?))),
            _ => Err("nbt predicate must be an NBT object".into()),
        }
    }
}

/// A JSON object as an NBT compound, the conversion Mantle's NBT loadable makes: text stays text, a whole
/// number is an int, any other number a double, a boolean a byte.
fn nbt_compound(value: &Value) -> Result<NbtCompound, String> {
    let fields = value.as_object().ok_or("nbt must be an object")?;
    fields.iter().map(|(key, value)| Ok((key.clone(), nbt_value(value)?))).collect()
}

fn nbt_value(value: &Value) -> Result<Nbt, String> {
    Ok(match value {
        Value::Null => return Err("nbt cannot hold null".into()),
        Value::Bool(flag) => Nbt::Byte(i8::from(*flag)),
        Value::Number(number) => match number.as_i64() {
            Some(whole) if i32::try_from(whole).is_ok() => Nbt::Int(whole as i32),
            _ => Nbt::Double(number.as_f64().ok_or("invalid nbt number")?),
        },
        Value::String(text) => Nbt::String(text.clone()),
        Value::Array(items) => Nbt::List(items.iter().map(nbt_value).collect::<Result<_, _>>()?),
        Value::Object(_) => Nbt::Compound(nbt_compound(value)?),
    })
}

/// NbtUtils.compareNbt(template, actual, true): numbers compare across the numeric tag kinds, a compound
/// matches when every template entry matches somewhere in it, a list when every template element matches
/// some element of the actual list.
fn nbt_matches(template: &Nbt, actual: &Nbt) -> bool {
    match (template, actual) {
        (Nbt::Compound(want), Nbt::Compound(have)) => want.iter().all(|(key, value)| have.get(key).is_some_and(|other| nbt_matches(value, other))),
        (Nbt::List(want), Nbt::List(have)) => want.len() <= have.len() && want.iter().all(|value| have.iter().any(|other| nbt_matches(value, other))),
        _ => match (template.as_f64(), actual.as_f64()) {
            (Some(want), Some(have)) => want == have,
            _ => template == actual,
        },
    }
}

/* Block and living-entity predicates (Mantle's IJsonPredicate vocabularies) */

/// A predicate over a block state (BlockPredicate): a bare string names one of Mantle's simple predicates,
/// an object is one of the record forms.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BlockPredicate {
    Any,
    Named(String),
    Tag(Identifier),
    Set(Vec<Identifier>),
    HarvestTier(String),
    Not(Box<BlockPredicate>),
    And(Vec<BlockPredicate>),
    Or(Vec<BlockPredicate>),
}

impl BlockPredicate {
    pub(crate) fn decode(value: &Value) -> Result<Self, String> {
        if let Some(name) = value.as_str() {
            return Ok(if name == "mantle:any" { Self::Any } else { Self::Named(name.to_owned()) });
        }
        let fields = value.as_object().ok_or("block predicate must be a name or an object")?;
        let kind = fields.get("type").and_then(Value::as_str).ok_or("block predicate needs a type")?;
        let children = |fields: &Map<String, Value>| -> Result<Vec<BlockPredicate>, String> {
            fields.get("predicates").and_then(Value::as_array).ok_or("predicates must be an array")?
                .iter().map(Self::decode).collect()
        };
        Ok(match kind {
            "mantle:any" => { only_keys(fields, "block predicate", &["type"])?; Self::Any }
            "mantle:tag" => { only_keys(fields, "block predicate", &["type", "tag"])?; Self::Tag(id_of(fields.get("tag").ok_or("tag missing")?)?) }
            "mantle:set" => {
                only_keys(fields, "block predicate", &["type", "blocks"])?;
                let blocks = fields.get("blocks").and_then(Value::as_array).ok_or("blocks must be an array")?;
                Self::Set(blocks.iter().map(id_of).collect::<Result<_, _>>()?)
            }
            "mantle:inverted" => {
                let inner = retyped(fields, "inverted_type")?;
                Self::Not(Box::new(Self::decode(&inner)?))
            }
            "mantle:and" => { only_keys(fields, "block predicate", &["type", "predicates"])?; Self::And(children(fields)?) }
            "mantle:or" => { only_keys(fields, "block predicate", &["type", "predicates"])?; Self::Or(children(fields)?) }
            "tconstruct:harvest_tier" => {
                only_keys(fields, "block predicate", &["type", "tier"])?;
                Self::HarvestTier(fields.get("tier").and_then(Value::as_str).ok_or("tier must be an item tier name")?.to_owned())
            }
            _ => return Err(format!("unimplemented block predicate type {kind}")),
        })
    }

    pub(crate) fn matches(&self, world: &dyn WorldAccess, pos: BlockPos, state: &BlockState) -> bool {
        match self {
            Self::Any => true,
            Self::Named(name) => world.named_block_predicate(name, pos, state).unwrap_or(false),
            Self::Tag(tag) => state.is_in_tag(&tag.to_string()),
            Self::Set(blocks) => blocks.contains(&state.block),
            Self::HarvestTier(tier) => world.correct_tier_for_drops(tier, pos),
            Self::Not(inner) => !inner.matches(world, pos, state),
            Self::And(children) => children.iter().all(|child| child.matches(world, pos, state)),
            Self::Or(children) => children.iter().any(|child| child.matches(world, pos, state)),
        }
    }
}

/// A predicate over a living entity (LivingEntityPredicate), in the same Mantle vocabulary.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum LivingPredicate {
    Any,
    Named(String),
    Tag(Identifier),
    Set(Vec<Identifier>),
    HasEffect(Identifier),
    BlockAtEntity(Box<BlockPredicate>),
    Not(Box<LivingPredicate>),
    And(Vec<LivingPredicate>),
    Or(Vec<LivingPredicate>),
}

impl LivingPredicate {
    pub(crate) fn decode(value: &Value) -> Result<Self, String> {
        if let Some(name) = value.as_str() {
            return Ok(if name == "mantle:any" { Self::Any } else { Self::Named(name.to_owned()) });
        }
        let fields = value.as_object().ok_or("entity predicate must be a name or an object")?;
        let kind = fields.get("type").and_then(Value::as_str).ok_or("entity predicate needs a type")?;
        let children = |fields: &Map<String, Value>| -> Result<Vec<LivingPredicate>, String> {
            fields.get("predicates").and_then(Value::as_array).ok_or("predicates must be an array")?
                .iter().map(Self::decode).collect()
        };
        Ok(match kind {
            "mantle:any" => { only_keys(fields, "entity predicate", &["type"])?; Self::Any }
            "mantle:fire_immune" => { only_keys(fields, "entity predicate", &["type"])?; Self::Named(kind.to_owned()) }
            "mantle:tag" => { only_keys(fields, "entity predicate", &["type", "tag"])?; Self::Tag(id_of(fields.get("tag").ok_or("tag missing")?)?) }
            "mantle:set" => {
                only_keys(fields, "entity predicate", &["type", "entity_types"])?;
                let types = fields.get("entity_types").and_then(Value::as_array).ok_or("entity_types must be an array")?;
                Self::Set(types.iter().map(id_of).collect::<Result<_, _>>()?)
            }
            "mantle:has_effect" => { only_keys(fields, "entity predicate", &["type", "effect"])?; Self::HasEffect(id_of(fields.get("effect").ok_or("effect missing")?)?) }
            "mantle:block_at_entity" => {
                only_keys(fields, "entity predicate", &["type", "block_type"])?;
                Self::BlockAtEntity(Box::new(BlockPredicate::decode(fields.get("block_type").ok_or("block_type missing")?)?))
            }
            "mantle:inverted" => {
                let inner = retyped(fields, "inverted_type")?;
                Self::Not(Box::new(Self::decode(&inner)?))
            }
            "mantle:and" => { only_keys(fields, "entity predicate", &["type", "predicates"])?; Self::And(children(fields)?) }
            "mantle:or" => { only_keys(fields, "entity predicate", &["type", "predicates"])?; Self::Or(children(fields)?) }
            _ => return Err(format!("unimplemented entity predicate type {kind}")),
        })
    }

    pub(crate) fn matches(&self, world: &dyn WorldAccess, target: &dyn EntityTarget) -> bool {
        match self {
            Self::Any => true,
            Self::Named(name) => target.named_predicate(name).unwrap_or(false),
            Self::Tag(tag) => target.in_type_tag(tag),
            Self::Set(types) => types.iter().any(|kind| kind.to_string() == target.kind()),
            Self::HasEffect(effect) => target.has_effect(effect),
            Self::BlockAtEntity(predicate) => {
                let pos = target.block_position();
                world.block_state(pos).is_some_and(|state| predicate.matches(world, pos, &state))
            }
            Self::Not(inner) => !inner.matches(world, target),
            Self::And(children) => children.iter().all(|child| child.matches(world, target)),
            Self::Or(children) => children.iter().any(|child| child.matches(world, target)),
        }
    }
}

/// Mantle's inverted loadable: the wrapped predicate's fields sit beside `inverted_type`, which takes the
/// place of `type` when the object is read again.
fn retyped(fields: &Map<String, Value>, type_key: &str) -> Result<Value, String> {
    let kind = fields.get(type_key).and_then(Value::as_str).ok_or_else(|| format!("inverted predicate needs {type_key}"))?;
    let mut inner = fields.clone();
    inner.remove(type_key);
    inner.insert("type".into(), Value::String(kind.to_owned()));
    Ok(Value::Object(inner))
}

/* The contexts effects run against (FluidEffectContext, Entity, Level) */

/// The entity using the fluid, as facts: which entity it is, whether it is a player, the projectile that
/// carries the fluid if any, the fallback stack and the hands, and what the game mode lets it do.
#[derive(Clone, Debug)]
pub(crate) struct Holder {
    /// The living entity using the fluid, when one does.
    pub entity: Option<i32>,
    /// The user is a player (the context's player, set when the entity is one).
    pub player: bool,
    /// The projectile carrying the fluid, when one does.
    pub projectile: Option<ProjectileFacts>,
    /// The stack the fluid came from, for when no entity holds one (the context's stack).
    pub stack: Stack,
    pub main_hand: Stack,
    pub off_hand: Stack,
    /// The player may build (always true when there is no player).
    pub may_build: bool,
    /// The player is in adventure mode (a server player), which restricts breaking and placing.
    pub adventure: bool,
    /// The player places without consuming (instabuild).
    pub creative: bool,
    /// The user is sneaking, which may skip a block's own use.
    pub sneak: bool,
    /// The entity's position, for directions taken from where it stands.
    pub position: [f64; 3],
    /// The entity's look direction (a unit vector).
    pub look_angle: [f64; 3],
}

impl Default for Holder {
    fn default() -> Self {
        Self {
            entity: None, player: false, projectile: None,
            stack: Stack::empty(), main_hand: Stack::empty(), off_hand: Stack::empty(),
            may_build: true, adventure: false, creative: false, sneak: false,
            position: [0.0; 3], look_angle: [0.0, 0.0, 1.0],
        }
    }
}

/// The projectile carrying the fluid: its id and the way it moves.
#[derive(Clone, Debug)]
pub(crate) struct ProjectileFacts {
    pub id: i32,
    pub delta: [f64; 3],
}

impl Holder {
    /// FluidEffectContext.breakRestricted: an adventure player who may not build.
    pub(crate) fn break_restricted(&self) -> bool { self.player && !self.may_build && self.adventure }
    /// The damage source a context makes when an effect does not name its own (createDamageSource).
    pub(crate) fn damage_source(&self) -> &'static str {
        if self.projectile.is_some() { "minecraft:mob_projectile" }
        else if self.player { "minecraft:player_attack" }
        else if self.entity.is_some() { "minecraft:mob_attack" }
        else { "minecraft:generic" }
    }
    /// The cause of effects from this context (getEffectSource): the entity, else the projectile.
    pub(crate) fn effect_source(&self) -> Option<i32> { self.entity.or(self.projectile.as_ref().map(|projectile| projectile.id)) }
}

/// An entity an effect acts on, or asks about. The queries are the facts the original reads off the game
/// entity; the actions are what it does to it, and each answers as the game's own call does (an effect is
/// reported given only when the game gives it, a hurt only when the damage lands).
pub(crate) trait EntityTarget {
    fn id(&self) -> i32;
    /// The entity's type, such as `minecraft:zombie`.
    fn kind(&self) -> &str;
    fn is_living(&self) -> bool;
    fn is_player(&self) -> bool;
    fn fire_immune(&self) -> bool;
    /// A named state predicate of Mantle's vocabulary (`mantle:fire_immune` and kin); None when this
    /// target cannot answer for that name.
    fn named_predicate(&self, name: &str) -> Option<bool>;
    fn in_type_tag(&self, tag: &Identifier) -> bool;
    fn has_effect(&self, effect: &Identifier) -> bool { self.active_effect(effect).is_some() }
    fn active_effect(&self, effect: &Identifier) -> Option<ActiveEffect>;
    /// LivingEntity.canBeAffected: the game would let this effect be given (immunities aside).
    fn can_be_affected(&self, effect: &NewEffect) -> bool;
    /// Any active effect lists this item among its curative items (the simulate half of curing).
    fn any_curative_effect(&self, item: &Stack) -> bool;
    fn remaining_fire_ticks(&self) -> i32;
    fn ticks_frozen(&self) -> i32;
    fn ticks_required_to_freeze(&self) -> i32;
    fn can_freeze(&self) -> bool;
    fn air_supply(&self) -> i32;
    fn max_air_supply(&self) -> i32;
    fn can_eat(&self, always_eat: bool) -> bool;
    fn position(&self) -> [f64; 3];
    fn look_angle(&self) -> [f64; 3];
    fn block_position(&self) -> BlockPos;
    /// The strong bones level of the helmet worn, and the helmet's item, for the calcified effect.
    fn strong_bones_helmet(&self) -> Option<(i32, Identifier)>;
    /// The total calcifiable armor level worn, for the calcified effect.
    fn calcifiable_level(&self) -> i32;
    fn set_remaining_fire_ticks(&mut self, ticks: i32);
    fn set_ticks_frozen(&mut self, ticks: i32);
    /// LivingEntity.addEffect: true when the effect was given.
    fn add_effect(&mut self, effect: NewEffect) -> bool;
    fn remove_effect(&mut self, effect: &Identifier) -> bool;
    /// LivingEntity.curePotionEffects: removes every effect this item cures; true when any went.
    fn cure_effects(&mut self, item: &Stack) -> bool;
    /// ToolAttackUtil's secondary hurt: true when the damage landed.
    fn hurt(&mut self, amount: f32, damage_type: &str) -> bool;
    /// FoodData.eat.
    fn eat(&mut self, hunger: i32, saturation: f32);
    /// ModifierUtil.foodConsumer.onConsume, for a representative item that was eaten as.
    fn food_eaten(&mut self, representative: &Stack, hunger: i32, saturation: f32);
    fn award_stat(&mut self, stat: &Identifier, amount: i32);
    fn set_air_supply(&mut self, air: i32);
    /// Entity.push, with the motion sync a player target needs.
    fn push(&mut self, delta: [f64; 3]);
    /// TeleportHelper.randomNearbyTeleport: attempts up to `chances` random spots within `diameter`.
    fn random_teleport(&mut self, diameter: i32, chances: i32);
}

/// setSecondsOnFire, shared by the effects that light a target: the fire lasts the longer of what burns
/// already and the new time.
fn set_seconds_on_fire(target: &mut dyn EntityTarget, seconds: i32) {
    let ticks = seconds.saturating_mul(20);
    if target.remaining_fire_ticks() < ticks { target.set_remaining_fire_ticks(ticks); }
}

/// What a use does (InteractionResult).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InteractionResult {
    Success,
    Consume,
    Pass,
    Fail,
}

impl InteractionResult {
    pub(crate) fn consumes_action(self) -> bool { matches!(self, Self::Success | Self::Consume) }
}

/// How a block answers a piston (PushReaction).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PushReaction {
    Normal,
    Destroy,
    Block,
    Ignore,
    PushOnly,
}

/// An explosion to make (CustomExplosion): the damage type is already chosen from the context.
#[derive(Clone, Debug)]
pub(crate) struct ExplosionSpec {
    pub at: [f64; 3],
    pub radius: f32,
    pub damage: f32,
    pub knockback: f32,
    pub place_fire: bool,
    pub interaction: BlockInteraction,
    pub damage_type: String,
    pub source_is_projectile: bool,
}

/// An area effect cloud to spawn (AreaEffectCloud): radius 1, radius on use -0.5, wait time 10 and a
/// duration of 600, so the radius per tick is -1/600, as the original sets it up.
#[derive(Clone, Debug)]
pub(crate) struct CloudSpec {
    pub at: [f64; 3],
    pub owner: Option<i32>,
    pub effects: Vec<NewEffect>,
}

/// A block to break with a fake tool (BreakBlockFluidEffect): a stick carrying the enchantments, broken by
/// the context's player when there is one (its drops and stat), else replaced by the fluid's own block.
#[derive(Clone, Debug)]
pub(crate) struct BreakSpec {
    pub pos: BlockPos,
    pub location: [f64; 3],
    pub enchantments: Vec<(Identifier, i32)>,
    pub by_player: bool,
    pub replacement: Option<BlockState>,
}

/// Which placement a PlaceBlockFluidEffect makes (its four paths).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PlaceMode {
    /// The named block has an item form: a BlockItem for it places.
    BlockItemForm,
    /// The named block has no item form: the state is set by hand, with the placement checks.
    Manual,
    /// No block named: a hand's item provides the block (BlockItemProviderCapability).
    ProviderHand,
    /// No block named and no hand provides one: the context's stack places.
    ContextStack,
}

/// A block placement to perform. `stack` is the stack being placed with; on a place it is the one the game
/// shrinks (unless the holder is in instabuild), which the caller reads back.
#[derive(Debug)]
pub(crate) struct PlaceSpec<'a> {
    pub mode: PlaceMode,
    pub pos: BlockPos,
    pub direction: Direction,
    pub location: [f64; 3],
    pub block: Option<BlockState>,
    pub stack: &'a mut Stack,
    pub holder: &'a Holder,
    pub action: FluidAction,
    /// The data's sound override, played instead of the block's place sound when set.
    pub sound: Option<String>,
}

/// A block interaction to perform (BlockInteractFluidEffect): the game's own use pipeline over the
/// holder's hands (both, when an entity holds them; the main hand alone otherwise).
#[derive(Debug)]
pub(crate) struct InteractSpec<'a> {
    pub pos: BlockPos,
    pub direction: Direction,
    pub location: [f64; 3],
    pub holder: &'a Holder,
    pub main_hand: &'a mut Stack,
    pub off_hand: &'a mut Stack,
    pub action: FluidAction,
}

/// An entity interaction to perform (EntityInteractFluidEffect) with the player's hands.
#[derive(Debug)]
pub(crate) struct EntityInteractSpec<'a> {
    pub target: i32,
    pub location: [f64; 3],
    pub holder: &'a Holder,
    pub main_hand: &'a mut Stack,
    pub off_hand: &'a mut Stack,
    pub action: FluidAction,
}

/// The level effects act on: block reads and writes, the entity and item reaches the block effects use, and
/// the facts about blocks and items that the game's registries answer. A context supplies one; the effects
/// never reach past it.
pub(crate) trait WorldAccess {
    fn block_state(&self, pos: BlockPos) -> Option<BlockState>;
    fn set_block(&mut self, pos: BlockPos, state: &BlockState) -> bool;
    /// The destroy-block particles and sound of a block going away (LevelEvent.PARTICLES_DESTROY_BLOCK).
    fn destroy_particles(&mut self, pos: BlockPos, state: &BlockState);
    fn play_sound(&mut self, at: [f64; 3], sound: &str, volume: f32, pitch: f32);
    /// ModifierUtil.dropItem: an item entity at the location.
    fn drop_item(&mut self, at: [f64; 3], stack: &Stack);
    fn explode(&mut self, spec: &ExplosionSpec);
    fn spawn_cloud(&mut self, spec: &CloudSpec);
    /// Breaks the block of a BreakSpec (drops at its location, spawnAfterBreak, particles); true when the
    /// block was broken.
    fn break_block(&mut self, spec: &BreakSpec) -> bool;
    /// MoveBlocksFluidEffect.destroyBlock: drops the block's resources and leaves air, with its particles.
    fn destroy_block(&mut self, pos: BlockPos);
    /// MoveBlocksFluidEffect.moveBlocks: the moving-piston row of `moving` blocks starting at `pos`,
    /// travelling `direction` (the way the row shifts) with piston facing `facing`.
    fn move_row(&mut self, pos: BlockPos, facing: Direction, direction: Direction, moving: i32, push: bool);
    fn push_reaction(&self, pos: BlockPos) -> PushReaction;
    fn has_block_entity(&self, pos: BlockPos) -> bool;
    /// PistonBaseBlock.isPushable, the game's own answer.
    fn is_pushable(&self, pos: BlockPos, direction: Direction, allow_destroy: bool, facing: Direction) -> bool;
    /// BlockState.getDestroySpeed: -1 for a block that cannot be broken.
    fn destroy_speed(&self, pos: BlockPos) -> f32;
    fn is_liquid_block(&self, pos: BlockPos) -> bool;
    /// Whether the block in front of a hit face can be replaced (isOffsetReplaceable).
    fn can_be_replaced(&self, pos: BlockPos) -> bool;
    fn within_world_border(&self, pos: BlockPos) -> bool;
    fn place_block(&mut self, spec: &mut PlaceSpec) -> InteractionResult;
    fn interact_block(&mut self, spec: &mut InteractSpec) -> InteractionResult;
    fn interact_entity(&mut self, spec: &mut EntityInteractSpec) -> InteractionResult;
    /// The block is one that weathers (WeatheringCopper).
    fn is_weathering_copper(&self, pos: BlockPos) -> bool;
    /// WeatheringCopper.applyChangeOverTime, server side.
    fn apply_weathering(&mut self, pos: BlockPos);
    /// MeltingRecipeLookup.findResult: the fluid melting this item at this temperature makes.
    fn melting_result(&self, item: &Identifier, temperature: i32) -> Option<FluidStack>;
    /// A fluid's own block, at source level (its own block).
    fn fluid_block(&self, fluid: &Identifier) -> Option<BlockState>;
    /// A fluid's flowing block at a flow level (1 to 7).
    fn flowing_fluid_block(&self, fluid: &Identifier, level: i32) -> Option<BlockState>;
    /// The item form of a block, when it has one.
    fn block_item(&self, block: &Identifier) -> Option<Identifier>;
    /// The block a stack provides for placing (BlockItemProviderCapability), when it provides one: a block item provides itself, a tool
    /// its provider module's item. The whole stack is the query, not just its item: a tool's provider lives in its data, so an item id
    /// alone cannot answer it.
    fn item_provided_block(&self, stack: &Stack) -> Option<BlockState>;
    /// TierSortingRegistry.isCorrectTierForDrops for a tier name such as `minecraft:stone`.
    fn correct_tier_for_drops(&self, tier: &str, pos: BlockPos) -> bool;
    /// A named block predicate of Mantle's (`mantle:blocks_motion`, `mantle:can_be_replaced`,
    /// `tconstruct:bush`); None when this world cannot answer for that name.
    fn named_block_predicate(&self, name: &str, pos: BlockPos, state: &BlockState) -> Option<bool>;
    /// The stack carries an adventure-mode place tag for the block at this position.
    fn adventure_place_allowed(&self, stack: &Stack, pos: BlockPos) -> bool;
    /// An ItemOutput as a stack: a tag resolves to the member the pack prefers.
    fn preferred_item(&self, output: &ItemOutput) -> Option<Stack>;
    /// PotionUtils.getPotion: the base potion's effects for the potion a fluid's NBT names.
    fn potion_effects(&self, tag: Option<&NbtCompound>) -> Vec<PotionEffect>;
    /// Every living entity inside this block's cell (the area effect's targets).
    fn living_in_block(&mut self, pos: BlockPos) -> Vec<&mut dyn EntityTarget>;
    fn random_float(&mut self) -> f32;
}

/// The shared half of a context: the world and who is using the fluid.
pub(crate) struct Env<'a> {
    pub world: &'a mut dyn WorldAccess,
    pub holder: Holder,
}

impl Env<'_> {
    /// FluidEffectContext.placeRestricted: an adventure player whose stack may not place this block.
    pub(crate) fn place_restricted(&self, stack: &Stack, pos: BlockPos) -> bool {
        self.holder.player && !self.holder.may_build && !self.world.adventure_place_allowed(stack, pos)
    }
}

/// A context for effects on an entity: the target, and where the fluid hit it.
pub(crate) struct EntityContext<'a> {
    pub env: Env<'a>,
    pub target: &'a mut dyn EntityTarget,
    pub location: [f64; 3],
}

/// A context for effects on a block: where it was hit, and on which face.
pub(crate) struct BlockContext<'a> {
    pub env: Env<'a>,
    pub pos: BlockPos,
    pub direction: Direction,
    pub location: [f64; 3],
}

impl BlockContext<'_> {
    /// The context shifted to another hit (OffsetBlockFluidEffect, conditional checks aside).
    pub(crate) fn at(&mut self, pos: BlockPos, direction: Direction) -> BlockContext<'_> {
        BlockContext { env: Env { world: &mut *self.env.world, holder: self.env.holder.clone() }, pos, direction, location: self.location }
    }
    /// The block in front of the hit face can be replaced (isOffsetReplaceable).
    pub(crate) fn offset_replaceable(&self) -> bool { self.env.world.can_be_replaced(self.direction.relative(self.pos)) }
}

/* The mob effect at the heart of several effects (FluidMobEffect) */

/// A mob effect definition (FluidMobEffect): the effect, its time in ticks (-1 is infinite), its level
/// starting at 1, and the items that cure it when named.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MobEffectDef {
    pub effect: Identifier,
    pub time: i32,
    pub level: i32,
    pub curative_items: Option<Vec<Identifier>>,
}

impl MobEffectDef {
    /// The instance this definition makes with a duration (effectWithTime).
    fn instance(&self, time: i32) -> NewEffect {
        NewEffect {
            effect: self.effect.clone(),
            duration: time,
            amplifier: self.level - 1,
            curative_items: self.curative_items.as_ref().map(|items| items.iter().map(|item| Stack::of(item.clone(), 1)).collect()),
        }
    }

    fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        let effect = id_of(fields.get("effect").ok_or("mob effect needs an effect")?)?;
        let time = match fields.get("time") {
            None | Some(Value::Null) => -1,
            Some(value) => {
                let time = int_of(value, "time")?;
                if time != -1 && time < 1 { return Err("time must be -1 (infinite) or at least 1".into()); }
                time
            }
        };
        let level = match fields.get("level") {
            None | Some(Value::Null) => 1,
            Some(value) => {
                let level = int_of(value, "level")?;
                if level < 1 { return Err("level must be at least 1".into()); }
                level
            }
        };
        let curative_items = match fields.get("curative_items") {
            None | Some(Value::Null) => None,
            Some(Value::Array(items)) => Some(items.iter().map(id_of).collect::<Result<_, _>>()?),
            Some(_) => return Err("curative_items must be an array of item ids".into()),
        };
        Ok(Self { effect, time, level, curative_items })
    }
}

/// FluidMobEffect.apply: how much of the level an effect definition uses on a target, combining with what
/// the target already has according to the time action.
fn apply_mob_effect(def: &MobEffectDef, target: &mut dyn EntityTarget, level: EffectLevel, time_action: TimeAction, action: FluidAction) -> f32 {
    // Infinite requires a full level, costs 1 and sets the full (infinite) time.
    if def.time == -1 && !level.is_full() { return 0.0; }
    let (used, time);
    if def.time == -1 {
        used = 1.0;
        time = -1;
    } else if let Some(existing) = target.active_effect(&def.effect).filter(|existing| existing.amplifier >= def.level - 1) {
        // An existing effect of a higher level is never extended; an equal one combines per the action.
        if existing.amplifier > def.level - 1 { return 0.0; }
        match time_action {
            TimeAction::Add => {
                let extra = (def.time as f32 * level.value) as i32;
                if extra <= 0 { return 0.0; }
                time = existing.duration + extra;
                used = level.value;
            }
            TimeAction::Set => {
                let existing_level = existing.duration as f32 / def.time as f32;
                let effective = level.effective(existing_level);
                if effective < existing_level { return 0.0; }
                used = effective - existing_level;
                time = (def.time as f32 * effective) as i32;
            }
        }
    } else {
        time = (def.time as f32 * level.value) as i32;
        used = level.value;
    }
    if time != 0 {
        let instance = def.instance(time);
        if action.simulate() { return if target.can_be_affected(&instance) { used } else { 0.0 }; }
        return if target.add_effect(instance) { used } else { 0.0 };
    }
    0.0
}

/* Entity effects */

/// A pair of damage types for a hit done in melee and at range (DamageFluidEffect.DamageTypePair).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DamagePair {
    pub melee: String,
    pub ranged: String,
}

impl DamagePair {
    fn decode(value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("damage type must be an object")?;
        only_keys(fields, "damage type", &["melee", "ranged"])?;
        let part = |key: &str| fields.get(key).and_then(Value::as_str).map(str::to_owned).ok_or_else(|| format!("damage type needs {key}"));
        Ok(Self { melee: part("melee")?, ranged: part("ranged")? })
    }
}

/// An effect on an entity, one of the loaders registered for the entity list (and the general list).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum EntityEffect {
    Conditional { target: LivingPredicate, effect: Box<EntityEffect> },
    Scaling { effects: Vec<(f32, EntityEffect)> },
    Alternatives { effects: Vec<EntityEffect> },
    Sequence { effects: Vec<EntityEffect> },
    Calcified,
    Extinguish,
    Teleport { diameter: LevelingInt, chances: LevelingInt },
    CureEffects { item: ItemOutput },
    RemoveEffect { effect: Identifier },
    MobEffect { def: MobEffectDef, action: TimeAction },
    Potion { scale: f32, nbt: TagPredicate },
    Damage { damage: f32, damage_type: Option<DamagePair> },
    RestoreHunger { hunger: i32, saturation: f32, can_always_eat: bool, representative: Option<ItemOutput> },
    Fire { action: TimeAction, time: i32 },
    Freeze { action: TimeAction, time: i32 },
    AwardStat { stat: Identifier, amount: i32 },
    AddBreath { amount: i32 },
    PushEntity { value: LevelingValue },
    Interact,
    General(GeneralEffect),
}

fn float_field(fields: &Map<String, Value>, key: &str, min: f32) -> Result<f32, String> {
    match fields.get(key) {
        None => Err(format!("{key} missing")),
        Some(Value::Number(number)) => {
            let value = number.as_f64().ok_or_else(|| format!("{key} must be a number"))? as f32;
            if value >= min { Ok(value) } else { Err(format!("{key} must be at least {min}")) }
        }
        Some(_) => Err(format!("{key} must be a number")),
    }
}

fn short_field(fields: &Map<String, Value>, key: &str) -> Result<i32, String> {
    match fields.get(key) {
        None => Err(format!("{key} missing")),
        Some(value) => {
            let value = int_of(value, key)?;
            if (-32768..=32767).contains(&value) { Ok(value) } else { Err(format!("{key} must fit in a short")) }
        }
    }
}

fn bool_field(fields: &Map<String, Value>, key: &str, default: bool) -> Result<bool, String> {
    match fields.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Bool(flag)) => Ok(*flag),
        Some(_) => Err(format!("{key} must be a boolean")),
    }
}

/// The fields of one effect object: `type` names it, the rest are the loader's.
fn effect_fields<'a>(value: &'a Value, what: &str) -> Result<(&'a Map<String, Value>, &'a str), String> {
    let fields = value.as_object().ok_or_else(|| format!("{what} effect must be an object"))?;
    let kind = fields.get("type").and_then(Value::as_str).ok_or_else(|| format!("{what} effect needs a type"))?;
    Ok((fields, kind))
}

/// A nested effect list (`effects` of the combinators), at least one entry.
fn effect_list<T>(fields: &Map<String, Value>, decode: fn(&Value) -> Result<T, String>) -> Result<Vec<T>, String> {
    let list = fields.get("effects").and_then(Value::as_array).ok_or("effects must be an array")?;
    if list.is_empty() { return Err("effects must not be empty".into()); }
    list.iter().map(decode).collect()
}

impl EntityEffect {
    pub(crate) fn decode(value: &Value) -> Result<Self, String> {
        let (fields, kind) = effect_fields(value, "entity")?;
        Ok(match kind {
            "tconstruct:conditional" => {
                only_keys(fields, "entity effect", &["type", "target", "effect"])?;
                let target = match fields.get("target") {
                    None | Some(Value::Null) => LivingPredicate::Any,
                    Some(value) => LivingPredicate::decode(value)?,
                };
                Self::Conditional { target, effect: Box::new(Self::decode(fields.get("effect").ok_or("conditional needs an effect")?)?) }
            }
            "tconstruct:scaling" => {
                only_keys(fields, "entity effect", &["type", "effects"])?;
                let entries = fields.get("effects").and_then(Value::as_array).ok_or("effects must be an array")?;
                if entries.is_empty() { return Err("effects must not be empty".into()); }
                let mut effects = Vec::new();
                for entry in entries {
                    let entry = entry.as_object().ok_or("scaling entry must be an object")?;
                    only_keys(entry, "scaling entry", &["effect", "level"])?;
                    effects.push((float_field(entry, "level", 0.0)?, Self::decode(entry.get("effect").ok_or("scaling entry needs an effect")?)?));
                }
                Self::Scaling { effects }
            }
            "tconstruct:alternatives" => { only_keys(fields, "entity effect", &["type", "effects"])?; Self::Alternatives { effects: effect_list(fields, Self::decode)? } }
            "tconstruct:sequence" => { only_keys(fields, "entity effect", &["type", "effects"])?; Self::Sequence { effects: effect_list(fields, Self::decode)? } }
            "tconstruct:calcified" => { only_keys(fields, "entity effect", &["type"])?; Self::Calcified }
            "tconstruct:extinguish" => { only_keys(fields, "entity effect", &["type"])?; Self::Extinguish }
            "tconstruct:teleport" => {
                only_keys(fields, "entity effect", &["type", "diameter", "chances"])?;
                let part = |key: &str| match fields.get(key) { None | Some(Value::Null) => Ok(LevelingInt::flat(16)), Some(value) => LevelingInt::decode(value) };
                Self::Teleport { diameter: part("diameter")?, chances: part("chances")? }
            }
            "tconstruct:cure_effects" => {
                only_keys(fields, "entity effect", &["type", "item"])?;
                Self::CureEffects { item: ItemOutput::decode(fields.get("item").ok_or("cure_effects needs an item")?, true)? }
            }
            "tconstruct:remove_effect" => {
                only_keys(fields, "entity effect", &["type", "effect"])?;
                Self::RemoveEffect { effect: id_of(fields.get("effect").ok_or("remove_effect needs an effect")?)? }
            }
            "tconstruct:mob_effect" => {
                only_keys(fields, "entity effect", &["type", "effect", "time", "level", "curative_items", "action"])?;
                Self::MobEffect {
                    def: MobEffectDef::decode(fields)?,
                    action: TimeAction::decode(fields.get("action").ok_or("mob_effect needs an action")?)?,
                }
            }
            "tconstruct:potion" => {
                only_keys(fields, "entity effect", &["type", "scale", "nbt"])?;
                Self::Potion {
                    scale: float_field(fields, "scale", 0.0)?,
                    nbt: match fields.get("nbt") { None | Some(Value::Null) => TagPredicate::any(), Some(value) => TagPredicate::decode(value)? },
                }
            }
            "tconstruct:damage" => {
                only_keys(fields, "entity effect", &["type", "damage", "damage_type"])?;
                Self::Damage {
                    damage: float_field(fields, "damage", 0.0)?,
                    damage_type: match fields.get("damage_type") { None | Some(Value::Null) => None, Some(value) => Some(DamagePair::decode(value)?) },
                }
            }
            "tconstruct:restore_hunger" => {
                only_keys(fields, "entity effect", &["type", "hunger", "saturation", "can_always_eat", "representative_item"])?;
                Self::RestoreHunger {
                    hunger: int_field(fields, "hunger", 1, None)?,
                    saturation: float_field(fields, "saturation", 0.0)?,
                    can_always_eat: bool_field(fields, "can_always_eat", false)?,
                    representative: match fields.get("representative_item") {
                        None | Some(Value::Null) => None,
                        Some(value) => Some(ItemOutput::decode(value, false)?),
                    },
                }
            }
            "tconstruct:fire" => {
                only_keys(fields, "entity effect", &["type", "action", "time"])?;
                Self::Fire { action: TimeAction::decode(fields.get("action").ok_or("fire needs an action")?)?, time: int_field(fields, "time", 1, None)? }
            }
            "tconstruct:freeze" => {
                only_keys(fields, "entity effect", &["type", "action", "time"])?;
                Self::Freeze { action: TimeAction::decode(fields.get("action").ok_or("freeze needs an action")?)?, time: int_field(fields, "time", 1, None)? }
            }
            "tconstruct:award_stat" => {
                only_keys(fields, "entity effect", &["type", "stat", "amount"])?;
                Self::AwardStat { stat: id_of(fields.get("stat").ok_or("award_stat needs a stat")?)?, amount: short_field(fields, "amount")? }
            }
            "tconstruct:add_breath" => {
                only_keys(fields, "entity effect", &["type", "amount"])?;
                Self::AddBreath { amount: short_field(fields, "amount")? }
            }
            "tconstruct:push_entity" => {
                only_keys(fields, "entity effect", &["type", "flat", "each_level"])?;
                let mut value = fields.clone();
                value.remove("type");
                Self::PushEntity { value: LevelingValue::decode(&Value::Object(value))? }
            }
            "tconstruct:interact" => { only_keys(fields, "entity effect", &["type"])?; Self::Interact }
            _ => return Ok(Self::General(GeneralEffect::decode(fields, kind)?)),
        })
    }

    /// Runs the effect: the part of the level it used (FluidEffect.apply).
    pub(crate) fn apply(&self, fluid: &FluidStack, level: EffectLevel, context: &mut EntityContext, action: FluidAction) -> f32 {
        match self {
            Self::Conditional { target, effect } => {
                if target.matches(context.env.world, context.target) { effect.apply(fluid, level, context, action) } else { 0.0 }
            }
            Self::Scaling { effects } => {
                // The last entry whose level the scale reaches wins, run at the divided level.
                for (entry_level, effect) in effects.iter().rev() {
                    if level.value >= *entry_level {
                        if *entry_level == 0.0 { return 0.0; }
                        return effect.apply(fluid, level.divide(*entry_level), context, action) * entry_level;
                    }
                }
                0.0
            }
            Self::Alternatives { effects } => {
                for effect in effects {
                    let applied = effect.apply(fluid, level, context, action);
                    if applied > 0.0 { return applied; }
                }
                0.0
            }
            Self::Sequence { effects } => {
                let mut max_applied = 0.0;
                for effect in effects {
                    let applied = effect.apply(fluid, level, context, action);
                    if applied <= 0.0 { break; }
                    if applied > max_applied { max_applied = applied; }
                }
                max_applied
            }
            Self::Calcified => calcified(level, context, action),
            Self::Extinguish => {
                if context.target.remaining_fire_ticks() > 0 && level.is_full() {
                    if action.execute() { context.target.set_remaining_fire_ticks(0); }
                    1.0
                } else { 0.0 }
            }
            Self::Teleport { diameter, chances } => {
                if !context.target.is_living() { return 0.0; }
                let flat = diameter.is_flat() && chances.is_flat();
                if flat && !level.is_full() { return 0.0; }
                if action.execute() {
                    context.target.random_teleport(diameter.compute(level.value), chances.compute(level.value));
                }
                if flat { 1.0 } else { level.value }
            }
            Self::CureEffects { item } => {
                if !context.target.is_living() || !level.is_full() { return 0.0; }
                let Some(stack) = context.env.world.preferred_item(item) else { return 0.0 };
                if action.simulate() {
                    if context.target.any_curative_effect(&stack) { 1.0 } else { 0.0 }
                } else if context.target.cure_effects(&stack) { 1.0 } else { 0.0 }
            }
            Self::RemoveEffect { effect } => {
                if !context.target.is_living() || !level.is_full() { return 0.0; }
                if action.simulate() {
                    if context.target.has_effect(effect) { 1.0 } else { 0.0 }
                } else if context.target.remove_effect(effect) { 1.0 } else { 0.0 }
            }
            Self::MobEffect { def, action: time_action } => {
                if !context.target.is_living() { return 0.0; }
                apply_mob_effect(def, context.target, level, *time_action, action)
            }
            Self::Potion { scale, nbt } => potion(*scale, nbt, fluid, level, context, action),
            Self::Damage { damage, damage_type } => {
                let value = level.value;
                if action.simulate() { return value; }
                let damage_type = match damage_type {
                    Some(pair) => {
                        if context.env.holder.projectile.is_some() {
                            // TinkerEffects.needsEnderferenceOverride: the melee type only for an
                            // enderman carrying the enderference effect — either half alone (an
                            // enderman without the effect, another kind carrying it) keeps the
                            // ranged type, as DamageFluidEffect reads the living target.
                            if context.target.kind() == "minecraft:enderman" && context.target.has_effect(&Identifier::new("tconstruct", "enderference").expect("valid id")) { pair.melee.clone() } else { pair.ranged.clone() }
                        } else { pair.melee.clone() }
                    }
                    None => context.env.holder.damage_source().to_owned(),
                };
                if context.target.hurt(damage * value, &damage_type) { value } else { 0.0 }
            }
            Self::RestoreHunger { hunger, saturation, can_always_eat, representative } => {
                if !context.target.is_player() || !context.target.can_eat(*can_always_eat) { return 0.0; }
                let value = level.value;
                if action.execute() {
                    let final_hunger = (*hunger as f32 * value) as i32;
                    context.target.eat(final_hunger, *saturation);
                    if let Some(representative) = representative {
                        if let Some(stack) = context.env.world.preferred_item(representative) {
                            if !stack.is_empty() { context.target.food_eaten(&stack, final_hunger, *saturation); }
                        }
                    }
                }
                value
            }
            Self::Fire { action: time_action, time } => {
                if context.target.fire_immune() { return 0.0; }
                match time_action {
                    TimeAction::Add => {
                        let value = level.value;
                        if action.execute() {
                            let seconds = (*time as f32 * value).round() as i32 + (context.target.remaining_fire_ticks() + 10) / 20;
                            set_seconds_on_fire(context.target, seconds);
                        }
                        value
                    }
                    TimeAction::Set => {
                        let existing = context.target.remaining_fire_ticks() as f32 / 20.0 / *time as f32;
                        let effective = level.effective(existing);
                        if action.execute() { set_seconds_on_fire(context.target, (*time as f32 * effective).round() as i32); }
                        effective - existing
                    }
                }
            }
            Self::Freeze { action: time_action, time } => {
                if !context.target.can_freeze() { return 0.0; }
                match time_action {
                    TimeAction::Add => {
                        let value = level.value;
                        if action.execute() {
                            let frozen = context.target.ticks_required_to_freeze().max(context.target.ticks_frozen()) + (*time as f32 * value).round() as i32;
                            context.target.set_ticks_frozen(frozen);
                            context.target.set_remaining_fire_ticks(0);
                        }
                        value
                    }
                    TimeAction::Set => {
                        let required = context.target.ticks_required_to_freeze();
                        let frozen = context.target.ticks_frozen();
                        let existing = if frozen < required { 0.0 } else { (frozen - required) as f32 / *time as f32 };
                        let effective = level.effective(existing);
                        if action.execute() {
                            context.target.set_ticks_frozen(required + (*time as f32 * 2.0 * effective).round() as i32);
                            context.target.set_remaining_fire_ticks(0);
                        }
                        effective - existing
                    }
                }
            }
            Self::AwardStat { stat, amount } => {
                if !context.target.is_living() || !context.target.is_player() { return 0.0; }
                if action.execute() { context.target.award_stat(stat, (*amount as f32 * level.value).round() as i32); }
                level.value
            }
            Self::AddBreath { amount } => {
                if !context.target.is_living() { return 0.0; }
                let amount = *amount;
                let (current, max) = (context.target.air_supply(), context.target.max_air_supply());
                if action.execute() {
                    let change = (amount as f32 * level.value).round() as i32;
                    context.target.set_air_supply((current + change).clamp(-20, max));
                }
                let max_change = if amount > 0 { max - current } else { current };
                level.compute_used(max_change as f32 / amount.unsigned_abs() as f32)
            }
            Self::PushEntity { value } => {
                if value.is_flat() && !level.is_full() { return 0.0; }
                let direction = match &context.env.holder.projectile {
                    Some(projectile) => projectile.delta,
                    None => {
                        if context.env.holder.entity.is_none() || context.env.holder.entity == Some(context.target.id()) {
                            context.target.look_angle()
                        } else {
                            let (target, holder) = (context.target.position(), context.env.holder.position);
                            [target[0] - holder[0], target[1] - holder[1], target[2] - holder[2]]
                        }
                    }
                };
                let length = (direction[0] * direction[0] + direction[1] * direction[1] + direction[2] * direction[2]).sqrt();
                if length > 0.0 {
                    let scale = value.compute(level.value) as f64 / length;
                    if action.execute() { context.target.push([direction[0] * scale, direction[1] * scale, direction[2] * scale]); }
                }
                if value.is_flat() { 1.0 } else { level.value }
            }
            Self::Interact => {
                if !context.env.holder.player { return 0.0; }
                if !context.env.world.within_world_border(context.target.block_position()) { return 0.0; }
                if action.simulate() { return 1.0; }
                let holder = context.env.holder.clone();
                let mut spec = EntityInteractSpec {
                    target: context.target.id(),
                    location: context.location,
                    holder: &holder,
                    main_hand: &mut context.env.holder.main_hand,
                    off_hand: &mut context.env.holder.off_hand,
                    action,
                };
                let result = context.env.world.interact_entity(&mut spec);
                if result.consumes_action() { 1.0 } else { 0.0 }
            }
            Self::General(general) => general.apply(fluid, level, &mut context.env, context.target.block_position(), context.location, action),
        }
    }
}

/// StrongBonesModifier.FLUID_EFFECT (the calcified effect): drinking in the fluid, the target's strong
/// bones helmet and calcifiable armor give it resistance and the calcified effect, scaled by the level.
fn calcified(level: EffectLevel, context: &mut EntityContext, action: FluidAction) -> f32 {
    if !context.target.is_living() { return 0.0; }
    let target = &mut *context.target;
    let each_level = (200.0 * level.value) as i32;
    let mut did_something = false;
    if let Some((bones, helmet)) = target.strong_bones_helmet() {
        if bones > 0 {
            let instance = NewEffect {
                effect: Identifier::new("minecraft", "resistance").expect("valid id"),
                duration: each_level * bones,
                amplifier: 0,
                curative_items: Some(vec![Stack::of(helmet, 1)]),
            };
            did_something = if action.execute() { target.add_effect(instance) } else { target.can_be_affected(&instance) };
            if did_something && action.simulate() { return level.value; }
        }
    }
    let calcifiable = target.calcifiable_level();
    if calcifiable > 0 {
        let instance = NewEffect {
            effect: Identifier::new("tconstruct", "calcified").expect("valid id"),
            duration: each_level * calcifiable,
            amplifier: 0,
            curative_items: None,
        };
        did_something |= if action.execute() { target.add_effect(instance) } else { target.can_be_affected(&instance) };
    }
    if did_something { level.value } else { 0.0 }
}

/// PotionFluidEffect.apply: the effects of the potion the fluid's NBT names, scaled; instant effects apply
/// at the full value, others extend what the target already has like a set mob effect. The original forces
/// the target's invulnerable time to 0 while it applies them; that is the target's own business here.
fn potion(scale: f32, nbt: &TagPredicate, fluid: &FluidStack, level: EffectLevel, context: &mut EntityContext, action: FluidAction) -> f32 {
    if !context.target.is_living() || !nbt.test(fluid.tag.as_ref()) { return 0.0; }
    let effects = context.env.world.potion_effects(fluid.tag.as_ref());
    if effects.is_empty() { return 0.0; }
    let mut used = 0.0;
    for instance in &effects {
        if instance.instant {
            used = level.value;
            if action.execute() {
                context.target.add_effect(NewEffect { effect: instance.effect.clone(), duration: instance.duration, amplifier: instance.amplifier, curative_items: None });
            }
            continue;
        }
        let duration;
        match context.target.active_effect(&instance.effect) {
            Some(existing) if existing.amplifier >= instance.amplifier => {
                if existing.amplifier > instance.amplifier { continue; }
                let existing_level = existing.duration as f32 / scale / instance.duration as f32;
                let effective = level.effective(existing_level);
                if effective <= existing_level { continue; }
                duration = (instance.duration as f32 * scale * effective) as i32;
                used = used.max(effective - existing_level);
            }
            _ => {
                used = level.value;
                duration = (instance.duration as f32 * scale * used) as i32;
            }
        }
        if action.execute() {
            context.target.add_effect(NewEffect { effect: instance.effect.clone(), duration, amplifier: instance.amplifier, curative_items: None });
        }
    }
    used
}

/* Effects registered for both contexts (the general list) */

/// An effect that runs the same on a block and on an entity.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum GeneralEffect {
    DropItem { item: ItemOutput },
    Explosion { radius: LevelingValue, damage: LevelingValue, knockback: LevelingValue, place_fire: bool, block_interaction: BlockInteraction },
    SetBlock { block: BlockState },
    AreaMobEffect { def: MobEffectDef, action: TimeAction, group_cost: GroupCost },
}

/// A block state as the data writes one (BlockStateLoadable): a plain id (with `[properties]` if it
/// carries any), or an object naming the block and its properties.
fn decode_block_state(value: &Value) -> Result<BlockState, String> {
    match value {
        Value::String(text) => BlockState::parse(text).map_err(|error| error.to_string()),
        Value::Object(fields) => {
            only_keys(fields, "block state", &["block", "properties"])?;
            let block = id_of(fields.get("block").ok_or("block state needs a block")?)?;
            let mut state = BlockState::new(block);
            if let Some(properties) = fields.get("properties") {
                let properties = properties.as_object().ok_or("properties must be an object")?;
                for (key, value) in properties {
                    let value = value.as_str().ok_or("property values must be strings")?;
                    state = state.with(key, value);
                }
            }
            Ok(state)
        }
        _ => Err("block state must be an id or an object".into()),
    }
}

fn direction_of(text: &str) -> Result<Direction, String> {
    Direction::parse(text).ok_or_else(|| format!("unknown direction {text}"))
}

impl GeneralEffect {
    fn decode(fields: &Map<String, Value>, kind: &str) -> Result<Self, String> {
        Ok(match kind {
            "tconstruct:drop_item" => {
                let mut stack = fields.clone();
                stack.remove("type");
                Self::DropItem { item: ItemOutput::decode(&Value::Object(stack), true)? }
            }
            "tconstruct:explosion" => {
                only_keys(fields, "general effect", &["type", "radius", "damage", "knockback", "place_fire", "block_interaction"])?;
                Self::Explosion {
                    radius: LevelingValue::decode(fields.get("radius").ok_or("explosion needs a radius")?)?,
                    damage: LevelingValue::decode(fields.get("damage").ok_or("explosion needs damage")?)?,
                    knockback: match fields.get("knockback") { None | Some(Value::Null) => LevelingValue::flat(1.0), Some(value) => LevelingValue::decode(value)? },
                    place_fire: bool_field(fields, "place_fire", false)?,
                    block_interaction: BlockInteraction::decode(fields.get("block_interaction").ok_or("explosion needs a block_interaction")?)?,
                }
            }
            "tconstruct:set_block" => {
                only_keys(fields, "general effect", &["type", "block"])?;
                Self::SetBlock { block: decode_block_state(fields.get("block").ok_or("set_block needs a block")?)? }
            }
            "tconstruct:area_mob_effect" => {
                only_keys(fields, "general effect", &["type", "effect", "time", "level", "curative_items", "action", "group_cost"])?;
                let mut def_fields = fields.clone();
                def_fields.remove("type");
                def_fields.remove("action");
                def_fields.remove("group_cost");
                Self::AreaMobEffect {
                    def: MobEffectDef::decode(&def_fields)?,
                    action: TimeAction::decode(fields.get("action").ok_or("area_mob_effect needs an action")?)?,
                    group_cost: GroupCost::decode(fields.get("group_cost").ok_or("area_mob_effect needs a group_cost")?)?,
                }
            }
            _ => return Err(format!("unimplemented general effect type {kind}")),
        })
    }

    /// Runs the effect at a block position (the target's, for an entity context).
    pub(crate) fn apply(&self, _fluid: &FluidStack, level: EffectLevel, env: &mut Env, pos: BlockPos, location: [f64; 3], action: FluidAction) -> f32 {
        match self {
            Self::DropItem { item } => {
                let Some(stack) = env.world.preferred_item(item) else { return 0.0 };
                let count = (level.value * stack.count as f32) as i32;
                if count <= 0 { return 0.0; }
                if action.execute() {
                    // The original drops the computed count times the stack's own count again
                    // (copyStackWithSize over a stack that already carries it); kept as written.
                    let mut dropped = stack.clone();
                    dropped.count = count * stack.count;
                    env.world.drop_item(location, &dropped);
                }
                count as f32 / stack.count as f32
            }
            Self::Explosion { radius, damage, knockback, place_fire, block_interaction } => {
                let flat = radius.is_flat() && damage.is_flat() && knockback.is_flat();
                if flat && !level.is_full() { return 0.0; }
                let value = level.value;
                let radius = radius.compute_for_scale(value);
                if radius < 0.5 { return 0.0; }
                if action.execute() {
                    let (melee, ranged) = if env.holder.entity.is_some() {
                        ("tconstruct:mob_explosion_melee", "tconstruct:mob_explosion_ranged")
                    } else {
                        ("tconstruct:explosion_melee", "tconstruct:explosion_ranged")
                    };
                    let restricted = env.holder.break_restricted();
                    env.world.explode(&ExplosionSpec {
                        at: location,
                        radius,
                        damage: damage.compute_for_scale(value),
                        knockback: knockback.compute_for_scale(value),
                        place_fire: *place_fire && !restricted,
                        interaction: if restricted { BlockInteraction::Keep } else { *block_interaction },
                        damage_type: if env.holder.projectile.is_some() { ranged } else { melee }.to_owned(),
                        source_is_projectile: env.holder.projectile.is_some(),
                    });
                }
                if flat { 1.0 } else { value }
            }
            Self::SetBlock { block } => {
                if !level.is_full() { return 0.0; }
                let Some(original) = env.world.block_state(pos) else { return 0.0 };
                if original != *block && !env.holder.break_restricted() {
                    if action.execute() {
                        let air = original.block.to_string() == "minecraft:air";
                        if env.world.set_block(pos, block) && !air { env.world.destroy_particles(pos, &original); }
                    }
                    return 1.0;
                }
                0.0
            }
            Self::AreaMobEffect { def, action: time_action, group_cost } => {
                let mut used = 0.0;
                let mut level = level;
                for target in env.world.living_in_block(pos) {
                    let local = apply_mob_effect(def, target, level, *time_action, action);
                    match group_cost {
                        GroupCost::Sum => { used += local; level = level.subtract(local); }
                        GroupCost::Max => { if local > used { used = local; } }
                    }
                }
                used
            }
        }
    }
}

/* Block effects */

/// An effect on a block, one of the loaders registered for the block list (and the general list).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BlockEffect {
    Conditional { block: BlockPredicate, effect: Box<BlockEffect> },
    Scaling { effects: Vec<(f32, BlockEffect)> },
    Alternatives { effects: Vec<BlockEffect> },
    Sequence { effects: Vec<BlockEffect> },
    Offset { effect: Box<BlockEffect>, direction: Option<Direction> },
    Weather,
    RemoveBlock,
    PlaceBlock { block: Option<BlockState>, sound: Option<String> },
    BreakBlock { hardness: f32, enchantments: Vec<(Identifier, i32)> },
    MobEffectCloud { effects: Vec<MobEffectDef> },
    PotionCloud { scale: f32, nbt: TagPredicate },
    MoveBlock { push: bool, sound: String },
    Interact,
    MeltBlock { blocks: BlockPredicate, min_amount: i32, temperature: i32 },
    General(GeneralEffect),
}

impl BlockEffect {
    pub(crate) fn decode(value: &Value) -> Result<Self, String> {
        let (fields, kind) = effect_fields(value, "block")?;
        Ok(match kind {
            "tconstruct:conditional" => {
                only_keys(fields, "block effect", &["type", "block", "effect"])?;
                let block = match fields.get("block") {
                    None | Some(Value::Null) => BlockPredicate::Any,
                    Some(value) => BlockPredicate::decode(value)?,
                };
                Self::Conditional { block, effect: Box::new(Self::decode(fields.get("effect").ok_or("conditional needs an effect")?)?) }
            }
            "tconstruct:scaling" => {
                only_keys(fields, "block effect", &["type", "effects"])?;
                let entries = fields.get("effects").and_then(Value::as_array).ok_or("effects must be an array")?;
                if entries.is_empty() { return Err("effects must not be empty".into()); }
                let mut effects = Vec::new();
                for entry in entries {
                    let entry = entry.as_object().ok_or("scaling entry must be an object")?;
                    only_keys(entry, "scaling entry", &["effect", "level"])?;
                    effects.push((float_field(entry, "level", 0.0)?, Self::decode(entry.get("effect").ok_or("scaling entry needs an effect")?)?));
                }
                Self::Scaling { effects }
            }
            "tconstruct:alternatives" => { only_keys(fields, "block effect", &["type", "effects"])?; Self::Alternatives { effects: effect_list(fields, Self::decode)? } }
            "tconstruct:sequence" => { only_keys(fields, "block effect", &["type", "effects"])?; Self::Sequence { effects: effect_list(fields, Self::decode)? } }
            "tconstruct:offset" => {
                let inner_kind = fields.get("offset_type").and_then(Value::as_str).ok_or("offset needs an offset_type")?;
                let direction = match fields.get("offset_direction") {
                    None | Some(Value::Null) => None,
                    Some(Value::String(text)) => Some(direction_of(text)?),
                    Some(_) => return Err("offset_direction must be a direction name".into()),
                };
                let mut inner = fields.clone();
                inner.remove("type");
                inner.remove("offset_type");
                inner.remove("offset_direction");
                inner.insert("type".into(), Value::String(inner_kind.to_owned()));
                Self::Offset { effect: Box::new(Self::decode(&Value::Object(inner))?), direction }
            }
            "tconstruct:weather" => { only_keys(fields, "block effect", &["type"])?; Self::Weather }
            "tconstruct:remove_block" => { only_keys(fields, "block effect", &["type"])?; Self::RemoveBlock }
            "tconstruct:place_block" => {
                only_keys(fields, "block effect", &["type", "block", "sound"])?;
                Self::PlaceBlock {
                    block: match fields.get("block") { None | Some(Value::Null) => None, Some(value) => Some(decode_block_state(value)?) },
                    sound: match fields.get("sound") {
                        None | Some(Value::Null) => None,
                        Some(Value::String(text)) => Some(text.clone()),
                        Some(_) => return Err("sound must be a sound event id".into()),
                    },
                }
            }
            "tconstruct:break_block" => {
                only_keys(fields, "block effect", &["type", "hardness", "enchantments"])?;
                let hardness = match fields.get("hardness") {
                    None | Some(Value::Null) => 0.0,
                    Some(value) => {
                        let hardness = value.as_f64().ok_or("hardness must be a number")? as f32;
                        if hardness < 0.0 { return Err("hardness must be at least 0".into()); }
                        hardness
                    }
                };
                let mut enchantments = Vec::new();
                if let Some(map) = fields.get("enchantments") {
                    let map = map.as_object().ok_or("enchantments must be an object")?;
                    for (name, level) in map {
                        let level = int_of(level, "enchantment level")?;
                        if level < 1 { return Err("enchantment level must be at least 1".into()); }
                        enchantments.push((Identifier::parse(name).map_err(|error| error.to_string())?, level));
                    }
                }
                Self::BreakBlock { hardness, enchantments }
            }
            "tconstruct:mob_effect_cloud" => {
                only_keys(fields, "block effect", &["type", "effects"])?;
                let entries = fields.get("effects").and_then(Value::as_array).ok_or("effects must be an array")?;
                if entries.is_empty() { return Err("effects must not be empty".into()); }
                let mut effects = Vec::new();
                for entry in entries {
                    let entry = entry.as_object().ok_or("cloud effect must be an object")?;
                    only_keys(entry, "cloud effect", &["effect", "time", "level", "curative_items"])?;
                    effects.push(MobEffectDef::decode(entry)?);
                }
                Self::MobEffectCloud { effects }
            }
            "tconstruct:potion_cloud" => {
                only_keys(fields, "block effect", &["type", "scale", "nbt"])?;
                let scale = fields.get("scale").and_then(Value::as_f64).ok_or("potion_cloud needs a scale")? as f32;
                if scale < 0.0 { return Err("scale must be at least 0".into()); }
                Self::PotionCloud {
                    scale,
                    nbt: match fields.get("nbt") { None | Some(Value::Null) => TagPredicate::any(), Some(value) => TagPredicate::decode(value)? },
                }
            }
            "tconstruct:move_block" => {
                only_keys(fields, "block effect", &["type", "push", "sound"])?;
                Self::MoveBlock {
                    push: bool_field(fields, "push", false).and_then(|push| if fields.contains_key("push") { Ok(push) } else { Err("move_block needs push".into()) })?,
                    sound: fields.get("sound").and_then(Value::as_str).ok_or("move_block needs a sound")?.to_owned(),
                }
            }
            "tconstruct:interact" => { only_keys(fields, "block effect", &["type"])?; Self::Interact }
            "tconstruct:melt_block" => {
                only_keys(fields, "block effect", &["type", "blocks", "min_amount", "temperature"])?;
                Self::MeltBlock {
                    blocks: match fields.get("blocks") { None | Some(Value::Null) => BlockPredicate::Any, Some(value) => BlockPredicate::decode(value)? },
                    min_amount: int_field(fields, "min_amount", 1, None)?,
                    temperature: int_field(fields, "temperature", 1, None)?,
                }
            }
            _ => return Ok(Self::General(GeneralEffect::decode(fields, kind)?)),
        })
    }

    /// Runs the effect: the part of the level it used (FluidEffect.apply).
    pub(crate) fn apply(&self, fluid: &FluidStack, level: EffectLevel, context: &mut BlockContext, action: FluidAction) -> f32 {
        match self {
            Self::Conditional { block, effect } => {
                let Some(state) = context.env.world.block_state(context.pos) else { return 0.0 };
                if block.matches(context.env.world, context.pos, &state) { effect.apply(fluid, level, context, action) } else { 0.0 }
            }
            Self::Scaling { effects } => {
                for (entry_level, effect) in effects.iter().rev() {
                    if level.value >= *entry_level {
                        if *entry_level == 0.0 { return 0.0; }
                        return effect.apply(fluid, level.divide(*entry_level), context, action) * entry_level;
                    }
                }
                0.0
            }
            Self::Alternatives { effects } => {
                for effect in effects {
                    let applied = effect.apply(fluid, level, context, action);
                    if applied > 0.0 { return applied; }
                }
                0.0
            }
            Self::Sequence { effects } => {
                let mut max_applied = 0.0;
                for effect in effects {
                    let applied = effect.apply(fluid, level, context, action);
                    if applied <= 0.0 { break; }
                    if applied > max_applied { max_applied = applied; }
                }
                max_applied
            }
            Self::Offset { effect, direction } => {
                let chosen = direction.unwrap_or(context.direction);
                let pos = chosen.relative(context.pos);
                effect.apply(fluid, level, &mut context.at(pos, chosen), action)
            }
            Self::Weather => {
                if level.is_full() && context.env.world.is_weathering_copper(context.pos) {
                    if action.execute() { context.env.world.apply_weathering(context.pos); }
                    1.0
                } else { 0.0 }
            }
            Self::RemoveBlock => {
                if !level.is_full() { return 0.0; }
                let Some(original) = context.env.world.block_state(context.pos) else { return 0.0 };
                let Some(replacement) = context.env.world.fluid_block(&fluid.fluid) else { return 0.0 };
                if original != replacement {
                    if action.execute() {
                        if context.env.world.set_block(context.pos, &replacement) { context.env.world.destroy_particles(context.pos, &original); }
                    }
                    1.0
                } else { 0.0 }
            }
            Self::PlaceBlock { block, sound } => place_block(block, sound, level, context, action),
            Self::BreakBlock { hardness, enchantments } => {
                let Some(state) = context.env.world.block_state(context.pos) else { return 0.0 };
                if state.block.to_string() == "minecraft:air" { return 0.0; }
                let speed = context.env.world.destroy_speed(context.pos);
                if speed < 0.0 || context.env.holder.break_restricted() { return 0.0; }
                let requirement = if *hardness == 0.0 { 1.0 } else { speed / hardness };
                if requirement > level.value { return 0.0; }
                if action.execute() {
                    let spec = BreakSpec {
                        pos: context.pos,
                        location: context.location,
                        enchantments: enchantments.clone(),
                        by_player: context.env.holder.player,
                        replacement: context.env.world.fluid_block(&fluid.fluid),
                    };
                    context.env.world.break_block(&spec);
                }
                requirement
            }
            Self::MobEffectCloud { effects } => {
                if !context.offset_replaceable() { return 0.0; }
                let scale = level.value;
                if action.execute() {
                    let mut included = Vec::new();
                    for def in effects {
                        let time = (def.time as f32 * scale) as i32;
                        if time > 10 { included.push(def.instance(time)); }
                    }
                    if included.is_empty() { return 0.0; }
                    context.env.world.spawn_cloud(&CloudSpec { at: context.location, owner: context.env.holder.effect_source(), effects: included });
                }
                scale
            }
            Self::PotionCloud { scale, nbt } => {
                if !nbt.test(fluid.tag.as_ref()) || !context.offset_replaceable() { return 0.0; }
                if action.execute() {
                    let mut included = Vec::new();
                    for instance in context.env.world.potion_effects(fluid.tag.as_ref()) {
                        if instance.instant {
                            let amplifier = ((instance.amplifier + 1) as f32 * scale * level.value * 2.0) as i32 - 1;
                            if amplifier >= 0 {
                                included.push(NewEffect { effect: instance.effect.clone(), duration: instance.duration, amplifier, curative_items: None });
                            }
                        } else {
                            let duration = (instance.duration as f32 * scale * level.value) as i32;
                            if duration > 10 {
                                included.push(NewEffect { effect: instance.effect.clone(), duration, amplifier: instance.amplifier, curative_items: None });
                            }
                        }
                    }
                    if included.is_empty() { return 0.0; }
                    context.env.world.spawn_cloud(&CloudSpec { at: context.location, owner: context.env.holder.effect_source(), effects: included });
                }
                level.value
            }
            Self::MoveBlock { push, sound } => move_block(*push, sound, level, context, action),
            Self::Interact => {
                if !context.env.world.within_world_border(context.pos) { return 0.0; }
                if action.simulate() { return 1.0; }
                let holder = context.env.holder.clone();
                let mut spec = InteractSpec {
                    pos: context.pos,
                    direction: context.direction,
                    location: context.location,
                    holder: &holder,
                    main_hand: &mut context.env.holder.main_hand,
                    off_hand: &mut context.env.holder.off_hand,
                    action,
                };
                let result = context.env.world.interact_block(&mut spec);
                if result.consumes_action() { 1.0 } else { 0.0 }
            }
            Self::MeltBlock { blocks, min_amount, temperature } => {
                let Some(state) = context.env.world.block_state(context.pos) else { return 0.0 };
                if state.block.to_string() == "minecraft:air" || !blocks.matches(context.env.world, context.pos, &state) || !level.is_full() {
                    return 0.0;
                }
                if context.env.world.destroy_speed(context.pos) < 0.0 || context.env.holder.break_restricted() { return 0.0; }
                let Some(item) = context.env.world.block_item(&state.block) else { return 0.0 };
                let Some(result) = context.env.world.melting_result(&item, *temperature) else { return 0.0 };
                if action.execute() {
                    let placed = if result.amount as i32 >= *min_amount {
                        context.env.world.fluid_block(&result.fluid).is_some_and(|placed| context.env.world.set_block(context.pos, &placed))
                    } else {
                        let flow = result.amount as i32 * 8 / min_amount;
                        flow > 0 && context.env.world.flowing_fluid_block(&result.fluid, flow).is_some_and(|placed| context.env.world.set_block(context.pos, &placed))
                    };
                    if placed { context.env.world.destroy_particles(context.pos, &state); }
                }
                1.0
            }
            Self::General(general) => general.apply(fluid, level, &mut context.env, context.pos, context.location, action),
        }
    }
}

/// PlaceBlockFluidEffect.apply: the named block places by its item form when it has one and by hand when
/// it does not; with no block named, a hand whose item provides a block places it (off hand first), and the
/// context's stack is the last resort. Each candidate is checked against the adventure place restriction.
fn place_block(block: &Option<BlockState>, sound: &Option<String>, level: EffectLevel, context: &mut BlockContext, action: FluidAction) -> f32 {
    if !level.is_full() { return 0.0; }
    // (mode, which stack: 0 main hand, 1 off hand, 2 the context stack, the block state to place)
    let candidate: Option<(PlaceMode, usize, BlockState)> = match block {
        Some(state) => {
            let mode = if context.env.world.block_item(&state.block).is_some() { PlaceMode::BlockItemForm } else { PlaceMode::Manual };
            Some((mode, 2, state.clone()))
        }
        None => {
            let mut found = None;
            for (index, hand) in [&context.env.holder.off_hand, &context.env.holder.main_hand].iter().enumerate() {
                if hand.is_empty() { continue; }
                if let Some(state) = context.env.world.item_provided_block(hand) {
                    found = Some((PlaceMode::ProviderHand, 1 - index, state));
                    break;
                }
            }
            match found {
                Some(found) => Some(found),
                None => {
                    let stack = &context.env.holder.stack;
                    if stack.is_empty() { None } else { context.env.world.item_provided_block(stack).map(|state| (PlaceMode::ContextStack, 2, state)) }
                }
            }
        }
    };
    let Some((mode, which, state)) = candidate else { return 0.0 };
    let holder = context.env.holder.clone();
    let probe = match which {
        0 => context.env.holder.main_hand.clone(),
        1 => context.env.holder.off_hand.clone(),
        _ => context.env.holder.stack.clone(),
    };
    if context.env.place_restricted(&probe, context.pos) { return 0.0; }
    let stack = match which {
        0 => &mut context.env.holder.main_hand,
        1 => &mut context.env.holder.off_hand,
        _ => &mut context.env.holder.stack,
    };
    let mut spec = PlaceSpec {
        mode,
        pos: context.pos,
        direction: context.direction,
        location: context.location,
        block: Some(state),
        stack,
        holder: &holder,
        action,
        sound: sound.clone(),
    };
    let result = context.env.world.place_block(&mut spec);
    match mode {
        PlaceMode::Manual => if result == InteractionResult::Success { 1.0 } else { 0.0 },
        _ => if result.consumes_action() { 1.0 } else { 0.0 },
    }
}

/// MoveBlocksFluidEffect.apply: a first block that breaks on push just breaks; otherwise a row of up to
/// twelve blocks shifts one cell, destroying a block marked for destroying at the far end.
fn move_block(push: bool, sound: &str, level: EffectLevel, context: &mut BlockContext, action: FluidAction) -> f32 {
    if !level.is_full() || context.env.holder.break_restricted() { return 0.0; }
    let pos = context.pos;
    let Some(state) = context.env.world.block_state(pos) else { return 0.0 };
    if state.block.to_string() == "minecraft:air" { return 0.0; }
    if context.env.world.push_reaction(pos) == PushReaction::Destroy {
        if context.env.world.is_liquid_block(pos) { return 0.0; }
        if action.execute() { context.env.world.destroy_block(pos); }
        return 1.0;
    }
    let facing = context.direction.opposite();
    let direction = if push { facing } else { facing.opposite() };
    if context.env.world.has_block_entity(pos) || !context.env.world.is_pushable(pos, direction, false, facing) {
        return 0.0;
    }
    let mut moving = 0;
    loop {
        moving += 1;
        if moving > 12 { return 0.0; }
        let mut target = pos;
        for _ in 0..moving { target = direction.relative(target); }
        let Some(state) = context.env.world.block_state(target) else { return 0.0 };
        if !context.env.world.is_pushable(target, direction, true, facing) { return 0.0; }
        if state.block.to_string() == "minecraft:air" || context.env.world.push_reaction(target) == PushReaction::Destroy { break; }
    }
    if action.execute() {
        context.env.world.move_row(pos, facing, direction, moving, push);
        let pitch = context.env.world.random_float() * if push { 0.25 } else { 0.15 } + 0.6;
        context.env.world.play_sound([pos[0] as f64 + 0.5, pos[1] as f64 + 0.5, pos[2] as f64 + 0.5], sound, 0.5, pitch);
    }
    1.0
}

/* One data file, the store, and the manager */

/// One fluid_effects file: the fluid it matches and the effects it runs (FluidEffects).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FluidEffects {
    ingredient: FluidIngredient,
    block_effects: Vec<BlockEffect>,
    entity_effects: Vec<EntityEffect>,
    /// Data pack UIs hide the entry (the `hidden` field); the engine itself ignores it.
    pub(crate) hidden: bool,
}

impl FluidEffects {
    pub(crate) fn decode(value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("fluid effects must be an object")?;
        only_keys(fields, "fluid effects", &["fluid", "block_effects", "entity_effects", "hidden", "conditions"])?;
        let mut block_effects = Vec::new();
        if let Some(list) = fields.get("block_effects") {
            for entry in list.as_array().ok_or("block_effects must be an array")? {
                block_effects.push(BlockEffect::decode(entry)?);
            }
        }
        let mut entity_effects = Vec::new();
        if let Some(list) = fields.get("entity_effects") {
            for entry in list.as_array().ok_or("entity_effects must be an array")? {
                entity_effects.push(EntityEffect::decode(entry)?);
            }
        }
        Ok(Self {
            ingredient: FluidIngredient::decode(fields.get("fluid").ok_or("fluid effects need a fluid")?)?,
            block_effects,
            entity_effects,
            hidden: bool_field(fields, "hidden", false)?,
        })
    }

    /// The entry's fluid ingredient (FluidEffects.ingredient).
    pub(crate) fn ingredient(&self) -> &FluidIngredient { &self.ingredient }
    /// The entry's block effects, in file order (FluidEffects.blockEffects).
    pub(crate) fn block_effects(&self) -> &[BlockEffect] { &self.block_effects }
    /// The entry's entity effects, in file order (FluidEffects.entityEffects).
    pub(crate) fn entity_effects(&self) -> &[EntityEffect] { &self.entity_effects }
    /// Whether the entry has any effect at all (FluidEffects.hasEffects).
    pub(crate) fn has_effects(&self) -> bool { !self.block_effects.is_empty() || !self.entity_effects.is_empty() }
    /// Whether the entry has block effects (FluidEffects.hasBlockEffects).
    pub(crate) fn has_block_effects(&self) -> bool { !self.block_effects.is_empty() }
    /// Whether the entry has entity effects (FluidEffects.hasEntityEffects).
    pub(crate) fn has_entity_effects(&self) -> bool { !self.entity_effects.is_empty() }
    /// The millibuckets one level of this entry costs for `fluid` (FluidEffects.getAmount).
    pub(crate) fn amount_per_level(&self, fluid: &FluidStack, in_tag: InTag) -> i32 { self.ingredient.amount(&fluid.fluid, in_tag) }

    /// FluidEffects.apply against a block: simulates every effect, then executes them, and returns the
    /// millibuckets consumed — the amount that covers the largest part of a level any effect used.
    /// The level is the original's float (a cannon's power is 1.5 for the end cannon), not a whole number.
    pub(crate) fn apply_to_block(&self, fluid: &FluidStack, level_cap: f32, context: &mut BlockContext, in_tag: InTag) -> i32 {
        let amount_per_level = self.ingredient.amount(&fluid.fluid, in_tag);
        if amount_per_level <= 0 { return 0; }
        let amount = fluid.amount as i32;
        let scale = if amount as f32 >= amount_per_level as f32 * level_cap {
            level_cap
        } else {
            amount as f32 / amount_per_level as f32
        };
        let level = EffectLevel { value: scale, max: level_cap };
        let mut used_scale = 0.0f32;
        for phase in [FluidAction::Simulate, FluidAction::Execute] {
            for effect in &self.block_effects {
                let used = effect.apply(fluid, level, context, phase);
                if used > used_scale { used_scale = used; }
            }
        }
        (amount_per_level as f32 * used_scale.min(scale)).ceil() as i32
    }

    /// FluidEffects.apply against an entity; same contract as apply_to_block.
    pub(crate) fn apply_to_entity(&self, fluid: &FluidStack, level_cap: f32, context: &mut EntityContext, in_tag: InTag) -> i32 {
        let amount_per_level = self.ingredient.amount(&fluid.fluid, in_tag);
        if amount_per_level <= 0 { return 0; }
        let amount = fluid.amount as i32;
        let scale = if amount as f32 >= amount_per_level as f32 * level_cap {
            level_cap
        } else {
            amount as f32 / amount_per_level as f32
        };
        let level = EffectLevel { value: scale, max: level_cap };
        let mut used_scale = 0.0f32;
        for phase in [FluidAction::Simulate, FluidAction::Execute] {
            for effect in &self.entity_effects {
                let used = effect.apply(fluid, level, context, phase);
                if used > used_scale { used_scale = used; }
            }
        }
        (amount_per_level as f32 * used_scale.min(scale)).ceil() as i32
    }

    /// What apply_to_entity would consume, running the simulate phase only: the original's
    /// FluidAction.SIMULATE call, which interaction gates (a slurp that may start, a drink that may
    /// finish) ask before anything is allowed to happen. Nothing is applied.
    pub(crate) fn simulate_to_entity(&self, fluid: &FluidStack, level_cap: f32, context: &mut EntityContext, in_tag: InTag) -> i32 {
        let amount_per_level = self.ingredient.amount(&fluid.fluid, in_tag);
        if amount_per_level <= 0 { return 0; }
        let amount = fluid.amount as i32;
        let scale = if amount as f32 >= amount_per_level as f32 * level_cap {
            level_cap
        } else {
            amount as f32 / amount_per_level as f32
        };
        let level = EffectLevel { value: scale, max: level_cap };
        let mut used_scale = 0.0f32;
        for effect in &self.entity_effects {
            let used = effect.apply(fluid, level, context, FluidAction::Simulate);
            if used > used_scale { used_scale = used; }
        }
        (amount_per_level as f32 * used_scale.min(scale)).ceil() as i32
    }
}

/// The loaded files in load order, plus a problem for every file that failed to decode
/// (FluidEffectManager keeps the rest of the pack usable and logs the bad file).
#[derive(Debug, Default)]
pub(crate) struct Effects {
    entries: Vec<(Identifier, FluidEffects)>,
    pub(crate) problems: Vec<String>,
}

impl Effects {
    /// Loads a folder of files, in file ID order so the first match is deterministic: a file whose load
    /// conditions fail is skipped whole (the manager's condition gate), and a file that fails to decode is
    /// recorded and skipped.
    pub(crate) fn load<C: Conditions>(files: &[(Identifier, Value)], conditions: &C) -> Self {
        let mut effects = Self::default();
        let mut sorted: Vec<&(Identifier, Value)> = files.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (id, value) in sorted {
            match fluid_recipes::effective(value, conditions) {
                Ok(Some(fields)) => match FluidEffects::decode(&Value::Object(fields.clone())) {
                    Ok(decoded) => effects.entries.push((id.clone(), decoded)),
                    Err(error) => effects.problems.push(format!("{id}: {error}")),
                },
                Ok(None) => {}
                Err(error) => effects.problems.push(format!("{id}: {error}")),
            }
        }
        effects
    }

    /// The first entry whose ingredient matches the fluid (FluidEffectManager.find).
    pub(crate) fn find(&self, fluid: &FluidStack, in_tag: InTag) -> Option<&FluidEffects> {
        self.entries.iter().map(|(_, effects)| effects).find(|effects| effects.ingredient.test(&fluid.fluid, fluid.amount as i32, in_tag))
    }

    /// How many files loaded.
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    /// The ids of the loaded files, in load order.
    pub(crate) fn ids(&self) -> impl Iterator<Item = &Identifier> {
        self.entries.iter().map(|(id, _)| id)
    }

    /// Every loaded entry with its effects, in load order, for consumers that list them all
    /// (the encyclopedia's fluid effect pages).
    pub(crate) fn entries(&self) -> impl Iterator<Item = (&Identifier, &FluidEffects)> {
        self.entries.iter().map(|(id, effects)| (id, effects))
    }
}

/// The domain's loaded data as common.rs keeps it: the fluid effects of the current reload. The fluid tags
/// the ingredients match against live in the smeltery's data, as with the other fluid consumers, and are
/// handed in at each call.
#[derive(Debug, Default)]
pub(crate) struct Data {
    pub(crate) effects: Effects,
    pub(crate) loaded: bool,
}

impl Data {
    /// What a shooter asks the matching entry before running anything (FluidEffectManager.find, then
    /// FluidEffects.hasEffects / hasBlockEffects / hasEntityEffects / getAmount): whether it has block
    /// effects, whether it has entity effects, and its millibuckets per level. None when no entry
    /// matches the fluid, which the original answers with an entry without effects.
    pub(crate) fn describe(&self, fluid: &FluidStack, tags: &FluidTags) -> Option<(bool, bool, i32)> {
        let in_tag = |fluid: &Identifier, tag: &Identifier| tags.contains(fluid, tag);
        self.effects.find(fluid, &in_tag).map(|effects| (effects.has_block_effects(), effects.has_entity_effects(), effects.amount_per_level(fluid, &in_tag)))
    }

    /// Finds and runs the matching entry's block effects, returning the millibuckets consumed.
    pub(crate) fn apply_to_block(&self, fluid: &FluidStack, level_cap: f32, context: &mut BlockContext, tags: &FluidTags) -> i32 {
        let in_tag = |fluid: &Identifier, tag: &Identifier| tags.contains(fluid, tag);
        match self.effects.find(fluid, &in_tag) {
            Some(effects) => effects.apply_to_block(fluid, level_cap, context, &in_tag),
            None => 0,
        }
    }

    /// Finds and runs the matching entry's entity effects, returning the millibuckets consumed.
    pub(crate) fn apply_to_entity(&self, fluid: &FluidStack, level_cap: f32, context: &mut EntityContext, tags: &FluidTags) -> i32 {
        let in_tag = |fluid: &Identifier, tag: &Identifier| tags.contains(fluid, tag);
        match self.effects.find(fluid, &in_tag) {
            Some(effects) => effects.apply_to_entity(fluid, level_cap, context, &in_tag),
            None => 0,
        }
    }

    /// What apply_to_entity would consume for the matching entry, simulate phase only.
    pub(crate) fn simulate_to_entity(&self, fluid: &FluidStack, level_cap: f32, context: &mut EntityContext, tags: &FluidTags) -> i32 {
        let in_tag = |fluid: &Identifier, tag: &Identifier| tags.contains(fluid, tag);
        match self.effects.find(fluid, &in_tag) {
            Some(effects) => effects.simulate_to_entity(fluid, level_cap, context, &in_tag),
            None => 0,
        }
    }
}

/// Registers the reload listener (id `tconstruct:fluid_effects`) and returns the shared state, mirroring
/// smeltery_data::register: a reload replaces the effects, a server stop clears them.
pub(crate) fn register<C: Conditions + 'static>(context: impl Fn() -> Result<C, String> + Send + Sync + 'static) -> Result<Arc<RwLock<Data>>, String> {
    let state = Arc::new(RwLock::new(Data::default()));
    let effects_out = state.clone();
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "fluid_effects").unwrap(),
        FOLDER,
        move |files| {
            let mut state = effects_out.write().map_err(|_| "fluid effects data poisoned")?;
            match context() {
                Ok(context) => {
                    state.effects = Effects::load(&files, &context);
                    state.loaded = true;
                    for problem in &state.effects.problems {
                        eprintln!("Tinkers: {problem}");
                    }
                    Ok(())
                }
                Err(error) => {
                    state.effects = Effects { problems: vec![error.clone()], ..Effects::default() };
                    state.loaded = false;
                    Err(error)
                }
            }
        },
    )?;
    let stopped = state.clone();
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| *stopped.write().expect("fluid effects data poisoned") = Data::default());
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    fn id(text: &str) -> Identifier {
        Identifier::parse(text).expect("test id")
    }

    fn stack(fluid: &str, amount: u32) -> FluidStack {
        FluidStack { fluid: id(fluid), amount, tag: None }
    }

    fn no_tag(_: &Identifier, _: &Identifier) -> bool {
        false
    }

    #[derive(Default)]
    struct TestWorld {
        blocks: HashMap<BlockPos, BlockState>,
        speeds: HashMap<BlockPos, f32>,
        replaceable: Vec<BlockPos>,
        clouds: Vec<CloudSpec>,
        drops: Vec<Stack>,
        broken: Vec<BlockPos>,
        placed: Vec<(BlockPos, BlockState)>,
        targets: Vec<TestTarget>,
        potions: Vec<PotionEffect>,
        /// What each stack provides for placing (item to block), for the provider half of place_block.
        provided: HashMap<Identifier, Identifier>,
    }

    impl WorldAccess for TestWorld {
        fn block_state(&self, pos: BlockPos) -> Option<BlockState> {
            self.blocks.get(&pos).cloned()
        }
        fn set_block(&mut self, pos: BlockPos, state: &BlockState) -> bool {
            self.placed.push((pos, state.clone()));
            self.blocks.insert(pos, state.clone());
            true
        }
        fn destroy_particles(&mut self, _pos: BlockPos, _state: &BlockState) {}
        fn play_sound(&mut self, _at: [f64; 3], _sound: &str, _volume: f32, _pitch: f32) {}
        fn drop_item(&mut self, _at: [f64; 3], stack: &Stack) {
            self.drops.push(stack.clone());
        }
        fn explode(&mut self, _spec: &ExplosionSpec) {}
        fn spawn_cloud(&mut self, spec: &CloudSpec) {
            self.clouds.push(CloudSpec { at: spec.at, owner: spec.owner, effects: spec.effects.clone() });
        }
        fn break_block(&mut self, spec: &BreakSpec) -> bool {
            self.broken.push(spec.pos);
            self.blocks.remove(&spec.pos);
            true
        }
        fn destroy_block(&mut self, pos: BlockPos) {
            self.blocks.remove(&pos);
        }
        fn move_row(&mut self, _pos: BlockPos, _facing: Direction, _direction: Direction, _moving: i32, _push: bool) {}
        fn push_reaction(&self, _pos: BlockPos) -> PushReaction {
            PushReaction::Normal
        }
        fn has_block_entity(&self, _pos: BlockPos) -> bool {
            false
        }
        fn is_pushable(&self, _pos: BlockPos, _direction: Direction, _allow_destroy: bool, _facing: Direction) -> bool {
            true
        }
        fn destroy_speed(&self, pos: BlockPos) -> f32 {
            self.speeds.get(&pos).copied().unwrap_or(1.5)
        }
        fn is_liquid_block(&self, _pos: BlockPos) -> bool {
            false
        }
        fn can_be_replaced(&self, pos: BlockPos) -> bool {
            self.replaceable.contains(&pos)
        }
        fn within_world_border(&self, _pos: BlockPos) -> bool {
            true
        }
        fn place_block(&mut self, spec: &mut PlaceSpec) -> InteractionResult {
            if let Some(block) = &spec.block {
                self.placed.push((spec.pos, block.clone()));
            }
            InteractionResult::Success
        }
        fn interact_block(&mut self, _spec: &mut InteractSpec) -> InteractionResult {
            InteractionResult::Pass
        }
        fn interact_entity(&mut self, _spec: &mut EntityInteractSpec) -> InteractionResult {
            InteractionResult::Pass
        }
        fn is_weathering_copper(&self, _pos: BlockPos) -> bool {
            false
        }
        fn apply_weathering(&mut self, _pos: BlockPos) {}
        fn melting_result(&self, _item: &Identifier, _temperature: i32) -> Option<FluidStack> {
            None
        }
        fn fluid_block(&self, fluid: &Identifier) -> Option<BlockState> {
            BlockState::parse(&format!("{}_fluid", fluid.path())).ok()
        }
        fn flowing_fluid_block(&self, _fluid: &Identifier, _level: i32) -> Option<BlockState> {
            None
        }
        fn block_item(&self, block: &Identifier) -> Option<Identifier> {
            Some(block.clone())
        }
        fn item_provided_block(&self, stack: &Stack) -> Option<BlockState> {
            self.provided.get(&stack.item).map(|block| BlockState::new(block.clone()))
        }
        fn correct_tier_for_drops(&self, _tier: &str, _pos: BlockPos) -> bool {
            false
        }
        fn named_block_predicate(&self, _name: &str, _pos: BlockPos, _state: &BlockState) -> Option<bool> {
            None
        }
        fn adventure_place_allowed(&self, _stack: &Stack, _pos: BlockPos) -> bool {
            false
        }
        fn preferred_item(&self, output: &ItemOutput) -> Option<Stack> {
            match &output.source {
                Source::Id(item) => Some(Stack::of(item.clone(), output.count)),
                Source::Tag(_) => None,
            }
        }
        fn potion_effects(&self, _tag: Option<&NbtCompound>) -> Vec<PotionEffect> {
            self.potions.clone()
        }
        fn living_in_block(&mut self, _pos: BlockPos) -> Vec<&mut dyn EntityTarget> {
            self.targets.iter_mut().map(|target| target as &mut dyn EntityTarget).collect()
        }
        fn random_float(&mut self) -> f32 {
            0.0
        }
    }

    #[derive(Clone, Default)]
    struct TestTarget {
        kind: String,
        living: bool,
        player: bool,
        effects: HashMap<Identifier, ActiveEffect>,
        added: Vec<NewEffect>,
        fire_ticks: i32,
        frozen: i32,
        air: i32,
        hurts: Vec<(String, f32)>,
        eaten: Vec<(i32, f32)>,
        stats: Vec<(Identifier, i32)>,
        pushes: Vec<[f64; 3]>,
    }

    impl TestTarget {
        fn living() -> Self {
            Self { kind: "minecraft:zombie".to_owned(), living: true, air: 300, ..Self::default() }
        }
    }

    impl EntityTarget for TestTarget {
        fn id(&self) -> i32 {
            1
        }
        fn kind(&self) -> &str {
            &self.kind
        }
        fn is_living(&self) -> bool {
            self.living
        }
        fn is_player(&self) -> bool {
            self.player
        }
        fn fire_immune(&self) -> bool {
            false
        }
        fn named_predicate(&self, _name: &str) -> Option<bool> {
            None
        }
        fn in_type_tag(&self, _tag: &Identifier) -> bool {
            false
        }
        fn active_effect(&self, effect: &Identifier) -> Option<ActiveEffect> {
            self.effects.get(effect).cloned()
        }
        fn can_be_affected(&self, _effect: &NewEffect) -> bool {
            true
        }
        fn any_curative_effect(&self, _item: &Stack) -> bool {
            false
        }
        fn remaining_fire_ticks(&self) -> i32 {
            self.fire_ticks
        }
        fn ticks_frozen(&self) -> i32 {
            self.frozen
        }
        fn ticks_required_to_freeze(&self) -> i32 {
            140
        }
        fn can_freeze(&self) -> bool {
            true
        }
        fn air_supply(&self) -> i32 {
            self.air
        }
        fn max_air_supply(&self) -> i32 {
            300
        }
        fn can_eat(&self, _always_eat: bool) -> bool {
            true
        }
        fn position(&self) -> [f64; 3] {
            [0.5, 0.0, 0.5]
        }
        fn look_angle(&self) -> [f64; 3] {
            [0.0, 0.0, 1.0]
        }
        fn block_position(&self) -> BlockPos {
            [0, 0, 0]
        }
        fn strong_bones_helmet(&self) -> Option<(i32, Identifier)> {
            None
        }
        fn calcifiable_level(&self) -> i32 {
            0
        }
        fn set_remaining_fire_ticks(&mut self, ticks: i32) {
            self.fire_ticks = ticks;
        }
        fn set_ticks_frozen(&mut self, ticks: i32) {
            self.frozen = ticks;
        }
        fn add_effect(&mut self, effect: NewEffect) -> bool {
            self.added.push(effect.clone());
            self.effects.insert(effect.effect.clone(), ActiveEffect { amplifier: effect.amplifier, duration: effect.duration });
            true
        }
        fn remove_effect(&mut self, effect: &Identifier) -> bool {
            self.effects.remove(effect).is_some()
        }
        fn cure_effects(&mut self, _item: &Stack) -> bool {
            false
        }
        fn hurt(&mut self, amount: f32, damage_type: &str) -> bool {
            self.hurts.push((damage_type.to_owned(), amount));
            true
        }
        fn eat(&mut self, hunger: i32, saturation: f32) {
            self.eaten.push((hunger, saturation));
        }
        fn food_eaten(&mut self, _representative: &Stack, _hunger: i32, _saturation: f32) {}
        fn award_stat(&mut self, stat: &Identifier, amount: i32) {
            self.stats.push((stat.clone(), amount));
        }
        fn set_air_supply(&mut self, air: i32) {
            self.air = air;
        }
        fn push(&mut self, delta: [f64; 3]) {
            self.pushes.push(delta);
        }
        fn random_teleport(&mut self, _diameter: i32, _chances: i32) {}
    }

    fn entity_context<'a>(world: &'a mut TestWorld, target: &'a mut TestTarget) -> EntityContext<'a> {
        EntityContext { env: Env { world, holder: Holder::default() }, target, location: [0.5, 0.5, 0.5] }
    }

    fn block_context<'a>(world: &'a mut TestWorld, pos: BlockPos) -> BlockContext<'a> {
        BlockContext { env: Env { world, holder: Holder::default() }, pos, direction: Direction::Up, location: [0.5, 1.5, 0.5] }
    }

    #[test]
    fn manager_charges_for_the_used_scale() {
        let effects = FluidEffects::decode(&json!({
            "fluid": {"fluid": "tconstruct:blazing_blood", "amount": 250},
            "entity_effects": [{"type": "tconstruct:fire", "action": "add", "time": 2}],
        })).expect("decode");
        let mut world = TestWorld::default();
        let mut target = TestTarget::living();
        let consumed = effects.apply_to_entity(&stack("tconstruct:blazing_blood", 1000), 5.0, &mut entity_context(&mut world, &mut target), &no_tag);
        // Scale 4 of 5: 8 more seconds of fire on an unburning target, the whole stack is used.
        assert_eq!(target.fire_ticks, 160);
        assert_eq!(consumed, 1000);
        let consumed = effects.apply_to_entity(&stack("tconstruct:blazing_blood", 100), 5.0, &mut entity_context(&mut world, &mut target), &no_tag);
        // Scale 0.4: the add rounds in the remaining ticks (170 / 20 = 8), so 2 * 0.4 rounds to 1 second
        // over the current 8, and only the used part is charged.
        assert_eq!(target.fire_ticks, 180);
        assert_eq!(consumed, 100);
    }

    #[test]
    fn simulate_charges_the_same_and_changes_nothing() {
        let effects = FluidEffects::decode(&json!({
            "fluid": {"fluid": "tconstruct:blazing_blood", "amount": 250},
            "entity_effects": [{"type": "tconstruct:fire", "action": "add", "time": 2}],
        })).expect("decode");
        let mut world = TestWorld::default();
        let mut target = TestTarget::living();
        let would_consume = effects.simulate_to_entity(&stack("tconstruct:blazing_blood", 1000), 5.0, &mut entity_context(&mut world, &mut target), &no_tag);
        assert_eq!(would_consume, 1000, "the simulate run prices the same application");
        assert_eq!(target.fire_ticks, 0, "and sets nothing on fire");
        let consumed = effects.apply_to_entity(&stack("tconstruct:blazing_blood", 1000), 5.0, &mut entity_context(&mut world, &mut target), &no_tag);
        assert_eq!(consumed, would_consume);
        assert_eq!(target.fire_ticks, 160);
    }

    #[test]
    fn mob_effect_add_extends_and_set_respects_a_stronger_existing() {
        let def = MobEffectDef::decode(&serde_json::from_value::<Map<String, Value>>(json!({"effect": "minecraft:poison", "time": 100, "level": 2})).expect("object")).expect("decode");
        let level = EffectLevel { value: 0.5, max: 1.0 };
        let mut target = TestTarget::living();
        let used = apply_mob_effect(&def, &mut target, level, TimeAction::Add, FluidAction::Execute);
        assert_eq!(used, 0.5);
        assert_eq!(target.effects[&id("minecraft:poison")], ActiveEffect { amplifier: 1, duration: 50 });
        let used = apply_mob_effect(&def, &mut target, level, TimeAction::Add, FluidAction::Execute);
        assert_eq!(used, 0.5);
        assert_eq!(target.effects[&id("minecraft:poison")].duration, 100);
        target.effects.insert(id("minecraft:poison"), ActiveEffect { amplifier: 2, duration: 30 });
        let used = apply_mob_effect(&def, &mut target, level, TimeAction::Set, FluidAction::Execute);
        assert_eq!(used, 0.0);
        assert_eq!(target.effects[&id("minecraft:poison")].duration, 30);
    }

    #[test]
    fn mob_effect_infinite_needs_a_full_level() {
        let def = MobEffectDef::decode(&serde_json::from_value::<Map<String, Value>>(json!({"effect": "tconstruct:calcified"})).expect("object")).expect("decode");
        let mut target = TestTarget::living();
        let used = apply_mob_effect(&def, &mut target, EffectLevel { value: 0.5, max: 1.0 }, TimeAction::Add, FluidAction::Execute);
        assert_eq!(used, 0.0);
        assert!(target.added.is_empty());
        let used = apply_mob_effect(&def, &mut target, EffectLevel { value: 1.0, max: 1.0 }, TimeAction::Add, FluidAction::Execute);
        assert_eq!(used, 1.0);
        assert_eq!(target.added[0].duration, -1);
    }

    #[test]
    fn damage_simulates_first_and_the_pair_picks_the_source() {
        let effect = EntityEffect::decode(&json!({
            "type": "tconstruct:damage", "damage": 3.0,
            "damage_type": {"melee": "tconstruct:melee", "ranged": "tconstruct:ranged"},
        })).expect("decode");
        let mut world = TestWorld::default();
        let mut target = TestTarget::living();
        target.kind = "minecraft:enderman".to_owned();
        target.effects.insert(id("tconstruct:enderference"), ActiveEffect { amplifier: 0, duration: 100 });
        let level = EffectLevel { value: 2.0, max: 2.0 };
        let fluid = stack("tconstruct:molten_ender", 100);
        {
            let mut context = entity_context(&mut world, &mut target);
            context.env.holder.projectile = Some(ProjectileFacts { id: 7, delta: [0.0, 0.0, 1.0] });
            assert_eq!(effect.apply(&fluid, level, &mut context, FluidAction::Simulate), 2.0);
        }
        assert!(target.hurts.is_empty());
        {
            let mut context = entity_context(&mut world, &mut target);
            context.env.holder.projectile = Some(ProjectileFacts { id: 7, delta: [0.0, 0.0, 1.0] });
            assert_eq!(effect.apply(&fluid, level, &mut context, FluidAction::Execute), 2.0);
        }
        // An enderman carrying the enderference effect takes the projectile's damage as melee.
        assert_eq!(target.hurts, vec![("tconstruct:melee".to_owned(), 6.0)]);
    }

    #[test]
    fn damage_pair_keeps_the_ranged_type_without_the_whole_override() {
        // TinkerEffects.needsEnderferenceOverride names the enderman and the effect together:
        // a marked zombie and an unmarked enderman both keep the pair's ranged type.
        let effect = EntityEffect::decode(&json!({
            "type": "tconstruct:damage", "damage": 3.0,
            "damage_type": {"melee": "tconstruct:melee", "ranged": "tconstruct:ranged"},
        })).expect("decode");
        let level = EffectLevel { value: 2.0, max: 2.0 };
        let fluid = stack("tconstruct:molten_ender", 100);
        for (kind, marked) in [("minecraft:zombie", true), ("minecraft:enderman", false), ("minecraft:zombie", false)] {
            let mut world = TestWorld::default();
            let mut target = TestTarget::living();
            target.kind = kind.to_owned();
            if marked {
                target.effects.insert(id("tconstruct:enderference"), ActiveEffect { amplifier: 0, duration: 100 });
            }
            let mut context = entity_context(&mut world, &mut target);
            context.env.holder.projectile = Some(ProjectileFacts { id: 7, delta: [0.0, 0.0, 1.0] });
            assert_eq!(effect.apply(&fluid, level, &mut context, FluidAction::Execute), 2.0);
            assert_eq!(target.hurts, vec![("tconstruct:ranged".to_owned(), 6.0)], "{kind} marked={marked}");
        }
    }

    #[test]
    fn scaling_divides_then_multiplies_back() {
        let effect = EntityEffect::decode(&json!({
            "type": "tconstruct:scaling",
            "effects": [{"level": 2.0, "effect": {"type": "tconstruct:fire", "action": "set", "time": 4}}],
        })).expect("decode");
        let mut world = TestWorld::default();
        let mut target = TestTarget::living();
        let used = {
            let mut context = entity_context(&mut world, &mut target);
            effect.apply(&stack("minecraft:lava", 100), EffectLevel { value: 3.0, max: 3.0 }, &mut context, FluidAction::Execute)
        };
        assert_eq!(used, 3.0);
        assert_eq!(target.fire_ticks, 120);
    }

    #[test]
    fn break_block_needs_the_requirement() {
        let effects = FluidEffects::decode(&json!({
            "fluid": {"fluid": "tconstruct:molten_obsidian", "amount": 100},
            "block_effects": [{"type": "tconstruct:break_block", "hardness": 0.75}],
        })).expect("decode");
        let mut world = TestWorld::default();
        world.blocks.insert([0, 0, 0], BlockState::parse("minecraft:stone").expect("state"));
        // Default speed 1.5 against hardness 0.75 needs 2 levels; 100 mB is only 1.
        let consumed = effects.apply_to_block(&stack("tconstruct:molten_obsidian", 100), 5.0, &mut block_context(&mut world, [0, 0, 0]), &no_tag);
        assert_eq!(consumed, 0);
        assert!(world.broken.is_empty());
        let consumed = effects.apply_to_block(&stack("tconstruct:molten_obsidian", 200), 5.0, &mut block_context(&mut world, [0, 0, 0]), &no_tag);
        assert_eq!(consumed, 200);
        assert_eq!(world.broken, vec![[0, 0, 0]]);
    }

    #[test]
    fn cloud_needs_room_and_enough_time() {
        let effects = FluidEffects::decode(&json!({
            "fluid": {"fluid": "tconstruct:liquid_soul", "amount": 50},
            "block_effects": [{"type": "tconstruct:mob_effect_cloud", "effects": [{"effect": "minecraft:blindness", "time": 100}]}],
        })).expect("decode");
        let mut world = TestWorld::default();
        let consumed = effects.apply_to_block(&stack("tconstruct:liquid_soul", 50), 1.0, &mut block_context(&mut world, [0, 1, 0]), &no_tag);
        assert_eq!(consumed, 0);
        assert!(world.clouds.is_empty());
        world.replaceable.push([0, 2, 0]);
        let consumed = effects.apply_to_block(&stack("tconstruct:liquid_soul", 50), 1.0, &mut block_context(&mut world, [0, 1, 0]), &no_tag);
        assert_eq!(consumed, 50);
        assert_eq!(world.clouds.len(), 1);
        assert_eq!(world.clouds[0].effects[0].duration, 100);
    }

    #[test]
    fn potion_effect_applies_the_potions_effects() {
        // PotionFluidEffect: the potion the fluid's tag names, scaled; an instant effect applies
        // at the full value, a lasting one for its scaled duration. The world's potion list
        // stands in for the registry read the live world does.
        let effects = FluidEffects::decode(&json!({
            "fluid": {"fluid": "tconstruct:potion", "amount": 250},
            "entity_effects": [{"type": "tconstruct:potion", "scale": 1.0}],
        })).expect("decode");
        let mut world = TestWorld::default();
        world.potions = vec![
            PotionEffect { effect: id("minecraft:poison"), duration: 200, amplifier: 0, instant: false },
            PotionEffect { effect: id("minecraft:instant_health"), duration: 1, amplifier: 0, instant: true },
        ];
        let mut target = TestTarget::living();
        let consumed = effects.apply_to_entity(&stack("tconstruct:potion", 250), 5.0, &mut entity_context(&mut world, &mut target), &no_tag);
        assert_eq!(consumed, 250, "one level spent on the potion");
        assert_eq!(target.added.len(), 2);
        assert_eq!(target.effects.get(&id("minecraft:poison")), Some(&ActiveEffect { amplifier: 0, duration: 200 }));
        assert_eq!(target.effects.get(&id("minecraft:instant_health")), Some(&ActiveEffect { amplifier: 0, duration: 1 }));
    }

    #[test]
    fn potion_cloud_spawns_a_cloud_with_the_potions_effects() {
        // PotionCloudFluidEffect: the potion the fluid's tag names, as a lingering cloud; an
        // instant effect scales its amplifier, a lasting one its duration.
        let effects = FluidEffects::decode(&json!({
            "fluid": {"fluid": "tconstruct:potion", "amount": 50},
            "block_effects": [{"type": "tconstruct:potion_cloud", "scale": 1.0}],
        })).expect("decode");
        let mut world = TestWorld::default();
        world.potions = vec![
            PotionEffect { effect: id("minecraft:poison"), duration: 200, amplifier: 0, instant: false },
            PotionEffect { effect: id("minecraft:instant_damage"), duration: 1, amplifier: 1, instant: true },
        ];
        world.replaceable.push([0, 2, 0]);
        let consumed = effects.apply_to_block(&stack("tconstruct:potion", 50), 1.0, &mut block_context(&mut world, [0, 1, 0]), &no_tag);
        assert_eq!(consumed, 50, "one level spent on the cloud");
        assert_eq!(world.clouds.len(), 1);
        let cloud = &world.clouds[0].effects;
        assert_eq!(cloud.len(), 2);
        assert_eq!(cloud[0], NewEffect { effect: id("minecraft:poison"), duration: 200, amplifier: 0, curative_items: None });
        // (1 + 1) * 1.0 * 1.0 * 2 - 1 = 3
        assert_eq!(cloud[1], NewEffect { effect: id("minecraft:instant_damage"), duration: 1, amplifier: 3, curative_items: None });
    }

    #[test]
    fn tag_predicate_matches_a_subset() {
        let predicate = TagPredicate::decode(&json!({"Bottle": "REGULAR"})).expect("decode");
        let mut tag = NbtCompound::new();
        tag.insert("Bottle".into(), Nbt::String("REGULAR".into()));
        tag.insert("CustomPotionEffects".into(), Nbt::List(vec![]));
        assert!(predicate.test(Some(&tag)));
        tag.insert("Bottle".into(), Nbt::String("TIPPED".into()));
        assert!(!predicate.test(Some(&tag)));
        assert!(!predicate.test(None));
        assert!(TagPredicate::any().test(None));
    }

    #[test]
    fn representative_files_decode() {
        // The offset effect inlines its inner effect's fields; push_entity inlines its leveling value;
        // an area effect carries its mob effect inline. All three shapes come from the shipped data.
        BlockEffect::decode(&json!({
            "type": "tconstruct:offset", "offset_type": "tconstruct:place_block",
            "block": "minecraft:glowstone",
        })).expect("offset");
        EntityEffect::decode(&json!({"type": "tconstruct:push_entity", "each_level": -1.0})).expect("push");
        EntityEffect::decode(&json!({
            "type": "tconstruct:area_mob_effect", "effect": "minecraft:slowness", "time": 60,
            "action": "add", "group_cost": "sum",
        })).expect("area");
        assert!(BlockEffect::decode(&json!({"type": "tconstruct:alternatives", "effects": []})).is_err());
    }

    fn place_block_effects() -> FluidEffects {
        FluidEffects::decode(&json!({
            "fluid": {"fluid": "tconstruct:molten_obsidian", "amount": 100},
            "block_effects": [{"type": "tconstruct:place_block"}],
        })).expect("place block")
    }

    fn place_block_context<'a>(world: &'a mut TestWorld, holder: Holder) -> BlockContext<'a> {
        BlockContext { env: Env { world, holder }, pos: [0, 0, 0], direction: Direction::Up, location: [0.5, 1.5, 0.5] }
    }

    #[test]
    fn place_block_without_a_named_block_uses_the_offhand_first() {
        let effects = place_block_effects();
        let mut world = TestWorld::default();
        world.provided.insert(id("minecraft:stone"), id("minecraft:stone"));
        world.provided.insert(id("minecraft:glowstone"), id("minecraft:glowstone"));
        let mut holder = Holder::default();
        holder.off_hand = Stack::of(id("minecraft:stone"), 64);
        holder.main_hand = Stack::of(id("minecraft:glowstone"), 64);
        let mut context = place_block_context(&mut world, holder);
        let consumed = effects.apply_to_block(&stack("tconstruct:molten_obsidian", 100), 5.0, &mut context, &no_tag);
        assert_eq!(consumed, 100);
        drop(context);
        assert_eq!(world.placed, vec![([0, 0, 0], BlockState::new(id("minecraft:stone")))]);
    }

    #[test]
    fn place_block_without_a_named_block_falls_back_to_the_mainhand() {
        let effects = place_block_effects();
        let mut world = TestWorld::default();
        world.provided.insert(id("minecraft:glowstone"), id("minecraft:glowstone"));
        let mut holder = Holder::default();
        holder.main_hand = Stack::of(id("minecraft:glowstone"), 64);
        let mut context = place_block_context(&mut world, holder);
        let consumed = effects.apply_to_block(&stack("tconstruct:molten_obsidian", 100), 5.0, &mut context, &no_tag);
        assert_eq!(consumed, 100);
        drop(context);
        assert_eq!(world.placed, vec![([0, 0, 0], BlockState::new(id("minecraft:glowstone")))]);
    }

    #[test]
    fn place_block_without_a_named_block_falls_back_to_the_context_stack() {
        let effects = place_block_effects();
        let mut world = TestWorld::default();
        world.provided.insert(id("minecraft:stone"), id("minecraft:stone"));
        let mut holder = Holder::default();
        holder.stack = Stack::of(id("minecraft:stone"), 1);
        let mut context = place_block_context(&mut world, holder);
        let consumed = effects.apply_to_block(&stack("tconstruct:molten_obsidian", 100), 5.0, &mut context, &no_tag);
        assert_eq!(consumed, 100);
        drop(context);
        assert_eq!(world.placed, vec![([0, 0, 0], BlockState::new(id("minecraft:stone")))]);
    }

    #[test]
    fn place_block_without_a_named_block_places_nothing_when_no_hand_provides() {
        let effects = place_block_effects();
        let mut world = TestWorld::default();
        let mut holder = Holder::default();
        holder.off_hand = Stack::of(id("minecraft:stick"), 1);
        let mut context = place_block_context(&mut world, holder);
        let consumed = effects.apply_to_block(&stack("tconstruct:molten_obsidian", 100), 5.0, &mut context, &no_tag);
        assert_eq!(consumed, 0);
        drop(context);
        assert!(world.placed.is_empty());
    }
}
