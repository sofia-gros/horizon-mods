//! Counterpart of Fabric's `fabric-entity-events-v1`: entity load and unload, and living-entity damage.
//!
//! Entities are identified by their numeric id (Fabric passes entity objects); PlutoLib grows the data as modules need it.

use std::cell::{Cell, RefCell};
use super::block_entity_api_v1::ItemSlot;

pub use super::entity_spawn_api::{MobSpawnInfo,MobSpawnPreparation,MobSpawnCompletion,MobSpawnAction,MobSpawnCallback};
use super::base::Event;
use super::player_api::StackData;

/// An entity entering or leaving a world.
#[derive(Clone, Debug, PartialEq)]
pub struct EntityInfo {
    pub id: u32,
    /// The entity type id, such as `minecraft:zombie`.
    pub kind: String,
    pub dimension: String,
}

/// Damage about to be applied to a living entity.
#[derive(Clone, Debug, PartialEq)]
pub struct LivingDamage {
    pub entity: i32,
    pub kind: String,
    /// The damage type id, such as `minecraft:player_attack`.
    pub damage_type: String,
    /// The entity that caused it (the attacker), or a negative id if none.
    pub attacker: i32,
    /// The entity that directly dealt it (a projectile, for example), or a negative id if none.
    pub direct: i32,
    pub amount: f32,
}

static ENTITY_LOAD: Event<EntityInfo, ()> = Event::void();
static ENTITY_UNLOAD: Event<EntityInfo, ()> = Event::void();

/// Fabric: `ServerEntityEvents`.
pub struct ServerEntityEvents;

impl ServerEntityEvents {
    pub const ENTITY_LOAD: &'static Event<EntityInfo, ()> = &ENTITY_LOAD;
    pub const ENTITY_UNLOAD: &'static Event<EntityInfo, ()> = &ENTITY_UNLOAD;
}

/// What a living entity is doing, as the game reports it when a player's weapon hits a mob.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EntityState {
    /// The entity type id, such as `minecraft:husk`.
    pub kind: String,
    pub sprinting: bool,
    /// Whether it stands on the ground; false in the air.
    pub on_ground: bool,
    /// Whether it is in water.
    pub in_water: bool,
    /// How long it burns for, in ticks; 0 or less when it does not.
    pub fire_ticks: i32,
    pub fire_immune: bool,
    /// The ids of the effects on it, such as `minecraft:poison`.
    pub effects: Vec<String>,
}

/// A status effect for a callback to put on the mob a weapon hit.
#[derive(Clone, Debug, PartialEq)]
pub struct EffectGrant {
    /// The effect id, such as `minecraft:slowness`; an id the game does not know is skipped.
    pub effect: String,
    /// The amplifier, the level less one: 0 is level I.
    pub amplifier: i32,
    /// How long it lasts, in ticks.
    pub duration: i32,
}

/// A player's weapon hit on a mob, heard on a server before the hit's damage lands.
#[derive(Clone, Debug)]
pub struct WeaponHit {
    /// The mob's entity id and type id, and the attacking player's entity id.
    pub entity: i32,
    pub kind: String,
    pub attacker: i32,
    /// The weapon: its damage, max damage and custom data; None for an empty hand.
    pub weapon: Option<StackData>,
    /// What the player and the mob were doing as the hit landed; None when the game could not tell.
    pub attacker_state: Option<EntityState>,
    pub target_state: Option<EntityState>,
    /// The effects to put on the mob, which callbacks add to.
    pub effects: RefCell<Vec<EffectGrant>>,
}

/// A server player's tick, heard while the player wears armor that carries custom data.
#[derive(Clone, Debug)]
pub struct PlayerTick {
    /// The player's uuid and entity id.
    pub player: u128,
    pub entity: i32,
    /// The armor the player wears, as (slot, stack): `feet`, `legs`, `chest`, `head`, only the pieces carrying custom data.
    pub armor: Vec<(String, StackData)>,
    /// The effects on the player, as (effect id, amplifier, ticks left).
    pub effects: Vec<(String, i32, i32)>,
    /// The effects to take off the player, which callbacks add to.
    pub remove_effects: RefCell<Vec<String>>,
    /// The amounts callbacks add to the base value of the player's attributes, as (attribute id, amount), which callbacks push to every tick
    /// for as long as they want them: an amount no callback names any more is taken off again.
    pub attribute_additions: RefCell<Vec<(String, f64)>>,
}

