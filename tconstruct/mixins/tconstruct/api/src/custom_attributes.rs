//! Tinkers' own attributes (TinkerAttributes), declared with PlutoLib's attribute registry
//! (attribute_api_v1) now that a mod can register an attribute the host does not have.
//!
//! The original registers twelve attributes of its own and reads them in its event handlers.
//! Here each is declared with its original default and range, the equipment, tool and effect
//! contributions to it are collected from the stacks an entity wears and holds, and the value
//! is worked out with PlutoLib's arithmetic. A player's values are recomputed on their tick,
//! and a mob's on its mob tick, and kept by entity id for the flows that read them: the mining speed multiplier in the
//! break-speed hook, the knockback multiplier on the knockback event, the crouch damage
//! multiplier in the melee hook, the bouncy value in the fall handler (`crate::bounce`), and
//! the effect duration multipliers where the hit hooks put
//! effects on a target. Chrysophilite is read in the loot flow itself, which already worked
//! the golden amount out from the worn pieces. Bouncy is read by the fall handler, the
//! critical damage, jump boost, jump count and use item speed by the attribute hooks, and
//! the self-destructing movement-speed payload is folded into the base value on the tick.
//! The experience multiplier still has no surface: the host has no hook for the experience
//! a block break or a death drops, so its value is computed and kept here and nothing reads
//! it until that reach exists. The original's percent and multiplier registrations do not
//! state their ranges in the source this port follows, so the declared ranges use the widest
//! bounds the original's own registrations use (0 to 100); no shipped content comes near them.
//!
//! An effect's attribute payload cannot ride its declaration (the schema carries only a
//! category and a colour), so the payloads are held here as a table and applied while the
//! effect is active: a payload naming one of the twelve joins the value's contributions, and
//! one naming a host attribute with the addition operation goes out with the tick's base
//! additions. The self-destructing multiply-total payload on the movement speed is folded
//! into the base value instead (see `self_destructing_factor`); antigravity's gravity has
//! no host attribute (gravity is hardcoded 0.08) and is realised by the mob-effect tick
//! module instead. SlimeKnights (MIT; see LICENSE).
use crate::{break_speed_hook::with_held_tool, max_armor, tool_stack};
use pluto_lib::{ActionTarget, AttributeContribution, AttributeOperation, AttributeRegistry, EntityAction, Identifier, PlayerTick, ServerEntityEvents, ServerLivingEntityEvents, ServerPlayer, StackData};
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

/// The factor a multiply-total payload of `amount` folds into a base value while its effect
/// is active on the tick, not being taken off by another callback this tick, and still kept
/// by the piece that granted it: 1 + the amount while all three hold, 1.0 otherwise. The
/// amount is deliberately UNSCALED by level: a vanilla effect attribute modifier applies its
/// fixed amount once whatever the amplifier, and the self-destructing amplifier encodes the
/// explosion radius (amplifier + 1), not an attribute level.
fn fold_factor(amount: f64, active: bool, removing: bool, kept: bool) -> f64 {
    if active && !removing && kept { 1.0 + amount } else { 1.0 }
}

/// Whether the helmet the tick wears still carries the self-destructive modifier: kept in
/// step with armor_interact_hook.rs, which takes the effect off when the helmet leaves.
fn helmet_keeps_self_destructing(tick: &PlayerTick) -> bool {
    tick.armor.iter().find(|(slot, _)| slot == "head")
        .is_some_and(|(_, head)| with_held_tool(head, |env, tool| tool_stack::modifier_level(&tool.custom_data, env, "tconstruct:self_destructive") > 0).unwrap_or(false))
}

