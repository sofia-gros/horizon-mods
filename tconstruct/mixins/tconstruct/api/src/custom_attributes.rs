//! Tinkers' own attributes (TinkerAttributes), declared with PlutoLib's attribute registry
//! (attribute_api_v1) now that a mod can register an attribute the host does not have.
//!
//! The original registers twelve attributes of its own and reads them in its event handlers.
//! Here each is declared with its original default and range, the equipment, tool and effect
//! contributions to it are collected from the stacks an entity wears and holds, and the value
//! is worked out with PlutoLib's arithmetic. A player's values are recomputed on their tick
//! and kept by entity id for the flows that read them: the mining speed multiplier in the
//! break-speed hook, the knockback multiplier on the knockback event, the crouch damage
//! multiplier in the melee hook, the bouncy value in the fall handler (`crate::bounce`), and
//! the effect duration multipliers where the hit hooks put
//! effects on a target. Chrysophilite is read in the loot flow itself, which already worked
//! the golden amount out from the worn pieces. The rest — jump boost and jump count (read by
//! the client's jump handling), use item speed (the client's item
//! use slowdown), critical damage (the critical hit event) and the experience multiplier (the
//! experience drop events) — have no surface in this port yet: their values are computed and
//! kept here, and nothing reads them until those surfaces exist. The original's percent and
//! multiplier registrations do not state their ranges in the source this port follows, so the
//! declared ranges use the widest bounds the original's own registrations use (0 to 100);
//! no shipped content comes near them.
//!
//! An effect's attribute payload cannot ride its declaration (the schema carries only a
//! category and a colour), so the payloads are held here as a table and applied while the
//! effect is active: a payload naming one of the twelve joins the value's contributions, and
//! one naming a host attribute with the addition operation goes out with the tick's base
//! additions. The two multiply-total payloads on host attributes (antigravity's gravity,
//! self-destructing's movement speed) cannot be expressed by moving a base value and are not
//! applied. SlimeKnights (MIT; see LICENSE).
use crate::{break_speed_hook::with_held_tool, max_armor, tool_stack};
use pluto_lib::{AttributeContribution, AttributeOperation, AttributeRegistry, Identifier, ServerLivingEntityEvents, ServerPlayer, StackData};
use std::{collections::BTreeMap, sync::Mutex};

pub(crate) const BOUNCY: &str = "tconstruct:generic.bouncy";
pub(crate) const CHRYSOPHILITE: &str = "tconstruct:generic.chrysophilite";
pub(crate) const CROUCH_DAMAGE_MULTIPLIER: &str = "tconstruct:generic.crouch_damage_multiplier";
pub(crate) const GOOD_EFFECT_DURATION_MULTIPLIER: &str = "tconstruct:generic.good_effect_duration_multiplier";
pub(crate) const BAD_EFFECT_DURATION_MULTIPLIER: &str = "tconstruct:generic.bad_effect_duration_multiplier";
pub(crate) const JUMP_BOOST: &str = "tconstruct:generic.jump_boost";
pub(crate) const KNOCKBACK_MULTIPLIER: &str = "tconstruct:generic.knockback_multiplier";
pub(crate) const CRITICAL_DAMAGE: &str = "tconstruct:player.critical_damage";
pub(crate) const EXPERIENCE_MULTIPLIER: &str = "tconstruct:player.experience_multiplier";
pub(crate) const JUMP_COUNT: &str = "tconstruct:player.jump_count";
pub(crate) const MINING_SPEED_MULTIPLIER: &str = "tconstruct:player.mining_speed_multiplier";
pub(crate) const USE_ITEM_SPEED: &str = "tconstruct:player.use_item_speed";

