//! TeleportDodgeModule from SlimeKnights (MIT; see LICENSE): a worn piece that can teleport
//! its wearer away from a hit it matches, preventing the damage entirely. The original binds
//! the modify hurt hook; this port hears it on its modify damage event, as it hears the other
//! modify hurt modules (see protection_hook). The wearer must not already have the
//! enderference effect — the module grants it for the cooldown, which is both the tell and
//! the lockout against dodging again — the defender and damage source predicates must match,
//! and the chance rolls at the module's level. CounterModule.getLevel doubles that level for
//! a piece actively blocking in its own hand slot; a worn piece never sits in a hand slot, so
//! the level here is the entry's effective level, as the port's other counter modules read
//! it. On a landed dodge the original teleports at once and only then grants the effect,
//! wears the piece and prevents the damage; the damage event this port hears the module on
//! runs with neither the entity nor the world reach open, so the consumer (protection_hook)
//! grants the effect, wears the piece and prevents the damage in the event, and queues the
//! teleport for the wearer's next tick, where it lands through TeleportHelper's own
//! destination and landing rules (slime_entities) — a queued teleport whose sixteen
//! candidates all fail moves nothing, where the original's failure left the damage standing.
//! The EnderdodgingTeleportEvent the original fires has no port analogue and no data
//! listeners, so nothing fires it.

use crate::{
    melee_damage::TargetCondition,
    modifier_entries::Entry,
    protection::{DamageSource, SourceCondition},
    wearer_modules::Leveling,
};
use pluto_lib::{EntityInfo, EntityReach, PlayerTick, World};
use serde_json::{Map, Value};
use std::{collections::BTreeSet, sync::Mutex};

/// The enderference effect (TinkerEffects.enderference): having it is what stops the dodge,
/// and a landed dodge grants it for the cooldown.
pub(crate) const ENDERFERENCE: &str = "tconstruct:enderference";

/// What a landed dodge does: the ticks of enderference it grants (the caller grants none
/// when it computes to 0 or less, as the original grants none) and the wear the piece takes,
/// the prevented damage cut to an int as ToolDamageUtil.damageAnimated is handed it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Dodge {
    pub cooldown: i32,
    pub wear: i32,
}

/// TeleportDodgeModule: on a matching hit, a chance to teleport the wearer away and prevent
/// the damage.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TeleportDodge {
    defender: TargetCondition,
    source: SourceCondition,
    chance: Leveling,
    cooldown: Leveling,
}

impl TeleportDodge {
    /// `None` for a defender condition or a damage source this port cannot evaluate.
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        let Some(defender) = TargetCondition::decode_for("target", fields.get("defender"))? else { return Ok(None); };
        let source = match fields.get("damage_source") {
            None | Some(Value::Null) => SourceCondition::Any,
            Some(value) => match SourceCondition::decode(value)? {
                Some(source) => source,
                None => return Ok(None),
            },
        };
        Ok(Some(Self {
            defender,
            source,
            chance: Leveling::decode(fields.get("chance"), "chance")?,
            cooldown: Leveling::decode(fields.get("cooldown"), "cooldown")?,
        }))
    }

    /// ModifyDamageModifierHook.modifyDamageTaken, up to the teleport: what the dodge does
    /// when the wearer — `enderference` says whether they already have the effect — takes
    /// `amount` of damage from `source`, or None when it does not fire: the effect is already
    /// on them, the predicates fail, or the chance misses. The caller performs the teleport
    /// and prevents the damage only on Some. `roll` gives a uniform value in [0, 1), drawn
    /// only when the chance is strictly between 0 and 1, as CounterModule.checkChance draws.
    pub(crate) fn attempt(&self, entry: &Entry, source: &DamageSource, enderference: bool, amount: f32, roll: &mut dyn FnMut() -> f32) -> Option<Dodge> {
        if enderference { return None; }
        if !self.defender.matches(&|tag| source.wearer_in_tag(tag)) || !self.source.matches(source) { return None; }
        let level = entry.effective_level();
        let chance = self.chance.compute(level);
        if chance <= 0.0 || (chance < 1.0 && roll() >= chance) { return None; }
        Some(Dodge { cooldown: self.cooldown.compute(level) as i32, wear: amount as i32 })
    }
}

/// The wearers (by entity id) whose dodge landed and whose teleport waits for their next
/// tick, when the entity and world reaches are open — the damage event that heard the dodge
/// runs with neither, as the ender slime's does (slime_entities).
static PENDING: Mutex<BTreeSet<i32>> = Mutex::new(BTreeSet::new());

/// Records a landed dodge's teleport for the wearer's next tick.
pub(crate) fn queue(entity: i32) {
    PENDING.lock().expect("teleport dodge queue poisoned").insert(entity);
}

/// A player's tick: a queued dodge teleports them now.
pub(crate) fn tick_player(tick: &PlayerTick) {
    if !PENDING.lock().expect("teleport dodge queue poisoned").remove(&tick.entity) { return; }
    teleport(tick.entity as u64, tick.position, &tick.dimension);
}

/// An entity's tick: the same drain for a wearer who is not a player.
pub(crate) fn tick_entity(info: &EntityInfo) {
    if !PENDING.lock().expect("teleport dodge queue poisoned").remove(&(info.id as i32)) { return; }
    let Some(origin) = EntityReach::position_of(info.id as u64) else { return; };
    teleport(info.id as u64, origin, &info.dimension);
}