/// Damage for a callback to deal to the mob that hurt the player (an armor's thorns).
#[derive(Clone, Debug, PartialEq)]
pub struct CounterDamage {
    /// The damage type id, such as `minecraft:thorns`.
    pub damage_type: String,
    pub amount: f32,
}

/// A mob hurting the player, heard on a server as the hit starts (before armor wears and the damage lands).
#[derive(Clone, Debug)]
pub struct WearerHit {
    /// The mob's entity id and type id.
    pub attacker: i32,
    pub kind: String,
    /// The damage type id and the amount the game works the hit out as: not reported yet, so an empty id and 0.0 for now (the hit is
    /// heard before the game works either out).
    pub damage_type: String,
    pub amount: f32,
    /// What the mob was doing as the hit landed.
    pub attacker_state: EntityState,
    /// The armor the player wore as of their last tick, as (slot, stack): `feet`, `legs`, `chest`, `head`, only the pieces carrying custom data.
    pub armor: Vec<(String, StackData)>,
    /// The effects to put on the mob, which callbacks add to.
    pub effects: RefCell<Vec<EffectGrant>>,
    /// The effects to put on the wearer, which callbacks add to; they land on the player's next tick.
    pub self_effects: RefCell<Vec<EffectGrant>>,
    /// The damage to deal to the mob, which callbacks add to; it lands on the player's next tick, as hurt by the player.
    pub counter_damage: RefCell<Vec<CounterDamage>>,
}

/// Damage about to be applied, which callbacks may change (NeoForge: `LivingDamageEvent.Pre`).
#[derive(Clone, Debug)]
pub struct DamageModification {
    /// What is known of the damage; its `amount` is the amount as the game works it out and does not change.
    pub damage: LivingDamage,
    /// The amount now, which callbacks may change with `set`; every callback is called in turn, each seeing the last one's.
    pub amount: Cell<f32>,
    /// When a player's attack caused it: the stack the player attacked with (damage, max damage, custom data); None otherwise.
    pub attacker_stack: Option<StackData>,
    /// The armor the damaged entity wore as of its last tick, as (slot, stack): `feet`, `legs`, `chest`, `head`, only the
    /// pieces carrying custom data (a mod's own armor keeps its state there). Empty for an entity that is not a player.
    pub armor: Vec<(String, StackData)>,
    /// When a player's weapon hit a mob: what the player and the mob were doing as the hit landed; None otherwise.
    pub attacker_state: Option<EntityState>,
    pub target_state: Option<EntityState>,
}

/// Knockback about to be applied to a living entity, which callbacks may change or cancel (NeoForge: `LivingKnockBackEvent`).
#[derive(Clone, Debug)]
pub struct KnockbackModification {
    pub entity: i32,
    pub kind: String,
    /// The entity that caused it, or a negative id if none.
    pub attacker: i32,
    /// The stack in the attacker's main hand, with its damage, max damage and custom data; None for an empty hand or no attacker.
    pub attacker_main: Option<StackData>,
    /// The stack in the attacker's off hand; None when empty.
    pub attacker_off: Option<StackData>,
    /// The strength now, which callbacks may change with `set`; each callback sees the last one's.
    pub strength: Cell<f64>,
    /// The direction the target is pushed away along, on the horizontal plane.
    pub x: Cell<f64>,
    pub z: Cell<f64>,
    /// Set to true for no knockback at all.
    pub cancel: Cell<bool>,
}

/// A player's item about to lose durability, which callbacks may change or cancel (NeoForge has no event for it; the vanilla
/// source is `ItemStack.hurtAndBreak`). It is heard on a server, once for each wear, after Unbreaking has had its say, and not for
/// a creative player's tools.
#[derive(Clone, Debug)]
pub struct ItemDamage {
    /// The player's uuid and name.
    pub player: u128,
    pub name: String,
    /// The item id of the stack worn.
    pub item: String,
    /// The stack as the player holds or wears it (main hand first, then the armor slots), found by its item id; None when none matches.
    pub stack: Option<StackData>,
    /// The wear now, which callbacks may change with `set` (0 or less leaves the item whole); each callback sees the last one's.
    pub amount: Cell<i32>,
    /// Set to true for no wear at all.
    pub cancel: Cell<bool>,
    /// The worn stack as an item slot, with its components (tool, attribute modifiers, enchantments); None when `stack` is.
    pub slot: Option<ItemSlot>,
    /// A whole new stack for the worn one, which a callback may set; it is written to the main-inventory stack that carried the data
    /// in `stack`, and then the game wears it by 0. When the stack is not in the main inventory (a worn armor piece) it is not
    /// written and the game wears the stack by `amount`, so a callback that sets `replacement` also sets `amount` to the wear left.
    pub replacement: RefCell<Option<ItemSlot>>,
}

