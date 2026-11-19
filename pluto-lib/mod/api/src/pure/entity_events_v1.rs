//! Counterpart of Fabric's `fabric-entity-events-v1`: entity load and unload, and living-entity damage.
//!
//! Entities are identified by their numeric id (Fabric passes entity objects); PlutoLib grows the data as modules need it.

use std::cell::{Cell, RefCell};
use super::block_entity_api_v1::ItemSlot;

pub use super::entity_spawn_api::{MobSpawnInfo,MobSpawnPreparation,MobSpawnCompletion,MobSpawnAction,MobSpawnCallback};
use super::base::Event;
use super::player_api::StackData;
use super::base::{Nbt, NbtCompound};

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
    /// A player is about to shear a mob (Bukkit: `PlayerShearEntityEvent`; Fabric has no counterpart). Set `cancel` to stop the shearing,
    /// or queue `actions` for the player. Server only; heard before the shears act, so a later callback may still cancel the shearing. The game
    /// reports only a guest's shearing on the owner's LAN world: the world owner's own shearing is heard by no hook.
    pub const SHEAR: &'static Event<ShearAttempt, ()> = &SHEAR;
}

/// A player about to shear a mob, heard on a server before the shearing happens.
#[derive(Clone, Debug)]
pub struct ShearAttempt {
    /// The player's uuid and name.
    pub player: u128,
    pub name: String,
    /// The mob's entity id.
    pub entity: i32,
    /// Whether the shears are in the off hand.
    pub off_hand: bool,
    /// The stack in that hand (its damage, max damage and custom data); None for an empty hand.
    pub tool: Option<StackData>,
    /// Set to true to stop the shearing.
    pub cancel: Cell<bool>,
    /// The actions to do to the player (`Attacker`), which callbacks add to; they land when the shearing is heard, even when a later
    /// callback cancels it.
    pub actions: RefCell<Vec<EntityAction>>,
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
    /// The armor the attacking player wore as of their last tick, as (slot, stack): `feet`, `legs`, `chest`, `head`, only the pieces
    /// carrying custom data. Empty when the player is not known (with several players online only the named player's armor is known).
    pub armor: Vec<(String, StackData)>,
    /// The effects to put on the mob, which callbacks add to.
    pub effects: RefCell<Vec<EffectGrant>>,
    /// The actions to do to the player (`Attacker`) or the mob (`Victim`), which callbacks add to; they land before the hit's damage.
    pub actions: RefCell<Vec<EntityAction>>,
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

/// Which party of a hit an [`EntityAction`] is done to. In a weapon hit the attacker is the player and the victim the mob; in a hit on a
/// player (`WearerHit`) the attacker is the mob and the victim the wearer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionTarget {
    Attacker,
    Victim,
}

/// How a [`EntityAction::Knockback`] picks the way the target is pushed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnockbackDirection {
    /// Away from the other party of the hit, as the game's own damage knockback pushes.
    AwayFromOther,
    /// Against the way the target faces, as `Entity.knockback(strength, -sin(yaw), cos(yaw))` pushes.
    AgainstFacing,
}

/// Something for a callback to do to one party of a hit, which the game does after the callbacks ran (a weapon hit's before its damage
/// lands, a hit on a player's on the player's next tick). Each action is guarded: one the target cannot take (a fire-immune mob, a player
/// for the actions that only reach mobs, an entity the host cannot reach) is skipped silently.
#[derive(Clone, Debug, PartialEq)]
pub enum EntityAction {
    /// `Entity.setSecondsOnFire`: the target burns for `seconds`, when that is longer than it already burns. Skipped for fire-immune
    /// mobs; players are reached only when the host can reach them (the acting player).
    SetFire { target: ActionTarget, seconds: i32 },
    /// `LivingEntity.heal`: gives the target `amount` health, cut to its maximum. Reaches mobs and players.
    Heal { target: ActionTarget, amount: f32 },
    /// Freezes a mob: the ticks frozen become the larger of 140 (the ticks that freeze it) and what it has, plus `ticks`, and its fire goes
    /// out (`Entity.setTicksFrozen` and `clearFire`). Mobs only, and not the kinds the game makes freeze-immune.
    Freeze { target: ActionTarget, ticks: i32 },
    /// `LivingEntity.knockback`: pushes a mob with `strength` along the horizontal plane, and up when it stands on the ground. Mobs only
    /// (the host cannot push a player); the mob's own knockback resistance is not applied.
    Knockback { target: ActionTarget, strength: f64, direction: KnockbackDirection },
    /// `LivingEntity.addEffect`: puts a status effect on the target, a player or a mob. An effect id the game does not know is skipped;
    /// `amplifier` is the level less one and `duration` is in ticks, and an effect with a negative amplifier or no duration is not given.
    AddEffect { target: ActionTarget, effect: String, amplifier: i32, duration: i32 },
    /// Edits the custom data of the stack a player carries in `slot` (`ItemStack.set(DataComponents.CUSTOM_DATA, ..)`), when that stack is
    /// still of `item`. Players only: a mob target, a slot that holds another item and an edit that changes nothing are skipped. Acts on
    /// the stack as it is when the action lands, not as the callback saw it, so wear that happened in between stays.
    EditStackData { target: ActionTarget, slot: StackSlot, item: String, edits: Vec<StackDataEdit> },
}

