//! Counterpart of Fabric's `fabric-entity-events-v1`: entity load and unload, and living-entity damage.
//!
//! Entities are identified by their numeric id (Fabric passes entity objects); PlutoLib grows the data as modules need it.

use std::cell::{Cell, RefCell};
use super::base::NbtCompound;

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
    /// New custom data for the worn stack, which a callback may set; written to the stack in the player's main inventory that
    /// carried the data in `stack` (a worn armor piece is not rewritten).
    pub custom_data: RefCell<Option<NbtCompound>>,
}

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
    /// NeoForge: `LivingKnockBackEvent`. Change the strength or direction with `set`, or set `cancel`. Server only.
    pub const MODIFY_KNOCKBACK: &'static Event<KnockbackModification, ()> = &MODIFY_KNOCKBACK;
    /// A player's item losing durability: change the amount with `amount.set` or set `cancel`. Server only.
    pub const MODIFY_ITEM_DAMAGE: &'static Event<ItemDamage, ()> = &MODIFY_ITEM_DAMAGE;
}

/// The knockback after the callbacks as (strength, x, z), or None when a callback cancelled it.
pub(crate) fn modify_knockback(modification: &KnockbackModification) -> Option<(f64, f64, f64)> {
    MODIFY_KNOCKBACK.invoke(modification);
    if modification.cancel.get() { None } else { Some((modification.strength.get(), modification.x.get(), modification.z.get())) }
}

/// The wear after the callbacks, with the new custom data a callback set, or None when a callback cancelled it.
pub(crate) fn modify_item_damage(wear: &ItemDamage) -> Option<(i32, Option<NbtCompound>)> {
    MODIFY_ITEM_DAMAGE.invoke(wear);
    if wear.cancel.get() { None } else { Some((wear.amount.get(), wear.custom_data.borrow_mut().take())) }
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
    let modification = DamageModification { damage: damage.clone(), amount: Cell::new(damage.amount), attacker_stack, armor };
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
        let wear = |item: &str, amount| ItemDamage { player: 1, name: "wearer".into(), item: item.into(), stack: None, amount: Cell::new(amount), cancel: Cell::new(false), custom_data: RefCell::new(None) };
        assert_eq!(modify_item_damage(&wear("example:pickaxe", 3)), Some((2, None)));
        assert_eq!(modify_item_damage(&wear("example:shield", 3)), None);
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