static PLAYER_TICK: Event<PlayerTick, ()> = Event::void();
static WEAPON_HIT: Event<WeaponHit, ()> = Event::void();
static WEARER_HIT: Event<WearerHit, ()> = Event::void();
static MODIFY_KNOCKBACK: Event<KnockbackModification, ()> = Event::void();
static MODIFY_ITEM_DAMAGE: Event<ItemDamage, ()> = Event::void();
static MODIFY_DAMAGE: Event<DamageModification, ()> = Event::void();
static ALLOW_DAMAGE: Event<LivingDamage, bool> = Event::allow_all();
static AFTER_DEATH: Event<LivingDamage, ()> = Event::void();

/// Fabric: `ServerLivingEntityEvents`.
pub struct ServerLivingEntityEvents;

impl ServerLivingEntityEvents {
    /// Fabric: `ALLOW_DAMAGE`: return `false` to cancel the damage.
    pub const ALLOW_DAMAGE: &'static Event<LivingDamage, bool> = &ALLOW_DAMAGE;
    /// NeoForge: `LivingDamageEvent.Pre`. Called for damage that goes ahead, before armor and effects take their share; change the amount
    /// with `amount.set`. For a player's attack on the server it carries the stack used.
    pub const MODIFY_DAMAGE: &'static Event<DamageModification, ()> = &MODIFY_DAMAGE;
    /// Fabric: `AFTER_DEATH`. Called when a living entity's death goes ahead, just before it is removed. Players are not reported yet.
    pub const AFTER_DEATH: &'static Event<LivingDamage, ()> = &AFTER_DEATH;
    /// A server player's tick, heard while they wear armor carrying custom data: take effects off them with `remove_effects`.
    pub const PLAYER_TICK: &'static Event<PlayerTick, ()> = &PLAYER_TICK;
    /// A mob hurting the player, as the hit starts: push status effects on the mob with `effects`. Server only.
    pub const WEARER_HIT: &'static Event<WearerHit, ()> = &WEARER_HIT;
    /// A player's weapon hit on a mob, before its damage lands: push status effects on the mob with `effects`. Server only.
    pub const WEAPON_HIT: &'static Event<WeaponHit, ()> = &WEAPON_HIT;
    /// NeoForge: `LivingKnockBackEvent`. Change the strength or direction with `set`, or set `cancel`. Server only.
    pub const MODIFY_KNOCKBACK: &'static Event<KnockbackModification, ()> = &MODIFY_KNOCKBACK;
    /// A player's item losing durability: change the amount with `amount.set` or set `cancel`. Server only.
    pub const MODIFY_ITEM_DAMAGE: &'static Event<ItemDamage, ()> = &MODIFY_ITEM_DAMAGE;
}

/// The effects the callbacks take off the player on their tick.
pub(crate) fn player_tick(tick: &PlayerTick) -> Vec<String> {
    PLAYER_TICK.invoke(tick);
    tick.remove_effects.borrow_mut().drain(..).collect()
}

static ATTRIBUTES: std::sync::Mutex<std::collections::BTreeMap<(u128, String), f64>> = std::sync::Mutex::new(std::collections::BTreeMap::new());
static TICKING: std::sync::Mutex<std::collections::BTreeSet<u128>> = std::sync::Mutex::new(std::collections::BTreeSet::new());

/// Whether the player's tick is heard: while they wear armor carrying custom data, and for one tick after the last such piece comes off, so
/// that what the callbacks kept for the armor (an attribute amount) is let go.
pub(crate) fn tick_wanted(player: u128, armor_worn: bool) -> bool {
    let Ok(mut ticking) = TICKING.lock() else { return armor_worn };
    if armor_worn { ticking.insert(player); true } else { ticking.remove(&player) }
}