/// Which stack of a player an [`EntityAction::EditStackData`] edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StackSlot { MainHand, OffHand, Feet, Legs, Chest, Head }

/// One change to a stack's custom data: `value` is put at `path` (compound keys from the root, the compounds on the way made as needed), or
/// the entry at `path` is removed when `value` is None.
#[derive(Clone, Debug, PartialEq)]
pub struct StackDataEdit {
    pub path: Vec<String>,
    pub value: Option<Nbt>,
}

/// Applies `edits` in order to `data`; true when any of them changed it. An edit with an empty path is skipped; one whose path runs
/// through a value that is not a compound replaces that value with a compound.
pub(crate) fn edit_custom_data(data: &mut NbtCompound, edits: &[StackDataEdit]) -> bool {
    let before = data.clone();
    for edit in edits {
        let Some((leaf, parents)) = edit.path.split_last() else { continue };
        let mut at = &mut *data;
        let mut present = true;
        for key in parents {
            if edit.value.is_none() && !matches!(at.get(key), Some(Nbt::Compound(_))) { present = false; break; }
            let entry = at.entry(key.clone()).or_insert_with(|| Nbt::Compound(NbtCompound::new()));
            if !matches!(entry, Nbt::Compound(_)) { *entry = Nbt::Compound(NbtCompound::new()); }
            let Nbt::Compound(inner) = entry else { unreachable!("made a compound just above") };
            at = inner;
        }
        if !present { continue; }
        match &edit.value { Some(value) => { at.insert(leaf.clone(), value.clone()); } None => { at.remove(leaf); } }
    }
    *data != before
}

/// The number the host calls a player's stack by (`Inventory` slot numbers): the selected hotbar slot for the main hand, 100 (feet) to 103
/// (head) for armor, -106 for the off hand.
pub(crate) fn carried_slot_number(slot: StackSlot, selected: u8) -> i32 {
    match slot {
        StackSlot::MainHand => i32::from(selected.min(8)),
        StackSlot::OffHand => -106,
        StackSlot::Feet => 100,
        StackSlot::Legs => 101,
        StackSlot::Chest => 102,
        StackSlot::Head => 103,
    }
}

/// The slot an armor slot name of `armor` lists (`feet`, `legs`, `chest`, `head`) stands for.
pub fn armor_stack_slot(name: &str) -> Option<StackSlot> {
    Some(match name { "feet" => StackSlot::Feet, "legs" => StackSlot::Legs, "chest" => StackSlot::Chest, "head" => StackSlot::Head, _ => return None })
}

/// The ticks a mob that burns for `current` ticks burns for after burning for `seconds` more, or None when that changes nothing (the new
/// fire is not longer, the seconds are not positive, or the target is fire-immune).
pub(crate) fn fire_ticks_after(current: i32, seconds: i32, fire_immune: bool) -> Option<i32> {
    let ticks = seconds.checked_mul(20)?;
    (!fire_immune && seconds > 0 && ticks > current.max(0)).then_some(ticks)
}

/// Whether a mob type can be frozen: the kinds of the game's `freeze_immune_entity_types` tag cannot.
pub(crate) fn can_freeze(kind: &str) -> bool {
    !matches!(kind, "minecraft:stray" | "minecraft:polar_bear" | "minecraft:snow_golem")
}