/// TeleportHelper.randomNearbyTeleport: up to sixteen candidate destinations around the
/// wearer, the first one that lands moving them (slime_entities owns the destination and
/// landing rules); a wearer in a vehicle steps out of it first, as LivingEntity.randomTeleport
/// stops riding.
fn teleport(id: u64, origin: [f64; 3], dimension: &str) {
    if EntityReach::vehicle_of(id).is_some() { EntityReach::stop_riding(id); }
    let world = World::new(dimension);
    for _ in 0..16 {
        let destination = crate::slime_entities::teleport_destination(origin, dimension, crate::tool_damage_hook::random() as f64, (crate::tool_damage_hook::random() * 16.0) as i32 - 8, crate::tool_damage_hook::random() as f64);
        if crate::slime_entities::teleport_lands(&world, destination) {
            EntityReach::teleport(id, destination);
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{damage_tags::DamageTags, entity_facts::Entities};
    use pluto_lib::{EntityState, Identifier};
    use serde_json::json;

    fn fields(value: Value) -> Map<String, Value> { value.as_object().unwrap().clone() }
    fn entry(level: i32) -> Entry { Entry::new(Identifier::parse("tconstruct:enderdodging").unwrap(), level) }
    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn with_source<R>(damage_type: &str, attacker: Option<&EntityState>, run: impl FnOnce(&DamageSource) -> R) -> R {
        let tags = DamageTags::load(&[]).0;
        let damage_type = id(damage_type);
        let none = |_: &Identifier| false;
        run(&DamageSource { damage_type: &damage_type, indirect: false, tags: &tags, attacker_in_tag: &none, entities: Entities { attacker, target: None } })
    }

    /// The first module of the shipped enderdodging record: projectiles, 15% per level.
    fn projectile_module() -> TeleportDodge {
        TeleportDodge::decode(&fields(json!({"type":"tconstruct:teleport_dodge","chance":{"each_level":0.15},"cooldown":{"flat":200},"damage_source":{"type":"mantle:tag","tag":"minecraft:is_projectile"}}))).unwrap().unwrap()
    }

    /// The second module of the shipped enderdodging record: direct damage with a causing
    /// entity (has one, and is not a projectile), 7.5% per level.
    fn direct_module() -> TeleportDodge {
        TeleportDodge::decode(&fields(json!({"type":"tconstruct:teleport_dodge","chance":{"each_level":0.075},"cooldown":{"flat":200},"damage_source":{"type":"mantle:and","predicates":["mantle:has_entity",{"type":"mantle:inverted","inverted_type":"mantle:tag","tag":"minecraft:is_projectile"}]}}))).unwrap().unwrap()
    }

    #[test]
    fn both_shipped_modules_decode() {
        // Decoding is what binds the kind: both of the shipped record's modules must decode.
        projectile_module();
        direct_module();
        // The chance and the cooldown are required fields of the record.
        assert!(TeleportDodge::decode(&fields(json!({"type":"tconstruct:teleport_dodge","cooldown":{"flat":200}}))).is_err());
        assert!(TeleportDodge::decode(&fields(json!({"type":"tconstruct:teleport_dodge","chance":{"each_level":0.15}}))).is_err());
    }

    #[test]
    fn a_projectile_hit_dodges_and_spends_the_cooldown() {
        let module = projectile_module();
        // A winning roll at level 1 (chance 0.15): the dodge lands, the wearer is granted 200
        // ticks of enderference and the piece takes the prevented damage as wear — the caller
        // prevents the damage and queues the teleport on exactly this result.
        assert_eq!(with_source("minecraft:arrow", None, |source| module.attempt(&entry(1), source, false, 7.9, &mut || 0.0)), Some(Dodge { cooldown: 200, wear: 7 }));
        // A losing roll leaves the damage standing.
        assert_eq!(with_source("minecraft:arrow", None, |source| module.attempt(&entry(1), source, false, 7.9, &mut || 0.5)), None);
        // A wearer who already has enderference cannot dodge, however the roll falls.
        assert_eq!(with_source("minecraft:arrow", None, |source| module.attempt(&entry(1), source, true, 7.9, &mut || 0.0)), None);
        // Direct damage is not this module's source.
        assert_eq!(with_source("minecraft:mob_attack", None, |source| module.attempt(&entry(1), source, false, 7.9, &mut || 0.0)), None);
    }

    #[test]
    fn the_direct_module_dodges_only_a_causing_entitys_direct_hit() {
        let module = direct_module();
        let attacker = EntityState::default();
        // A mob's own hit: a causing entity, not a projectile — the dodge lands.
        assert_eq!(with_source("minecraft:mob_attack", Some(&attacker), |source| module.attempt(&entry(1), source, false, 4.0, &mut || 0.0)), Some(Dodge { cooldown: 200, wear: 4 }));
        // A projectile source is the other module's ground: the damage stands unchanged.
        assert_eq!(with_source("minecraft:arrow", Some(&attacker), |source| module.attempt(&entry(1), source, false, 4.0, &mut || 0.0)), None);
        // Damage with no causing entity (a fall) fails has_entity.
        assert_eq!(with_source("minecraft:fall", None, |source| module.attempt(&entry(1), source, false, 4.0, &mut || 0.0)), None);
    }

    #[test]
    fn a_certain_chance_draws_no_roll() {
        let module = TeleportDodge::decode(&fields(json!({"type":"tconstruct:teleport_dodge","chance":{"flat":1.0},"cooldown":{"flat":50}}))).unwrap().unwrap();
        // CounterModule.checkChance: a chance of 1 or more succeeds without drawing.
        assert_eq!(with_source("minecraft:fall", None, |source| module.attempt(&entry(1), source, false, 3.0, &mut || panic!("no roll at a certain chance"))), Some(Dodge { cooldown: 50, wear: 3 }));
    }
}