/// The changes to a player's attribute bases for the amounts `wanted` (summed per attribute): (attribute, change, the total then applied).
/// Nothing is recorded until `attribute_applied` is told the host took the change.
pub(crate) fn attribute_changes(player: u128, wanted: &[(String, f64)]) -> Vec<(String, f64, f64)> {
    let mut totals: std::collections::BTreeMap<&str, f64> = std::collections::BTreeMap::new();
    for (attribute, amount) in wanted { *totals.entry(attribute).or_insert(0.0) += amount; }
    let Ok(applied) = ATTRIBUTES.lock() else { return Vec::new() };
    let mut changes = Vec::new();
    for (attribute, total) in &totals {
        let held = applied.get(&(player, (*attribute).to_owned())).copied().unwrap_or(0.0);
        if *total != held { changes.push(((*attribute).to_owned(), total - held, *total)); }
    }
    for ((who, attribute), held) in applied.iter() {
        if *who == player && *held != 0.0 && !totals.contains_key(attribute.as_str()) { changes.push((attribute.clone(), -held, 0.0)); }
    }
    changes
}

/// Records that the host took a change: the player's total for the attribute is now `total`.
pub(crate) fn attribute_applied(player: u128, attribute: &str, total: f64) {
    if let Ok(mut applied) = ATTRIBUTES.lock() {
        if total == 0.0 { applied.remove(&(player, attribute.to_owned())); } else { applied.insert((player, attribute.to_owned()), total); }
    }
}

/// What a player who leaves holds on their attributes (to take off), forgotten.
pub(crate) fn forget_attributes(player: u128) -> Vec<(String, f64)> {
    if let Ok(mut ticking) = TICKING.lock() { ticking.remove(&player); }
    let Ok(mut applied) = ATTRIBUTES.lock() else { return Vec::new() };
    let held: Vec<(String, f64)> = applied.iter().filter(|((who, _), _)| *who == player).map(|((_, attribute), held)| (attribute.clone(), *held)).collect();
    applied.retain(|(who, _), _| *who != player);
    held
}

/// The effects the callbacks put on a mob that hurt the player (`wearer`, the entity id of the player the armor is attributed to); the ones for
/// the wearer and the damage for the mob are queued for that player's next tick, and dropped when no wearer is known.
pub(crate) fn wearer_hit(hit: &WearerHit, wearer: Option<i32>) -> Vec<EffectGrant> {
    WEARER_HIT.invoke(hit);
    let own: Vec<(i32, EffectGrant)> = hit.self_effects.borrow_mut().drain(..).filter_map(|grant| wearer.map(|wearer| (wearer, grant))).collect();
    if !own.is_empty() { if let Ok(mut queue) = SELF_EFFECTS.lock() { queue.extend(own); } }
    let damage: Vec<(i32, i32, CounterDamage)> = hit.counter_damage.borrow_mut().drain(..).filter_map(|damage| wearer.map(|wearer| (wearer, hit.attacker, damage))).collect();
    if !damage.is_empty() { if let Ok(mut queue) = COUNTER_DAMAGE.lock() { queue.extend(damage); } }
    hit.effects.borrow_mut().drain(..).collect()
}

/// The effects for a wearer that hits have queued, with the wearer's entity id.
static SELF_EFFECTS: std::sync::Mutex<Vec<(i32, EffectGrant)>> = std::sync::Mutex::new(Vec::new());

/// The damage for the mobs that hits have queued, with the wearer's entity id and the mob's.
static COUNTER_DAMAGE: std::sync::Mutex<Vec<(i32, i32, CounterDamage)>> = std::sync::Mutex::new(Vec::new());

/// Takes the damage queued for the mobs that hurt the wearer `wearer`, as (mob, damage); other players' stay queued.
pub(crate) fn take_counter_damage(wearer: i32) -> Vec<(i32, CounterDamage)> {
    let Ok(mut queue) = COUNTER_DAMAGE.lock() else { return Vec::new() };
    let (mine, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut *queue).into_iter().partition(|(owner, _, _)| *owner == wearer);
    *queue = rest;
    mine.into_iter().map(|(_, mob, damage)| (mob, damage)).collect()
}

/// Takes the effects queued for the wearer `wearer`; other players' stay queued.
pub(crate) fn take_self_effects(wearer: i32) -> Vec<EffectGrant> {
    let Ok(mut queue) = SELF_EFFECTS.lock() else { return Vec::new() };
    let (mine, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut *queue).into_iter().partition(|(owner, _)| *owner == wearer);
    *queue = rest;
    mine.into_iter().map(|(_, grant)| grant).collect()
}

/// The entity id of the only player wearing custom-data armor, which a hit on a player is attributed to.
pub(crate) fn only_wearer_entity() -> Option<i32> {
    ARMOR.lock().ok().and_then(|worn| if worn.len() == 1 { worn.keys().next().copied() } else { None })
}