/// The ticks frozen a mob has after being frozen for `ticks` more, when it had `current`: a mob already frozen keeps its count, one below
/// the 140 ticks that freeze it starts from them. None for ticks that are not positive.
pub(crate) fn frozen_ticks_after(current: i32, ticks: i32) -> Option<i32> {
    (ticks > 0).then(|| current.max(140).saturating_add(ticks))
}

/// What a knockback adds to a mob's velocity, as `LivingEntity.knockback(strength, x, z)` does it: `strength` along the horizontal plane
/// against the direction (x, z), and up by the strength (at most 0.4) when the mob stands on the ground. None for no strength or no direction.
pub(crate) fn knockback_delta(strength: f64, x: f64, z: f64, on_ground: bool) -> Option<[f64; 3]> {
    let length = x.hypot(z);
    if !(strength.is_finite() && strength > 0.0 && length.is_finite() && length >= 1.0e-5) { return None; }
    Some([-x / length * strength, if on_ground { strength.min(0.4) } else { 0.0 }, -z / length * strength])
}

/// The direction (x, z) `Entity.knockback` takes for a push against the way something faces at `yaw` degrees: (-sin, cos).
pub(crate) fn facing_direction(yaw: f32) -> (f64, f64) {
    let angle = f64::from(yaw).to_radians();
    (-angle.sin(), angle.cos())
}

/// The health a living entity has after being healed by `amount`, which the host then cuts to the maximum. None for an amount that is
/// not a positive number.
pub(crate) fn healed_health(current: f32, amount: f32) -> Option<f32> {
    (amount.is_finite() && amount > 0.0 && current.is_finite()).then(|| current + amount)
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
    /// The actions to do to the mob (`Attacker`) or the wearer (`Victim`), which callbacks add to; they land on the player's next tick.
    pub actions: RefCell<Vec<EntityAction>>,
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
    /// The actions to do to the damaged player (`Victim`), which callbacks add to; they land on the player's next tick (only for a player
    /// wearing armor with custom data, whose armor is listed in `armor`), and the other party's are dropped.
    pub actions: RefCell<Vec<EntityAction>>,
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

/// How far a mob sees a player, about to be scaled for the kind of mob that is looking (NeoForge: `LivingVisibilityEvent`). It is
/// heard on a server each time a mob works out its range for a player who wears armor carrying custom data (a mod's own armor keeps its
/// state there); the game's own factors (sneaking, invisibility, worn heads) are already in the range and a callback only scales it
/// further. Nothing is heard for a player wearing no such armor.
#[derive(Clone, Debug)]
pub struct VisibilityModification {
    /// The wearer's uuid.
    pub wearer: u128,
    /// The type id of the mob that is looking, such as `minecraft:skeleton`.
    pub observer: String,
    /// What the wearer wore as of their last tick, as (slot, stack) for `feet`, `legs`, `chest` and `head` (only pieces carrying custom data).
    pub armor: Vec<(String, StackData)>,
    /// Changes whenever what any player wears changes, so a callback may keep what it worked out from `armor` for as long as it stays
    /// the same. It is never 0.
    pub armor_revision: u64,
    /// The factor on the mob's range now (a `Cell`: change it with `set`; 1 leaves the range whole, 0.5 halves it, 0 hides the wearer);
    /// each callback sees the last one's. NeoForge's `modifyVisibility` multiplies, so a callback usually sets it to its product with the
    /// factor it wants.
    pub multiplier: Cell<f64>,
}

static MODIFY_VISIBILITY: Event<VisibilityModification, ()> = Event::void();
static SHEAR: Event<ShearAttempt, ()> = Event::void();
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
    /// NeoForge: `LivingVisibilityEvent`. How far a mob sees a player in armor carrying custom data: scale it with `multiplier.set`. Server only.
    pub const MODIFY_VISIBILITY: &'static Event<VisibilityModification, ()> = &MODIFY_VISIBILITY;
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
    let actions: Vec<(i32, i32, EntityAction)> = hit.actions.borrow_mut().drain(..).filter_map(|action| wearer.map(|wearer| (wearer, hit.attacker, action))).collect();
    if !actions.is_empty() { if let Ok(mut queue) = ENTITY_ACTIONS.lock() { queue.extend(actions); } }
    hit.effects.borrow_mut().drain(..).collect()
}

/// The actions for the parties of hits on wearers, with the wearer's entity id and the mob's.
static ENTITY_ACTIONS: std::sync::Mutex<Vec<(i32, i32, EntityAction)>> = std::sync::Mutex::new(Vec::new());

