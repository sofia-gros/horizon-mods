//! LifestealModule, from SlimeKnights (MIT; see LICENSE): a tool that heals its wielder by a share of the damage it deals. Ported: the melee half
//! for a player (`afterMeleeHit`: a critical hit of a fully charged swing heals `percentage` of the damage dealt, unless the mob hit is of
//! the necrotic blacklist or fails the `attacker`/`target` conditions or the modifier level range). Not ported, each for a reason PlutoLib
//! cannot answer yet: the durability the heal costs the tool (`durability_usage`: no action wears a held stack, and the visible stack is only
//! rewritten by the item-wear event), the heal sound (no sound action), the projectile half (a projectile's hit event neither carries the
//! modifiers on it nor can heal its shooter), a mob that wields the tool (`onMonsterMeleeHit`; only players' hits are reported) and the
//! tooltip line (client text). The damage is the amount the game worked out for the hit before armor, cut to the mob's health; the original
//! heals by the damage actually dealt, after armor.
use crate::{melee_damage::TargetCondition, modifier_entries::Entry, thorns::Leveling};
use pluto_lib::EntityState;
use serde_json::{Map, Value};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Lifesteal { percent: Leveling, attacker: TargetCondition, target: TargetCondition }

impl Lifesteal {
    /// `None` when a condition names state this port does not hold.
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        if !fields.get("percentage").is_some_and(Value::is_object) { return Err("percentage must be an object".into()); }
        if !fields.get("durability_usage").is_some_and(Value::is_object) { return Err("durability_usage must be an object".into()); }
        let Some(attacker) = TargetCondition::decode_for("attacker", fields.get("attacker"))? else { return Ok(None); };
        let Some(target) = TargetCondition::decode_for("target", fields.get("target"))? else { return Ok(None); };
        Ok(Some(Self { percent: Leveling::decode(fields, "percentage", Leveling::ZERO)?, attacker, target }))
    }

    /// LifestealModule.onMonsterMeleeHit as afterMeleeHit calls it: the health to give the wielder for a hit that dealt `damage`, or None.
    /// `level_ok` is the modifier's level range, `blacklisted` the necrotic blacklist; the tags answer the entity type and state tags of the
    /// attacker and of the target.
    pub(crate) fn heal(&self, entry: &Entry, level_ok: bool, damage: f32, blacklisted: bool, attacker_in_tag: &dyn Fn(&pluto_lib::Identifier) -> bool, target_in_tag: &dyn Fn(&pluto_lib::Identifier) -> bool) -> Option<f32> {
        if !(damage > 0.0 && level_ok) || blacklisted || !self.attacker.matches(attacker_in_tag) || !self.target.matches(target_in_tag) { return None; }
        let percent = self.percent.compute(entry.effective_level());
        (percent > 0.0).then(|| percent * damage)
    }
}

/// ToolAttackUtil.getCriticalModifier for what PlutoLib reports of the attacker: off the ground, out of the water, not sprinting and not
/// blinded. The fall distance, climbing, riding and the full charge of the swing are not reported (the swing is taken as fully charged, and
/// a rising jump counts as the fall that makes a critical).
pub(crate) fn is_critical(attacker: &EntityState) -> bool {
    !attacker.on_ground && !attacker.in_water && !attacker.sprinting && !attacker.effects.iter().any(|effect| effect == "minecraft:blindness")
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::Identifier;
    use serde_json::json;

    fn entry(level: i32) -> Entry { Entry::new(Identifier::parse("tconstruct:necrotic").unwrap(), level) }
    fn lifesteal(value: Value) -> Option<Lifesteal> { Lifesteal::decode(value.as_object().unwrap()).unwrap() }
    fn any(_: &Identifier) -> bool { true }
    fn none(_: &Identifier) -> bool { false }
    fn necrotic() -> Lifesteal { lifesteal(json!({"durability_usage":{"each_level":1},"percentage":{"each_level":0.05}})).unwrap() }

    #[test]
    fn a_critical_hit_heals_five_percent_of_the_damage_a_level() {
        let heal = necrotic().heal(&entry(2), true, 8.0, false, &none, &none).unwrap();
        assert!((heal - 0.8).abs() < 1e-6, "10% of 8 at level 2: {heal}");
        assert_eq!(necrotic().heal(&entry(2), true, 0.0, false, &none, &none), None, "no damage, no heal");
        assert_eq!(necrotic().heal(&entry(2), false, 8.0, false, &none, &none), None, "the modifier level range holds first");
        assert_eq!(necrotic().heal(&entry(2), true, 8.0, true, &none, &none), None, "a mob of the necrotic blacklist gives nothing");
        let nothing = lifesteal(json!({"durability_usage":{"flat":1},"percentage":{}})).unwrap();
        assert_eq!(nothing.heal(&entry(1), true, 8.0, false, &none, &none), None, "a share of zero heals nothing");
    }

    #[test]
    fn the_attacker_and_target_conditions_gate_the_heal_each_on_its_own_entity() {
        let module = lifesteal(json!({"durability_usage":{"flat":1},"percentage":{"flat":0.5},"target":{"type":"mantle:tag","tag":"minecraft:undead"},"attacker":{"type":"mantle:tag","tag":"example:vampires"}})).unwrap();
        let undead = |tag: &Identifier| tag.to_string() == "minecraft:undead";
        let vampire = |tag: &Identifier| tag.to_string() == "example:vampires";
        assert_eq!(module.heal(&entry(1), true, 4.0, false, &vampire, &undead), Some(2.0));
        assert_eq!(module.heal(&entry(1), true, 4.0, false, &undead, &vampire), None, "swapped, the conditions ask the wrong entities");
        assert_eq!(module.heal(&entry(1), true, 4.0, false, &any, &none), None);
    }

    #[test]
    fn a_missing_share_or_cost_is_malformed_and_an_unreported_condition_leaves_it_unbound() {
        assert!(Lifesteal::decode(json!({"durability_usage":{"flat":1}}).as_object().unwrap()).is_err(), "the percentage is required");
        assert!(Lifesteal::decode(json!({"percentage":{"flat":1.0}}).as_object().unwrap()).is_err(), "the durability usage is required");
        assert!(lifesteal(json!({"durability_usage":{},"percentage":{"flat":1.0},"attacker":"mantle:raining_at"})).is_none());
        assert!(lifesteal(json!({"durability_usage":{},"percentage":{"flat":1.0},"attacker":"mantle:sprinting","target":"mantle:on_fire"})).is_some());
    }

    #[test]
    fn a_critical_hit_needs_the_attacker_airborne_dry_not_sprinting_and_not_blind() {
        let airborne = EntityState { on_ground: false, ..EntityState::default() };
        assert!(is_critical(&airborne));
        assert!(!is_critical(&EntityState { on_ground: true, ..EntityState::default() }));
        assert!(!is_critical(&EntityState { in_water: true, ..airborne.clone() }));
        assert!(!is_critical(&EntityState { sprinting: true, ..airborne.clone() }));
        assert!(!is_critical(&EntityState { effects: vec!["minecraft:blindness".into()], ..airborne.clone() }));
        assert!(is_critical(&EntityState { effects: vec!["minecraft:speed".into()], ..airborne }));
    }
}