/// What each player wore, by entity id, for a hit on them: the armor cached by `remember_armor`, for the entity the game names.
pub(crate) fn armor_of_the_only_wearer() -> Vec<(String, StackData)> {
    ARMOR.lock().ok().and_then(|worn| if worn.len() == 1 { worn.values().next().map(|(_, pieces)| pieces.clone()) } else { None }).unwrap_or_default()
}

/// The effects the callbacks put on the mob a weapon hit.
pub(crate) fn weapon_hit(hit: &WeaponHit) -> Vec<EffectGrant> {
    WEAPON_HIT.invoke(hit);
    hit.effects.borrow_mut().drain(..).collect()
}

/// The knockback after the callbacks as (strength, x, z), or None when a callback cancelled it.
pub(crate) fn modify_knockback(modification: &KnockbackModification) -> Option<(f64, f64, f64)> {
    MODIFY_KNOCKBACK.invoke(modification);
    if modification.cancel.get() { None } else { Some((modification.strength.get(), modification.x.get(), modification.z.get())) }
}

/// The wear after the callbacks, with the replacement stack a callback set, or None when a callback cancelled it.
pub(crate) fn modify_item_damage(wear: &ItemDamage) -> Option<(i32, Option<ItemSlot>)> {
    MODIFY_ITEM_DAMAGE.invoke(wear);
    if wear.cancel.get() { None } else { Some((wear.amount.get(), wear.replacement.borrow_mut().take())) }
}

pub(crate) fn entity_loaded(id: u32, kind: &str, dimension: &str) {
    ENTITY_LOAD.invoke(&EntityInfo { id, kind: kind.to_owned(), dimension: dimension.to_owned() });
}

pub(crate) fn entity_unloaded(id: u32, kind: &str, dimension: &str) {
    ENTITY_UNLOAD.invoke(&EntityInfo { id, kind: kind.to_owned(), dimension: dimension.to_owned() });
}

/// True when the damage may be applied.
pub(crate) fn after_death(death: &LivingDamage) {
    AFTER_DEATH.invoke(death);
}

thread_local! {
    static PENDING_ATTACK: RefCell<Option<(i32, StackData)>> = const { RefCell::new(None) };
    static PENDING_STATES: RefCell<Option<(i32, Option<EntityState>, Option<EntityState>, std::time::Instant)>> = const { RefCell::new(None) };
}

/// How long the states of a hit stay good: the damage event follows in the same call, so a few milliseconds is plenty, and a longer gap
/// means the hit never reached it (cancelled, absorbed) and the states belong to something that is over.
const HIT_STATES_LIFETIME: std::time::Duration = std::time::Duration::from_millis(100);

/// A hit is landing on `entity`: what the attacker and the target were doing, remembered for the damage that follows in the same call.
pub(crate) fn hit_states(entity: i32, attacker: Option<EntityState>, target: Option<EntityState>) {
    PENDING_STATES.with(|pending| *pending.borrow_mut() = Some((entity, attacker, target, std::time::Instant::now())));
}

/// A player is attacking `entity` with `stack`: remembered for the damage that follows in the same call.
pub(crate) fn attack_started(entity: i32, stack: Option<StackData>) {
    PENDING_ATTACK.with(|pending| *pending.borrow_mut() = stack.map(|s| (entity, s)));
}

/// What each player wears, by entity id, with the player's uuid so a logout can forget it.
static ARMOR: std::sync::Mutex<std::collections::BTreeMap<i32, (u128, Vec<(String, StackData)>)>> = std::sync::Mutex::new(std::collections::BTreeMap::new());

/// Records what the player (`entity`) wears; an empty list forgets it.
pub(crate) fn remember_armor(uuid: u128, entity: i32, armor: Vec<(String, StackData)>) {
    let Ok(mut worn) = ARMOR.lock() else { return };
    if armor.is_empty() { worn.remove(&entity); } else { worn.insert(entity, (uuid, armor)); }
}

/// Forgets a player who left.
pub(crate) fn forget_armor(uuid: u128) {
    if let Ok(mut worn) = ARMOR.lock() { worn.retain(|_, (known, _)| *known != uuid); }
}