/// Takes the actions queued for the hits on the wearer `wearer`, as (mob, action); other players' stay queued.
pub(crate) fn take_wearer_actions(wearer: i32) -> Vec<(i32, EntityAction)> {
    let Ok(mut queue) = ENTITY_ACTIONS.lock() else { return Vec::new() };
    let (mine, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut *queue).into_iter().partition(|(owner, _, _)| *owner == wearer);
    *queue = rest;
    mine.into_iter().map(|(_, mob, action)| (mob, action)).collect()
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

/// What the callbacks decided about a shearing: whether it goes ahead, and the actions queued for the player.
pub(crate) fn shear(attempt: &ShearAttempt) -> (bool, Vec<EntityAction>) {
    SHEAR.invoke(attempt);
    (!attempt.cancel.get(), attempt.actions.borrow_mut().drain(..).collect())
}

/// The actions the callbacks queued on a weapon hit, taken after `weapon_hit`.
pub(crate) fn take_weapon_actions(hit: &WeaponHit) -> Vec<EntityAction> {
    hit.actions.borrow_mut().drain(..).collect()
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

/// Changes whenever the armor any player wears changes; see `VisibilityModification::armor_revision`.
static ARMOR_REVISION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// The armor the player with entity id `entity` wore as of their last tick; the game names the local player of a single-player world as
/// entity 0, which is the only wearer when one is known.
pub(crate) fn armor_worn_by(entity: i32) -> Vec<(String, StackData)> {
    let Ok(worn) = ARMOR.lock() else { return Vec::new() };
    if let Some((_, pieces)) = worn.get(&entity) { return pieces.clone(); }
    if entity == 0 && worn.len() == 1 { return worn.values().next().map(|(_, pieces)| pieces.clone()).unwrap_or_default(); }
    Vec::new()
}

/// What each player wears, by entity id, with the player's uuid so a logout can forget it.
static ARMOR: std::sync::Mutex<std::collections::BTreeMap<i32, (u128, Vec<(String, StackData)>)>> = std::sync::Mutex::new(std::collections::BTreeMap::new());

/// Records what the player (`entity`) wears; an empty list forgets it.
pub(crate) fn remember_armor(uuid: u128, entity: i32, armor: Vec<(String, StackData)>) {
    let Ok(mut worn) = ARMOR.lock() else { return };
    if armor.is_empty() {
        if worn.remove(&entity).is_some() { ARMOR_REVISION.fetch_add(1, std::sync::atomic::Ordering::Relaxed); }
    } else if worn.get(&entity).map(|(known, pieces)| (*known, pieces)) != Some((uuid, &armor)) {
        worn.insert(entity, (uuid, armor));
        ARMOR_REVISION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// The piece (`feet`, `legs`, `chest` or `head`) the player with `uuid` wore as of their last tick, when it carries custom data.
pub(crate) fn worn_piece(uuid: u128, slot: &str) -> Option<StackData> {
    let worn = ARMOR.lock().ok()?;
    worn.values().find(|(known, _)| *known == uuid)?.1.iter().find(|(name, _)| name == slot).map(|(_, stack)| stack.clone())
}

/// The factor on how far a mob of type `observer` sees the player `wearer`: 1 when the player wears no armor carrying custom data.
pub(crate) fn modify_visibility(wearer: u128, observer: &str) -> f64 {
    let Ok(worn) = ARMOR.lock() else { return 1.0 };
    let Some((_, armor)) = worn.values().find(|(known, _)| *known == wearer) else { return 1.0 };
    let armor = armor.clone();
    drop(worn);
    let modification = VisibilityModification { wearer, observer: observer.to_owned(), armor, armor_revision: ARMOR_REVISION.load(std::sync::atomic::Ordering::Relaxed), multiplier: Cell::new(1.0) };
    MODIFY_VISIBILITY.invoke(&modification);
    let factor = modification.multiplier.get();
    if factor.is_finite() { factor.max(0.0) } else { 1.0 }
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
    let modification = DamageModification { damage: damage.clone(), amount: Cell::new(damage.amount), attacker_stack, armor, attacker_state, target_state, actions: RefCell::new(Vec::new()) };
    MODIFY_DAMAGE.invoke(&modification);
    // The actions for the wearer land on their next tick, with the attacker's entity id as the other party.
    let actions: Vec<(i32, i32, EntityAction)> = modification.actions.borrow_mut().drain(..).map(|action| (damage.entity, damage.attacker, action)).collect();
    if !actions.is_empty() && !modification.armor.is_empty() { if let Ok(mut queue) = ENTITY_ACTIONS.lock() { queue.extend(actions); } }
    modification.amount.get()
}

pub(crate) fn allow_damage(damage: &LivingDamage) -> bool {
    ALLOW_DAMAGE.invoke(damage)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shearing_can_be_cancelled_and_queues_actions_for_the_player() {
        ServerEntityEvents::SHEAR.register(|attempt| {
            if attempt.entity != 8181 { return; }
            if attempt.tool.as_ref().is_some_and(|tool| tool.item == "example:cutters") {
                attempt.actions.borrow_mut().push(EntityAction::AddEffect { target: ActionTarget::Attacker, effect: "minecraft:speed".into(), amplifier: 0, duration: 40 });
            }
            if attempt.off_hand { attempt.cancel.set(true); }
        });
        let attempt = |item: Option<&str>, off_hand| ShearAttempt {
            player: 1, name: "shearer".into(), entity: 8181, off_hand,
            tool: item.map(|item| StackData { item: item.into(), count: 1, ..StackData::default() }),
            cancel: Cell::new(false), actions: RefCell::new(Vec::new()),
        };
        let (allowed, actions) = shear(&attempt(Some("example:cutters"), false));
        assert!(allowed);
        assert_eq!(actions, vec![EntityAction::AddEffect { target: ActionTarget::Attacker, effect: "minecraft:speed".into(), amplifier: 0, duration: 40 }]);
        let (allowed, actions) = shear(&attempt(Some("minecraft:shears"), true));
        assert!(!allowed && actions.is_empty(), "the off hand's shearing is cancelled and queues nothing");
    }

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
    fn visibility_scales_only_for_a_wearer_of_custom_data_armor_and_the_revision_follows_the_armor() {
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = seen.clone();
        ServerLivingEntityEvents::MODIFY_VISIBILITY.register(move |look| {
            if look.wearer != 31337 { return; }
            log.lock().unwrap().push(look.armor_revision);
            if look.observer == "minecraft:skeleton" && look.armor.iter().any(|(slot, piece)| slot == "head" && piece.item == "example:mask") {
                look.multiplier.set(look.multiplier.get() * 0.25);
            }
        });
        let piece = |item: &str| StackData { item: item.into(), count: 1, ..StackData::default() };
        assert_eq!(modify_visibility(31337, "minecraft:skeleton"), 1.0, "a player with no cached armor is not asked about");
        remember_armor(31337, 4141, vec![("head".into(), piece("example:mask"))]);
        assert_eq!(worn_piece(31337, "head").map(|stack| stack.item), Some("example:mask".to_string()));
        assert!(worn_piece(31337, "feet").is_none() && worn_piece(1, "head").is_none());
        assert_eq!(modify_visibility(31337, "minecraft:skeleton"), 0.25);
        assert_eq!(modify_visibility(31337, "minecraft:zombie"), 1.0);
        remember_armor(31337, 4141, vec![("head".into(), piece("example:mask"))]);
        assert_eq!(modify_visibility(31337, "minecraft:zombie"), 1.0);
        remember_armor(31337, 4141, vec![("head".into(), piece("example:hat"))]);
        assert_eq!(modify_visibility(31337, "minecraft:skeleton"), 1.0, "the armor changed");
        forget_armor(31337);
        assert_eq!(modify_visibility(31337, "minecraft:skeleton"), 1.0);
        let revisions = seen.lock().unwrap().clone();
        assert!(revisions.iter().all(|revision| *revision != 0));
        assert_eq!(revisions.len(), 4);
        assert_eq!(revisions[0], revisions[1], "the same armor keeps its revision");
        assert_eq!(revisions[1], revisions[2], "re-reporting identical armor keeps it too");
        assert!(revisions[3] > revisions[2], "changed armor takes a new one");
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
        let hit = |item: Option<&str>| WeaponHit { entity: 7373, kind: "minecraft:husk".into(), attacker: 1, weapon: item.map(|item| StackData { item: item.into(), count: 1, ..StackData::default() }), attacker_state: None, target_state: None, armor: Vec::new(), effects: RefCell::new(Vec::new()), actions: RefCell::new(Vec::new()) };
        assert_eq!(weapon_hit(&hit(Some("example:wand"))), vec![EffectGrant { effect: "minecraft:slowness".into(), amplifier: 3, duration: 20 }]);
        assert!(weapon_hit(&hit(Some("example:stick"))).is_empty());
        assert!(weapon_hit(&hit(None)).is_empty());
    }

    #[test]
    fn the_armor_a_weapon_hit_carries_is_the_attacking_players_as_of_their_last_tick() {
        let helmet = StackData { item: "example:helmet".into(), count: 1, ..StackData::default() };
        remember_armor(0xA11CE, 4343, vec![("head".into(), helmet.clone())]);
        assert_eq!(armor_worn_by(4343), vec![("head".to_owned(), helmet.clone())]);
        assert!(armor_worn_by(4344).is_empty(), "an unknown player wears nothing known");
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = seen.clone();
        ServerLivingEntityEvents::WEAPON_HIT.register(move |hit| if hit.entity == 4545 { log.lock().unwrap().push(hit.armor.len()); });
        let hit = WeaponHit { entity: 4545, kind: "minecraft:husk".into(), attacker: 4343, weapon: None, attacker_state: None, target_state: None, armor: armor_worn_by(4343), effects: RefCell::new(Vec::new()), actions: RefCell::new(Vec::new()) };
        assert!(weapon_hit(&hit).is_empty());
        assert_eq!(*seen.lock().unwrap(), vec![1]);
        forget_armor(0xA11CE);
        assert!(armor_worn_by(4343).is_empty(), "a player who left is forgotten");
    }

    #[test]
    fn a_hit_on_the_wearer_collects_the_effects_callbacks_put_on_the_attacker() {
        ServerLivingEntityEvents::WEARER_HIT.register(|hit| {
            if hit.attacker != 8484 { return; }
            if hit.armor.iter().any(|(slot, piece)| slot == "head" && piece.item == "example:helmet") {
                hit.effects.borrow_mut().push(EffectGrant { effect: "minecraft:weakness".into(), amplifier: 0, duration: 100 });
            }
        });
        let hit = |armor: Vec<(String, StackData)>| WearerHit { attacker: 8484, kind: "minecraft:zombie".into(), damage_type: "minecraft:mob_attack".into(), amount: 3.0, attacker_state: EntityState::default(), armor, effects: RefCell::new(Vec::new()), self_effects: RefCell::new(Vec::new()), counter_damage: RefCell::new(Vec::new()), actions: RefCell::new(Vec::new()) };
        let helmet = StackData { item: "example:helmet".into(), count: 1, ..StackData::default() };
        assert_eq!(wearer_hit(&hit(vec![("head".into(), helmet)]), Some(7)), vec![EffectGrant { effect: "minecraft:weakness".into(), amplifier: 0, duration: 100 }]);
        assert!(wearer_hit(&hit(Vec::new()), Some(7)).is_empty());
    }

    #[test]
    fn actions_on_a_hit_on_the_wearer_wait_for_the_next_tick_and_a_weapon_hit_hands_its_own_over() {
        ServerLivingEntityEvents::WEARER_HIT.register(|hit| {
            if hit.attacker == 6161 { hit.actions.borrow_mut().push(EntityAction::SetFire { target: ActionTarget::Attacker, seconds: 4 }); }
        });
        ServerLivingEntityEvents::WEAPON_HIT.register(|hit| {
            if hit.entity == 6262 { hit.actions.borrow_mut().push(EntityAction::Freeze { target: ActionTarget::Victim, ticks: 80 }); }
        });
        let hit = WearerHit { attacker: 6161, kind: "minecraft:zombie".into(), damage_type: String::new(), amount: 0.0, attacker_state: EntityState::default(), armor: Vec::new(), effects: RefCell::new(Vec::new()), self_effects: RefCell::new(Vec::new()), counter_damage: RefCell::new(Vec::new()), actions: RefCell::new(Vec::new()) };
        wearer_hit(&hit, Some(41));
        assert!(take_wearer_actions(42).is_empty(), "another player's tick takes nothing");
        assert_eq!(take_wearer_actions(41), vec![(6161, EntityAction::SetFire { target: ActionTarget::Attacker, seconds: 4 })]);
        assert!(take_wearer_actions(41).is_empty(), "the queue is emptied once taken");
        wearer_hit(&hit, None);
        assert!(take_wearer_actions(41).is_empty(), "a hit with no known wearer queues nothing");
        let weapon = WeaponHit { entity: 6262, kind: "minecraft:husk".into(), attacker: 1, weapon: None, attacker_state: None, target_state: None, armor: Vec::new(), effects: RefCell::new(Vec::new()), actions: RefCell::new(Vec::new()) };
        weapon_hit(&weapon);
        assert_eq!(take_weapon_actions(&weapon), vec![EntityAction::Freeze { target: ActionTarget::Victim, ticks: 80 }]);
        assert!(take_weapon_actions(&weapon).is_empty());
    }

    #[test]
    fn the_action_guards_follow_the_games_own_rules() {
        assert_eq!(fire_ticks_after(0, 5, false), Some(100));
        assert_eq!(fire_ticks_after(100, 5, false), None, "a fire that is not longer changes nothing");
        assert_eq!(fire_ticks_after(-1, 1, false), Some(20), "a mob that does not burn reports 0 or less");
        assert_eq!(fire_ticks_after(0, 5, true), None, "fire-immune mobs do not burn");
        assert_eq!(fire_ticks_after(0, 0, false), None);
        assert!(!can_freeze("minecraft:stray") && !can_freeze("minecraft:snow_golem") && can_freeze("minecraft:zombie"));
        assert_eq!(frozen_ticks_after(0, 80), Some(220), "a mob starts from the 140 ticks that freeze it");
        assert_eq!(frozen_ticks_after(200, 80), Some(280), "a frozen mob keeps what it has");
        assert_eq!(frozen_ticks_after(0, 0), None);
        assert_eq!(healed_health(5.0, 2.5), Some(7.5));
        assert_eq!(healed_health(5.0, 0.0), None);
        assert_eq!(healed_health(5.0, f32::NAN), None);
    }

    #[test]
    fn knockback_pushes_against_the_direction_and_lifts_a_mob_on_the_ground() {
        assert_eq!(knockback_delta(0.5, 0.0, 2.0, true), Some([0.0, 0.4_f64.min(0.5), -0.5]));
        assert_eq!(knockback_delta(0.3, 3.0, 0.0, false), Some([-0.3, 0.0, 0.0]));
        assert_eq!(knockback_delta(0.3, 3.0, 0.0, true).map(|delta| delta[1]), Some(0.3), "lifted by the strength below 0.4");
        assert!(knockback_delta(0.0, 1.0, 0.0, true).is_none() && knockback_delta(1.0, 0.0, 0.0, true).is_none());
        let (x, z) = facing_direction(0.0);
        assert!(x.abs() < 1e-12 && (z - 1.0).abs() < 1e-12, "yaw 0 faces south");
        let [dx, _, dz] = knockback_delta(1.0, x, z, false).unwrap();
        assert!(dx.abs() < 1e-12 && (dz + 1.0).abs() < 1e-12, "a mob facing south is pushed north");
        let (x, z) = facing_direction(90.0);
        assert!((x + 1.0).abs() < 1e-12 && z.abs() < 1e-9, "yaw 90 faces west");
    }

    #[test]
    fn effects_for_the_wearer_wait_for_the_next_tick() {
        ServerLivingEntityEvents::WEARER_HIT.register(|hit| {
            if hit.attacker == 9595 { hit.self_effects.borrow_mut().push(EffectGrant { effect: "minecraft:strength".into(), amplifier: 0, duration: 200 }); }
        });
        let hit = WearerHit { attacker: 9595, kind: "minecraft:zombie".into(), damage_type: "minecraft:mob_attack".into(), amount: 3.0, attacker_state: EntityState::default(), armor: Vec::new(), effects: RefCell::new(Vec::new()), self_effects: RefCell::new(Vec::new()), counter_damage: RefCell::new(Vec::new()), actions: RefCell::new(Vec::new()) };
        hit.counter_damage.borrow_mut().push(CounterDamage { damage_type: "minecraft:thorns".into(), amount: 2.0 });
        assert!(wearer_hit(&hit, Some(31)).is_empty(), "nothing goes on the attacker");
        assert!(take_counter_damage(32).is_empty() && take_self_effects(32).is_empty(), "another player's tick takes nothing");
        assert_eq!(take_counter_damage(31), vec![(9595, CounterDamage { damage_type: "minecraft:thorns".into(), amount: 2.0 })]);
        assert_eq!(take_self_effects(31), vec![EffectGrant { effect: "minecraft:strength".into(), amplifier: 0, duration: 200 }]);
        assert!(take_self_effects(31).is_empty(), "the queue is emptied once taken");
        let unattributed = WearerHit { attacker: 9595, kind: "minecraft:zombie".into(), damage_type: String::new(), amount: 0.0, attacker_state: EntityState::default(), armor: Vec::new(), effects: RefCell::new(Vec::new()), self_effects: RefCell::new(Vec::new()), counter_damage: RefCell::new(Vec::new()), actions: RefCell::new(Vec::new()) };
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
    #[test]
    fn stack_edits_set_nested_values_and_remove_them_and_report_whether_anything_changed() {
        let edit = |path: &[&str], value: Option<Nbt>| StackDataEdit { path: path.iter().map(|key| key.to_string()).collect(), value };
        let mut data = NbtCompound::new();
        assert!(edit_custom_data(&mut data, &[edit(&["persistent", "mod:bar"], Some(Nbt::Int(40)))]), "the compounds on the way are made");
        let Some(Nbt::Compound(persistent)) = data.get("persistent") else { panic!("persistent compound") };
        assert_eq!(persistent.get("mod:bar"), Some(&Nbt::Int(40)));
        assert!(!edit_custom_data(&mut data, &[edit(&["persistent", "mod:bar"], Some(Nbt::Int(40))), edit(&[], Some(Nbt::Int(1))), edit(&["absent", "key"], None)]), "the same value, no path and a missing entry change nothing");
        assert!(!data.contains_key("absent"), "removing does not make the compounds it would run through");
        assert!(edit_custom_data(&mut data, &[edit(&["persistent", "mod:bar"], Some(Nbt::Int(10))), edit(&["persistent", "mod:other"], Some(Nbt::Int(5))), edit(&["persistent", "mod:other"], None)]));
        let Some(Nbt::Compound(persistent)) = data.get("persistent") else { panic!("persistent compound") };
        assert_eq!((persistent.get("mod:bar"), persistent.contains_key("mod:other")), (Some(&Nbt::Int(10)), false));
        data.insert("flat".into(), Nbt::Int(1));
        assert!(edit_custom_data(&mut data, &[edit(&["flat", "inner"], Some(Nbt::Int(2)))]), "a value in the way becomes a compound");
        assert!(matches!(data.get("flat"), Some(Nbt::Compound(_))));
    }

    #[test]
    fn stack_slots_map_to_the_hosts_inventory_numbers() {
        assert_eq!(carried_slot_number(StackSlot::MainHand, 3), 3);
        assert_eq!(carried_slot_number(StackSlot::MainHand, 200), 8, "never past the hotbar");
        assert_eq!([StackSlot::Feet, StackSlot::Legs, StackSlot::Chest, StackSlot::Head].map(|slot| carried_slot_number(slot, 0)), [100, 101, 102, 103]);
        assert_eq!(carried_slot_number(StackSlot::OffHand, 0), -106);
        assert_eq!(armor_stack_slot("chest"), Some(StackSlot::Chest));
        assert_eq!(armor_stack_slot("body"), None);
    }

    #[test]
    fn a_damage_callbacks_actions_wait_for_the_wearers_next_tick() {
        remember_armor(77, 7777, vec![("chest".into(), StackData { item: "example:plate".into(), count: 1, ..StackData::default() })]);
        ServerLivingEntityEvents::MODIFY_DAMAGE.register(|hit| {
            if hit.damage.entity != 7777 { return; }
            hit.actions.borrow_mut().push(EntityAction::EditStackData { target: ActionTarget::Victim, slot: StackSlot::Chest, item: "example:plate".into(), edits: vec![StackDataEdit { path: vec!["bar".into()], value: Some(Nbt::Int(3)) }] });
        });
        let hit = |entity| LivingDamage { entity, kind: "minecraft:player".into(), damage_type: "minecraft:freeze".into(), attacker: -1, direct: -1, amount: 2.0 };
        modify_damage(&hit(7777));
        modify_damage(&hit(7778));
        assert!(take_wearer_actions(7778).is_empty(), "a callback that only listens for one wearer queues for that one");
        let queued = take_wearer_actions(7777);
        assert_eq!(queued.len(), 1);
        assert!(matches!(&queued[0], (-1, EntityAction::EditStackData { slot: StackSlot::Chest, .. })));
        assert!(take_wearer_actions(7777).is_empty());
        forget_armor(77);
    }
}