/// The factor the self-destructing effect folds into the base value of the movement speed
/// while it is primed on the tick: 0.1 while the effect is active, not being taken off, and
/// the helmet that primed it still keeps it; 1.0 otherwise (which unfolds it). The helmet
/// check matters because this callback registers before the armor-interact one: on the tick
/// the helmet leaves, the removal is not yet queued when this runs, and without the check
/// the fold would stay stuck, as the tick is not heard again once the helmet is gone.
fn self_destructing_factor(tick: &PlayerTick) -> f64 {
    let Some(grant) = EFFECT_GRANTS.iter().find(|grant| grant.effect == "tconstruct:self_destructing") else { return 1.0 };
    let active = tick.effects.iter().any(|(id, _, _)| id == grant.effect);
    let removing = tick.remove_effects.borrow().iter().any(|id| id == grant.effect);
    fold_factor(grant.amount, active, removing, helmet_keeps_self_destructing(tick))
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

/// The entities' custom attribute values as of their last tick, by entity id: only the values
/// that differ from their default are kept.
static VALUES: Mutex<BTreeMap<i32, BTreeMap<String, f64>>> = Mutex::new(BTreeMap::new());

/// An entity's custom attribute value as of its last tick (a player's or a mob's), or the
/// declared default when the tick kept none (the entity is not one the ticks cover, or
/// nothing bends the attribute). 0 for an attribute nobody declared.
pub(crate) fn value(entity: i32, attribute: &str) -> f64 {
    if let Some(stored) = VALUES.lock().ok().and_then(|values| values.get(&entity).and_then(|attributes| attributes.get(attribute).copied())) { return stored; }
    AttributeRegistry::get(attribute).map(|def| def.default).unwrap_or(0.0)
}

/// Keeps the values the `contributions` work out to for `entity`, as of this tick: only the
/// values that differ from their default are kept, and an entity nothing bends is forgotten.
fn store_values(entity: i32, contributions: &BTreeMap<String, Vec<AttributeContribution>>) {
    let mut stored = BTreeMap::new();
    for def in AttributeRegistry::registered() {
        let value = pluto_lib::attribute_value(&def, contributions.get(&def.id.to_string()).map(Vec::as_slice).unwrap_or(&[]));
        if value != def.default { stored.insert(def.id.to_string(), value); }
    }
    if let Ok(mut values) = VALUES.lock() {
        if stored.is_empty() { values.remove(&entity); } else { values.insert(entity, stored); }
    }
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
        store_values(tick.entity, &contributions);
        tick.attribute_additions.borrow_mut().extend(vanilla_effect_additions(&effects));
        // The self-destructing effect's movement-speed payload folds into the base value (a
        // multiply-total cannot ride a base move): the action lands at the end of the tick, and
        // the factor is 1.0 once the effect is gone, which unfolds it. It is pushed every tick
        // the callback runs; the fold is a ratio, so a repeated 1.0 changes nothing.
        tick.actions.borrow_mut().push(EntityAction::MultiplyAttribute { target: ActionTarget::Attacker, attribute: "minecraft:movement_speed".to_owned(), factor: self_destructing_factor(tick) });
    });
    // A mob's values are recomputed on its tick the same way, from the pieces its report
    // carries: the report lists every piece worn (not only those carrying custom data), so the
    // golden amounts count the golden pieces among them directly.
    ServerLivingEntityEvents::MOB_TICK.register(|tick| {
        let effects: Vec<(String, i32)> = tick.effects.iter().map(|(id, amplifier, _)| (id.clone(), amplifier + 1)).collect();
        let contributions = collect(&tick.armor, &tick.hands, &effects, golden_total(&tick.armor));
        store_values(tick.entity, &contributions);
    });
    // A mob's kept values go with it when it leaves the world (its entity id is not reused by
    // the same mob); a player's live for the session.
    ServerEntityEvents::ENTITY_UNLOAD.register(|info| {
        if info.kind != "minecraft:player" {
            if let Ok(mut values) = VALUES.lock() { values.remove(&(info.id as i32)); }
        }
    });
    // LivingKnockBackEvent: the strength is scaled by the victim's knockback multiplier.
    ServerLivingEntityEvents::MODIFY_KNOCKBACK.register(|push| {
        if push.cancel.get() { return; }
        let multiplier = value(push.entity, KNOCKBACK_MULTIPLIER);
        if multiplier != 1.0 { push.strength.set(push.strength.get() * multiplier); }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    fn tick_for(entity: i32, effects: Vec<(String, i32, i32)>, armor: Vec<(String, StackData)>, remove: Vec<String>) -> PlayerTick {
        PlayerTick {
            player: entity as u128, entity, dimension: "minecraft:overworld".to_owned(),
            position: [0.0; 3], on_ground: true, sneaking: false, gliding: false, spectator: false, in_water: false,
            armor, hands: Vec::new(), block_light: 0, sky_light: 0, sky_darken: 0, effects,
            remove_effects: RefCell::new(remove), attribute_additions: RefCell::new(Vec::new()), actions: RefCell::new(Vec::new()),
        }
    }

    fn head(item: &str) -> (String, StackData) {
        ("head".to_owned(), StackData { item: item.into(), count: 1, ..StackData::default() })
    }

    #[test]
    fn the_fold_factor_is_a_tenth_while_primed_and_kept_and_one_otherwise() {
        // 1.0 - 0.9 is not exactly 0.1 in binary, so the tenth is compared loosely.
        assert!((fold_factor(-0.9, true, false, true) - 0.1).abs() < 1e-9, "primed and kept: 1 - 0.9");
        assert_eq!(fold_factor(-0.9, false, false, true), 1.0, "no effect, nothing to fold");
        assert_eq!(fold_factor(-0.9, true, true, true), 1.0, "being taken off, unfold");
        assert_eq!(fold_factor(-0.9, true, false, false), 1.0, "the helmet left, unfold");
        assert!((fold_factor(-0.9, true, false, true) - 0.1).abs() < 1e-9, "the level is not scaled: the amplifier is the blast radius, not a level");
    }

    #[test]
    fn the_self_destructing_factor_follows_the_effect_and_the_helmet() {
        let primed = vec![("tconstruct:self_destructing".to_owned(), 1, 30)];
        assert_eq!(self_destructing_factor(&tick_for(7705, Vec::new(), Vec::new(), Vec::new())), 1.0, "no effect");
        assert_eq!(self_destructing_factor(&tick_for(7705, primed.clone(), Vec::new(), Vec::new())), 1.0, "no helmet to keep it");
        assert_eq!(
            self_destructing_factor(&tick_for(7705, primed.clone(), vec![head("tconstruct:helmet")], Vec::new())),
            1.0,
            "a head piece with no tool data keeps nothing"
        );
        assert_eq!(
            self_destructing_factor(&tick_for(7705, primed, vec![head("tconstruct:helmet")], vec!["tconstruct:self_destructing".to_owned()])),
            1.0,
            "being taken off this tick, unfold"
        );
    }

    #[test]
    fn the_tick_pushes_the_fold_action() {
        register();
        let tick = tick_for(7706, Vec::new(), Vec::new(), Vec::new());
        ServerLivingEntityEvents::PLAYER_TICK.invoke(&tick);
        assert!(
            tick.actions.borrow().iter().any(|action| matches!(action,
                EntityAction::MultiplyAttribute { target: ActionTarget::Attacker, attribute, factor }
                if attribute == "minecraft:movement_speed" && *factor == 1.0)),
            "the fold action is queued with 1.0 while nothing is primed: {:?}",
            tick.actions.borrow()
        );
    }
}