/// The twelve declarations, as (id, default, min, max) from TinkerAttributes.
const DECLARED: &[(&str, f64, f64, f64)] = &[
    (BOUNCY, 0.0, 0.0, 100.0),
    (CHRYSOPHILITE, 0.0, 0.0, 100.0),
    (CROUCH_DAMAGE_MULTIPLIER, 1.0, 0.0, 100.0),
    (GOOD_EFFECT_DURATION_MULTIPLIER, 1.0, 0.0, 100.0),
    (BAD_EFFECT_DURATION_MULTIPLIER, 1.0, 0.0, 100.0),
    (JUMP_BOOST, 0.0, 0.0, 100.0),
    (KNOCKBACK_MULTIPLIER, 1.0, 0.0, 100.0),
    (CRITICAL_DAMAGE, 1.5, 0.0, 100.0),
    (EXPERIENCE_MULTIPLIER, 1.0, 0.0, 100.0),
    (JUMP_COUNT, 1.0, 1.0, 100.0),
    (MINING_SPEED_MULTIPLIER, 1.0, 0.0, 100.0),
    (USE_ITEM_SPEED, 0.2, 0.0, 100.0),
];

/// Declares the twelve attributes with PlutoLib, once. The module decoders call this (through
/// [`is_declared`]) so the declarations stand whether the data or the hooks load first.
pub(crate) fn declare() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        for (id, default, min, max) in DECLARED {
            let def = pluto_lib::AttributeDef { id: Identifier::parse(id).expect("declared attribute id"), default: *default, min: *min, max: *max };
            let _ = AttributeRegistry::register(def);
        }
    });
}

/// Whether `name` is an attribute declared here (a module naming one binds as a custom
/// attribute module instead of staying unbound).
pub(crate) fn is_declared(name: &str) -> bool {
    declare();
    AttributeRegistry::is_registered(name)
}

/// One effect's attribute payload: while the effect is active on an entity, it contributes
/// `amount` per level (amplifier + 1) to `attribute` under `operation`, as the original's
/// `addAttributeModifier` calls declare.
struct EffectGrant {
    effect: &'static str,
    attribute: &'static str,
    amount: f64,
    operation: AttributeOperation,
}

/// The effects' attribute payloads, from TinkerEffects, AntigravityEffect,
/// SelfDestructiveModifier and the insatiable registration in TinkerModifiers.
const EFFECT_GRANTS: &[EffectGrant] = &[
    EffectGrant { effect: "tconstruct:experienced", attribute: EXPERIENCE_MULTIPLIER, amount: 0.25, operation: AttributeOperation::MultiplyBase },
    EffectGrant { effect: "tconstruct:ricochet", attribute: KNOCKBACK_MULTIPLIER, amount: 0.5, operation: AttributeOperation::MultiplyBase },
    EffectGrant { effect: "tconstruct:bouncy", attribute: BOUNCY, amount: 1.0, operation: AttributeOperation::Addition },
    EffectGrant { effect: "tconstruct:double_jump", attribute: JUMP_COUNT, amount: 1.0, operation: AttributeOperation::Addition },
    EffectGrant { effect: "tconstruct:pierce", attribute: "minecraft:armor", amount: -1.0, operation: AttributeOperation::Addition },
    EffectGrant { effect: "tconstruct:insatiable_armor", attribute: "minecraft:attack_damage", amount: 1.0, operation: AttributeOperation::Addition },
    EffectGrant { effect: "tconstruct:antigravity", attribute: "minecraft:gravity", amount: -2.0, operation: AttributeOperation::MultiplyTotal },
    EffectGrant { effect: "tconstruct:self_destructing", attribute: "minecraft:movement_speed", amount: -0.9, operation: AttributeOperation::MultiplyTotal },
];

/// The base additions the active effects make to the host's own attributes, as (attribute,
/// amount): the addition payloads only, scaled by each effect's level. Multiply-total
/// payloads on a host attribute are not here (a base move cannot express them).
fn vanilla_effect_additions(effects: &[(String, i32)]) -> Vec<(String, f64)> {
    effects.iter().filter_map(|(id, level)| {
        let grant = EFFECT_GRANTS.iter().find(|grant| grant.effect == id)?;
        if grant.operation != AttributeOperation::Addition || AttributeRegistry::is_registered(grant.attribute) { return None; }
        Some((grant.attribute.to_owned(), grant.amount * f64::from(*level)))
    }).collect()
}