/// The amount after the callbacks; the stack of the attack that is being applied to `damage.entity`, if there is one.
pub(crate) fn modify_damage(damage: &LivingDamage) -> f32 {
    let attacker_stack = PENDING_ATTACK.with(|pending| {
        let mut pending = pending.borrow_mut();
        match pending.take() {
            Some((entity, stack)) if entity == damage.entity => Some(stack),
            other => {
                *pending = other;
                None
            }
        }
    });
    let armor = ARMOR.lock().ok().and_then(|worn| worn.get(&damage.entity).map(|(_, pieces)| pieces.clone())).unwrap_or_default();
    let (attacker_state, target_state) = PENDING_STATES.with(|pending| {
        let mut pending = pending.borrow_mut();
        match pending.take() {
            Some((entity, attacker, target, at)) if entity == damage.entity && at.elapsed() < HIT_STATES_LIFETIME => (attacker, target),
            Some((entity, _, _, at)) if entity == damage.entity || at.elapsed() >= HIT_STATES_LIFETIME => (None, None),
            other => { *pending = other; (None, None) }
        }
    });
    let modification = DamageModification { damage: damage.clone(), amount: Cell::new(damage.amount), attacker_stack, armor, attacker_state, target_state };
    MODIFY_DAMAGE.invoke(&modification);
    modification.amount.get()
}

