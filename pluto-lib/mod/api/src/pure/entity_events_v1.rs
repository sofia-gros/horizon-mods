//! Counterpart of Fabric's `fabric-entity-events-v1`: entity load and unload, and living-entity damage.
//!
//! Entities are identified by their numeric id (Fabric passes entity objects); PlutoLib grows the data as modules need it.

use super::base::Event;

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

static ALLOW_DAMAGE: Event<LivingDamage, bool> = Event::allow_all();
static AFTER_DEATH: Event<LivingDamage, ()> = Event::void();

/// Fabric: `ServerLivingEntityEvents`.
pub struct ServerLivingEntityEvents;

impl ServerLivingEntityEvents {
    /// Fabric: `ALLOW_DAMAGE`: return `false` to cancel the damage.
    pub const ALLOW_DAMAGE: &'static Event<LivingDamage, bool> = &ALLOW_DAMAGE;
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