/// The golden total the golden attribute modules among `pieces` are worked out over, when any
/// piece carries one: how many of the pieces have gold.
fn golden_total(pieces: &[(String, StackData)]) -> Option<i32> {
    let present = pieces.iter().any(|(_, piece)| matches!(with_held_tool(piece, |env, stack| tool_stack::has_golden_modules(&stack.custom_data, env)), Ok(true)));
    present.then(|| pieces.iter().filter(|(_, piece)| max_armor::is_golden(piece)).count() as i32)
}

/// Every contribution the worn `pieces`, the held `hands` and the active `effects` (as
/// (effect id, level)) make to the declared attributes, by attribute id: the plain attribute
/// modules of each stack for its slot, the max armor claims at their largest level, the
/// golden amounts over `total_gold` golden pieces (when the caller counted them), and the
/// effects' payloads scaled by level.
pub(crate) fn collect(pieces: &[(String, StackData)], hands: &[(String, StackData)], effects: &[(String, i32)], total_gold: Option<i32>) -> BTreeMap<String, Vec<AttributeContribution>> {
    declare();
    let mut contributions: BTreeMap<String, Vec<AttributeContribution>> = BTreeMap::new();
    let add = |contributions: &mut BTreeMap<String, Vec<AttributeContribution>>, attribute: String, operation: AttributeOperation, amount: f64| {
        contributions.entry(attribute).or_default().push(AttributeContribution { operation, amount });
    };
    for (slot, piece) in pieces.iter().chain(hands.iter()) {
        if let Ok(amounts) = with_held_tool(piece, |env, stack| tool_stack::custom_attribute_amounts(&stack.custom_data, env, slot)) {
            for (attribute, operation, amount) in amounts { add(&mut contributions, attribute, operation, amount); }
        }
    }
    let mut candidates = Vec::new();
    for (_, piece) in pieces {
        if let Ok(claims) = with_held_tool(piece, |env, stack| tool_stack::max_armor_claims(&stack.custom_data, env)) { candidates.extend(claims); }
    }
    for (attribute, operation, amount) in max_armor::strongest_custom(candidates) { add(&mut contributions, attribute, operation, amount); }
    if let Some(total_gold) = total_gold {
        let mut claims = Vec::new();
        for (_, piece) in pieces {
            if let Ok(amounts) = with_held_tool(piece, |env, stack| tool_stack::golden_amounts(&stack.custom_data, env, total_gold)) { claims.extend(amounts); }
        }
        for (attribute, operation, amount) in max_armor::golden_custom(claims) { add(&mut contributions, attribute, operation, amount); }
    }
    for (id, level) in effects {
        if let Some(grant) = EFFECT_GRANTS.iter().find(|grant| grant.effect == id) {
            if AttributeRegistry::is_registered(grant.attribute) {
                add(&mut contributions, grant.attribute.to_owned(), grant.operation, grant.amount * f64::from(*level));
            }
        }
    }
    contributions
}

/// The value of `attribute` for an entity wearing `pieces`, holding `hands` and carrying
/// `effects` (as (effect id, level)): the declared default when nothing contributes.
pub(crate) fn value_from(pieces: &[(String, StackData)], hands: &[(String, StackData)], effects: &[(String, i32)], attribute: &str) -> f64 {
    let contributions = collect(pieces, hands, effects, golden_total(pieces));
    AttributeRegistry::value(attribute, contributions.get(attribute).map(Vec::as_slice).unwrap_or(&[])).unwrap_or(0.0)
}

/// The value of `attribute` from one loaded stack's modules for `slot`, plus the effect
/// payloads among `effects` (as (effect id, level)): the form for a flow that already holds
/// the stack as a tool (the break-speed hook's digging tool).
pub(crate) fn stack_value(nbt: &pluto_lib::NbtCompound, env: &tool_stack::Environment, slot: &str, effects: &[(String, i32)], attribute: &str) -> f64 {
    declare();
    let mut contributions = Vec::new();
    for (named, operation, amount) in tool_stack::custom_attribute_amounts(nbt, env, slot) {
        if named == attribute { contributions.push(AttributeContribution { operation, amount }); }
    }
    for (id, level) in effects {
        if let Some(grant) = EFFECT_GRANTS.iter().find(|grant| grant.effect == id && grant.attribute == attribute) {
            contributions.push(AttributeContribution { operation: grant.operation, amount: grant.amount * f64::from(*level) });
        }
    }
    AttributeRegistry::value(attribute, &contributions).unwrap_or(0.0)
}