pub(crate) fn allow_damage(damage: &LivingDamage) -> bool {
    ALLOW_DAMAGE.invoke(damage)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modify_damage_carries_the_armor_a_player_wore_and_forgets_it_on_logout() {
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = seen.clone();
        ServerLivingEntityEvents::MODIFY_DAMAGE.register(move |hit| if hit.damage.entity == 4242 { log.lock().unwrap().push(hit.armor.iter().map(|(slot, stack)| format!("{slot}:{}", stack.item)).collect::<Vec<_>>()); });
        let hurt = LivingDamage { entity: 4242, kind: "minecraft:player".into(), damage_type: "minecraft:mob_attack".into(), attacker: 3, direct: 3, amount: 5.0 };
        let helmet = StackData { item: "example:helmet".into(), count: 1, ..StackData::default() };
        remember_armor(99, 4242, vec![("head".into(), helmet)]);
        modify_damage(&hurt);
        forget_armor(99);
        modify_damage(&hurt);
        assert_eq!(*seen.lock().unwrap(), vec![vec!["head:example:helmet".to_string()], Vec::<String>::new()]);
    }

    #[test]
    fn knockback_and_item_wear_can_be_changed_or_cancelled_by_callbacks() {
        ServerLivingEntityEvents::MODIFY_KNOCKBACK.register(|push| {
            if push.entity != 5151 { return; }
            if push.attacker_main.as_ref().is_some_and(|stack| stack.item == "example:hammer") { push.strength.set(push.strength.get() * 2.0); }
            if push.attacker_off.is_some() { push.cancel.set(true); }
        });
        ServerLivingEntityEvents::MODIFY_ITEM_DAMAGE.register(|wear| {
            if wear.name != "wearer" { return; }
            if wear.item == "example:shield" { wear.cancel.set(true); } else { wear.amount.set(wear.amount.get() - 1); }
        });
        let hammer = StackData { item: "example:hammer".into(), count: 1, ..StackData::default() };
        let push = |main: Option<StackData>, off: Option<StackData>| KnockbackModification { entity: 5151, kind: "minecraft:zombie".into(), attacker: 1, attacker_main: main, attacker_off: off, strength: Cell::new(0.4), x: Cell::new(1.0), z: Cell::new(0.0), cancel: Cell::new(false) };
        assert_eq!(modify_knockback(&push(Some(hammer.clone()), None)), Some((0.8, 1.0, 0.0)));
        assert_eq!(modify_knockback(&push(None, None)), Some((0.4, 1.0, 0.0)));
        assert_eq!(modify_knockback(&push(None, Some(hammer))), None);
        let wear = |item: &str, amount| ItemDamage { player: 1, name: "wearer".into(), item: item.into(), stack: None, slot: None, amount: Cell::new(amount), cancel: Cell::new(false), replacement: RefCell::new(None) };
        assert_eq!(modify_item_damage(&wear("example:pickaxe", 3)), Some((2, None)));
        assert_eq!(modify_item_damage(&wear("example:shield", 3)), None);
    }

    #[test]
    fn a_weapon_hit_carries_what_the_attacker_and_the_target_were_doing() {
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = seen.clone();
        ServerLivingEntityEvents::MODIFY_DAMAGE.register(move |hit| if hit.damage.entity == 6262 {
            log.lock().unwrap().push((hit.attacker_state.as_ref().map(|state| state.sprinting), hit.target_state.as_ref().map(|state| state.fire_ticks)));
        });
        let hurt = LivingDamage { entity: 6262, kind: "minecraft:husk".into(), damage_type: "minecraft:player_attack".into(), attacker: 1, direct: 1, amount: 5.0 };
        hit_states(6262, Some(EntityState { sprinting: true, ..EntityState::default() }), Some(EntityState { fire_ticks: 40, ..EntityState::default() }));
        modify_damage(&hurt);
        modify_damage(&hurt);
        assert_eq!(*seen.lock().unwrap(), vec![(Some(true), Some(40)), (None, None)], "the states belong to one hit");
        hit_states(6262, Some(EntityState { sprinting: true, ..EntityState::default() }), None);
        std::thread::sleep(std::time::Duration::from_millis(150));
        modify_damage(&hurt);
        assert_eq!(seen.lock().unwrap().last(), Some(&(None, None)), "states that outlived their hit are dropped");
    }

    #[test]
    fn a_weapon_hit_collects_the_effects_callbacks_put_on_the_mob() {
        ServerLivingEntityEvents::WEAPON_HIT.register(|hit| {
            if hit.entity != 7373 { return; }
            if hit.weapon.as_ref().is_some_and(|weapon| weapon.item == "example:wand") {
                hit.effects.borrow_mut().push(EffectGrant { effect: "minecraft:slowness".into(), amplifier: 3, duration: 20 });
            }
        });
        let hit = |item: Option<&str>| WeaponHit { entity: 7373, kind: "minecraft:husk".into(), attacker: 1, weapon: item.map(|item| StackData { item: item.into(), count: 1, ..StackData::default() }), attacker_state: None, target_state: None, effects: RefCell::new(Vec::new()) };
        assert_eq!(weapon_hit(&hit(Some("example:wand"))), vec![EffectGrant { effect: "minecraft:slowness".into(), amplifier: 3, duration: 20 }]);
        assert!(weapon_hit(&hit(Some("example:stick"))).is_empty());
        assert!(weapon_hit(&hit(None)).is_empty());
    }

    #[test]
    fn a_hit_on_the_wearer_collects_the_effects_callbacks_put_on_the_attacker() {
        ServerLivingEntityEvents::WEARER_HIT.register(|hit| {
            if hit.attacker != 8484 { return; }
            if hit.armor.iter().any(|(slot, piece)| slot == "head" && piece.item == "example:helmet") {
                hit.effects.borrow_mut().push(EffectGrant { effect: "minecraft:weakness".into(), amplifier: 0, duration: 100 });
            }
        });
        let hit = |armor: Vec<(String, StackData)>| WearerHit { attacker: 8484, kind: "minecraft:zombie".into(), damage_type: "minecraft:mob_attack".into(), amount: 3.0, attacker_state: EntityState::default(), armor, effects: RefCell::new(Vec::new()), self_effects: RefCell::new(Vec::new()), counter_damage: RefCell::new(Vec::new()) };
        let helmet = StackData { item: "example:helmet".into(), count: 1, ..StackData::default() };
        assert_eq!(wearer_hit(&hit(vec![("head".into(), helmet)]), Some(7)), vec![EffectGrant { effect: "minecraft:weakness".into(), amplifier: 0, duration: 100 }]);
        assert!(wearer_hit(&hit(Vec::new()), Some(7)).is_empty());
    }

    #[test]
    fn effects_for_the_wearer_wait_for_the_next_tick() {
        ServerLivingEntityEvents::WEARER_HIT.register(|hit| {
            if hit.attacker == 9595 { hit.self_effects.borrow_mut().push(EffectGrant { effect: "minecraft:strength".into(), amplifier: 0, duration: 200 }); }
        });
        let hit = WearerHit { attacker: 9595, kind: "minecraft:zombie".into(), damage_type: "minecraft:mob_attack".into(), amount: 3.0, attacker_state: EntityState::default(), armor: Vec::new(), effects: RefCell::new(Vec::new()), self_effects: RefCell::new(Vec::new()), counter_damage: RefCell::new(Vec::new()) };
        hit.counter_damage.borrow_mut().push(CounterDamage { damage_type: "minecraft:thorns".into(), amount: 2.0 });
        assert!(wearer_hit(&hit, Some(31)).is_empty(), "nothing goes on the attacker");
        assert!(take_counter_damage(32).is_empty() && take_self_effects(32).is_empty(), "another player's tick takes nothing");
        assert_eq!(take_counter_damage(31), vec![(9595, CounterDamage { damage_type: "minecraft:thorns".into(), amount: 2.0 })]);
        assert_eq!(take_self_effects(31), vec![EffectGrant { effect: "minecraft:strength".into(), amplifier: 0, duration: 200 }]);
        assert!(take_self_effects(31).is_empty(), "the queue is emptied once taken");
        let unattributed = WearerHit { attacker: 9595, kind: "minecraft:zombie".into(), damage_type: String::new(), amount: 0.0, attacker_state: EntityState::default(), armor: Vec::new(), effects: RefCell::new(Vec::new()), self_effects: RefCell::new(Vec::new()), counter_damage: RefCell::new(Vec::new()) };
        wearer_hit(&unattributed, None);
        assert!(take_self_effects(31).is_empty(), "a hit with no known wearer queues nothing");
    }

    #[test]
    fn a_player_tick_collects_the_effects_callbacks_take_off() {
        ServerLivingEntityEvents::PLAYER_TICK.register(|tick| {
            if tick.entity != 5757 { return; }
            for (id, amplifier, _) in &tick.effects { if id == "minecraft:poison" && *amplifier == 0 { tick.remove_effects.borrow_mut().push(id.clone()); } }
        });
        let tick = |effects: Vec<(String, i32, i32)>| PlayerTick { player: 1, entity: 5757, armor: Vec::new(), effects, remove_effects: RefCell::new(Vec::new()), attribute_additions: RefCell::new(Vec::new()) };
        assert_eq!(player_tick(&tick(vec![("minecraft:poison".into(), 0, 100), ("minecraft:speed".into(), 0, 100)])), vec!["minecraft:poison".to_string()]);
        assert!(player_tick(&tick(vec![("minecraft:poison".into(), 1, 100)])).is_empty());
    }

    #[test]
    fn attribute_amounts_are_applied_once_and_taken_off_when_no_callback_names_them() {
        let wanted = vec![("minecraft:knockback_resistance".to_string(), 0.1), ("minecraft:knockback_resistance".to_string(), 0.05)];
        let first = attribute_changes(31, &wanted);
        assert_eq!(first.len(), 1);
        assert!((first[0].1 - 0.15).abs() < 1e-12 && (first[0].2 - 0.15).abs() < 1e-12);
        attribute_applied(31, &first[0].0, first[0].2);
        assert!(attribute_changes(31, &wanted).is_empty(), "an amount already applied is not applied again");
        let less = attribute_changes(31, &wanted[..1]);
        assert!((less[0].1 + 0.05).abs() < 1e-12);
        attribute_applied(31, &less[0].0, less[0].2);
        let gone = attribute_changes(31, &[]);
        assert!((gone[0].1 + 0.1).abs() < 1e-12 && gone[0].2 == 0.0);
        attribute_applied(31, &gone[0].0, 0.0);
        assert!(attribute_changes(31, &[]).is_empty());
        attribute_applied(31, "minecraft:armor", 2.0);
        assert_eq!(forget_attributes(31), vec![("minecraft:armor".to_string(), 2.0)]);
        assert!(forget_attributes(31).is_empty());
    }

    #[test]
    fn a_player_is_ticked_while_armor_is_worn_and_once_after() {
        assert!(!tick_wanted(32, false));
        assert!(tick_wanted(32, true));
        assert!(tick_wanted(32, false), "one more tick to let go of what the armor kept");
        assert!(!tick_wanted(32, false));
    }

    #[test]
    fn after_death_reaches_every_callback_with_the_killer() {
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = seen.clone();
        ServerLivingEntityEvents::AFTER_DEATH.register(move |d| log.lock().unwrap().push((d.entity, d.attacker)));
        after_death(&LivingDamage { entity: 7, kind: "minecraft:zombie".into(), damage_type: "minecraft:player_attack".into(), attacker: 2, direct: -1, amount: 0.0 });
        assert_eq!(*seen.lock().unwrap(), [(7, 2)]);
    }

    #[test]
    fn any_veto_cancels_the_damage() {
        ServerLivingEntityEvents::ALLOW_DAMAGE.register(|d| d.amount < 100.0);
        let hit = |amount| LivingDamage { entity: 1, kind: "minecraft:zombie".into(), damage_type: "minecraft:generic".into(), attacker: -1, direct: -1, amount };
        assert!(allow_damage(&hit(3.0)));
        assert!(!allow_damage(&hit(500.0)));
    }
}
