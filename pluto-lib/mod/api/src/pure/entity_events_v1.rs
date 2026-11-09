//! Counterpart of Fabric's `fabric-entity-events-v1`: entity load and unload, and living-entity damage.
//!
//! Entities are identified by their numeric id (Fabric passes entity objects); PlutoLib grows the data as modules need it.

use std::cell::{Cell, RefCell};

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
}

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
    let modification = DamageModification { damage: damage.clone(), amount: Cell::new(damage.amount), attacker_stack };
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