/// The multiplier an effect landing on an entity gets to its duration: the good effect
/// duration multiplier for a beneficial effect, the bad one otherwise, from the entity's
/// worn pieces and active effects.
pub(crate) fn effect_duration_multiplier(pieces: &[(String, StackData)], effects: &[(String, i32)], beneficial: bool) -> f64 {
    value_from(pieces, &[], effects, if beneficial { GOOD_EFFECT_DURATION_MULTIPLIER } else { BAD_EFFECT_DURATION_MULTIPLIER })
}

/// A duration after the multiplier the original applies on an effect's start: scaled and
/// rounded, at least a tick, and unchanged when the multiplier is 1.
pub(crate) fn scaled_duration(duration: i32, multiplier: f64) -> i32 {
    if duration <= 0 || multiplier == 1.0 || !multiplier.is_finite() { return duration; }
    (f64::from(duration) * multiplier).round().max(1.0) as i32
}

/// The players' custom attribute values as of their last tick, by entity id: only the values
/// that differ from their default are kept.
static VALUES: Mutex<BTreeMap<i32, BTreeMap<String, f64>>> = Mutex::new(BTreeMap::new());

/// A player's custom attribute value as of their last tick, or the declared default when the
/// tick kept none (the reader's entity is not a player the tick covers, or nothing bends the
/// attribute). 0 for an attribute nobody declared.
pub(crate) fn value(entity: i32, attribute: &str) -> f64 {
    if let Some(stored) = VALUES.lock().ok().and_then(|values| values.get(&entity).and_then(|attributes| attributes.get(attribute).copied())) { return stored; }
    AttributeRegistry::get(attribute).map(|def| def.default).unwrap_or(0.0)
}

/// Registers the callbacks. Called once from the product's hooks, after the melee hook (the
/// knockback callback here scales the strength the tool's modules have already shaped).
pub(crate) fn register() {
    declare();
    ServerLivingEntityEvents::PLAYER_TICK.register(|tick| {
        // The golden amounts count the wearer's whole armor, the game's own golden pieces
        // included, so when a worn piece carries a golden module the armor is read whole.
        let total_gold = if tick.armor.iter().any(|(_, piece)| matches!(with_held_tool(piece, |env, stack| tool_stack::has_golden_modules(&stack.custom_data, env)), Ok(true))) {
            let player = ServerPlayer { uuid: tick.player, name: String::new() };
            let pieces: Vec<StackData> = (100..=103).filter_map(|slot| player.stack(slot)).collect();
            Some(max_armor::gold_count(&pieces))
        } else { None };
        let effects: Vec<(String, i32)> = tick.effects.iter().map(|(id, amplifier, _)| (id.clone(), amplifier + 1)).collect();
        let contributions = collect(&tick.armor, &tick.hands, &effects, total_gold);
        let mut stored = BTreeMap::new();
        for def in AttributeRegistry::registered() {
            let value = pluto_lib::attribute_value(&def, contributions.get(&def.id.to_string()).map(Vec::as_slice).unwrap_or(&[]));
            if value != def.default { stored.insert(def.id.to_string(), value); }
        }
        if let Ok(mut values) = VALUES.lock() {
            if stored.is_empty() { values.remove(&tick.entity); } else { values.insert(tick.entity, stored); }
        }
        tick.attribute_additions.borrow_mut().extend(vanilla_effect_additions(&effects));
    });
    // LivingKnockBackEvent: the strength is scaled by the victim's knockback multiplier.
    ServerLivingEntityEvents::MODIFY_KNOCKBACK.register(|push| {
        if push.cancel.get() { return; }
        let multiplier = value(push.entity, KNOCKBACK_MULTIPLIER);
        if multiplier != 1.0 { push.strength.set(push.strength.get() * multiplier); }
    });
}
