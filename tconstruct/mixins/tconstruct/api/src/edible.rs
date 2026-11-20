//! The edible modifier family (EdibleModule and the EdibleEffectHook modules, SlimeKnights, MIT; see LICENSE): a tool whose hunger stat is above 0
//! can be eaten with a right click. Eating lasts the tool's eat duration; when it is done the player is fed `round(hunger)` and the saturation
//! the conditional stats make of the stats, the eating sounds play, and then each modifier's edible effects run in modifier order: the tool is
//! worn (`edible_consume_durability`), a random curable effect is cured (`edible_cure_random_effect`) or one effect is removed
//! (`edible_remove_effect`).
//!
//! Not carried over: the armor counter-attack of EdibleModule (`onAttacked`, which eats a worn piece when its wearer is hit), because the game
//! gives no way to wear or change a worn armor piece from that event, and the representative item (`edible_representative_item`), whose only
//! effects are item particles and eating sounds while the use runs and the Diet integration, which this port does not include.
use crate::modifier_entries::Entry;
use pluto_lib::Identifier;
use serde_json::{Map, Value};

/// What eating a tool does after the player is fed: one effect of one modifier.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Effect {
    /// EdibleConsumeDurabilityModule: the tool takes this much direct damage.
    ConsumeDurability(i32),
    /// EdibleCureRandomEffectModule: one of the player's curable effects is removed.
    CureRandomEffect,
    /// EdibleRemoveEffectModule: this effect is removed.
    RemoveEffect(Identifier),
}

/// The leveling value of `durability_usage` (LevelingInt): `flat` plus `each_level` for each level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DurabilityUsage { flat: i32, each: i32 }

impl DurabilityUsage {
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        let Some(Value::Object(usage)) = fields.get("durability_usage") else { return Err("durability_usage must be a leveling value".into()) };
        let int = |name: &str| match usage.get(name) {
            None | Some(Value::Null) => Ok(0),
            Some(value) => value.as_f64().map(|value| value as i32).ok_or_else(|| format!("{name} must be a number")),
        };
        Ok(Self { flat: int("flat")?, each: int("each_level")? })
    }

    /// LevelingInt.compute(entry): `(int)(flat + eachLevel * effectiveLevel)`.
    pub(crate) fn compute(self, entry: &Entry) -> i32 { (self.flat as f32 + self.each as f32 * entry.effective_level()) as i32 }
}

/// A module that names no holder predicate: a holder condition needs the eater's state, which the finish event does not carry.
pub(crate) fn has_holder(fields: &Map<String, Value>) -> bool { fields.get("holder").is_some_and(|value| !value.is_null()) }

/// The modifier's durability cost for an eaten tool, when it is above 0.
pub(crate) fn wear(usage: DurabilityUsage, entry: &Entry) -> Option<Effect> {
    let damage = usage.compute(entry);
    (damage > 0).then_some(Effect::ConsumeDurability(damage))
}

/// EdibleModule.onToolUse: eating starts only while the tool is whole and its hunger stat, as an integer, is above 0.
pub(crate) fn can_start(hunger: f32, broken: bool) -> bool { !broken && hunger as i32 > 0 }

/// EdibleModule.getUseDuration: the eat duration stat as an integer.
pub(crate) fn use_ticks(eat_duration: f32) -> i32 { eat_duration as i32 }

/// Player.canEat(false): a creative player or one who is not full.
pub(crate) fn can_eat(creative: bool, food_level: i32) -> bool { creative || food_level < 20 }

/// Math.round of the conditional hunger stat: the nutrition eaten.
pub(crate) fn nutrition(hunger: f32) -> i32 { (hunger + 0.5).floor() as i32 }

/// The two sounds of eating, as (sound, volume, pitch), from the random values the original draws (`rolls` give uniform values in [0, 1)):
/// the bite at volume 1 with a pitch of `1 + (a - b) * 0.4`, then the burp at volume 0.5 with a pitch of `c * 0.1 + 0.9`.
pub(crate) fn sounds(rolls: &mut dyn FnMut() -> f32) -> [(&'static str, f32, f32); 2] {
    let bite = 1.0 + (rolls() - rolls()) * 0.4;
    let burp = rolls() * 0.1 + 0.9;
    [("minecraft:entity.generic.eat", 1.0, bite), ("minecraft:entity.player.burp", 0.5, burp)]
}

/// CheeseItem.removeRandomEffect: the effect to remove, chosen uniformly from the player's effects with `roll` in [0, 1). Every effect the game
/// has is curable by milk, so none is filtered out.
pub(crate) fn pick_effect(effects: &[String], roll: f32) -> Option<&String> {
    if effects.is_empty() { return None; }
    effects.get(((roll * effects.len() as f32) as usize).min(effects.len() - 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn entry(level: i32) -> Entry { Entry::new(Identifier::parse("tconstruct:tasty").unwrap(), level) }

    #[test]
    fn durability_usage_counts_the_flat_part_and_each_level() {
        let usage = DurabilityUsage::decode(json!({"durability_usage":{"each_level":5,"flat":5}}).as_object().unwrap()).unwrap();
        assert_eq!((usage.compute(&entry(1)), usage.compute(&entry(3))), (10, 20));
        assert_eq!(wear(usage, &entry(1)), Some(Effect::ConsumeDurability(10)));
        let free = DurabilityUsage::decode(json!({"durability_usage":{}}).as_object().unwrap()).unwrap();
        assert_eq!(wear(free, &entry(2)), None, "a cost of 0 wears nothing");
        assert!(DurabilityUsage::decode(json!({}).as_object().unwrap()).is_err(), "the usage is required");
        assert!(DurabilityUsage::decode(json!({"durability_usage":{"flat":"many"}}).as_object().unwrap()).is_err());
    }

    #[test]
    fn eating_starts_on_a_whole_tool_with_a_hunger_stat_and_feeds_the_rounded_stat() {
        assert!(can_start(1.0, false));
        assert!(!can_start(0.9, false), "the stat is read as an integer: below 1 is 0");
        assert!(!can_start(4.0, true), "a broken tool is never eaten");
        assert_eq!((use_ticks(16.0), use_ticks(23.9)), (16, 23));
        assert_eq!((nutrition(2.4), nutrition(2.5), nutrition(0.4)), (2, 3, 0), "Math.round rounds halves up");
        assert!(can_eat(false, 19) && !can_eat(false, 20));
        assert!(can_eat(true, 20), "creative players eat when full");
    }

    #[test]
    fn the_eating_sounds_draw_their_pitch_from_the_original_formula() {
        let mut values = [0.75_f32, 0.25, 0.5].into_iter();
        let [bite, burp] = sounds(&mut || values.next().unwrap());
        assert_eq!(bite, ("minecraft:entity.generic.eat", 1.0, 1.0 + (0.75 - 0.25) * 0.4));
        assert_eq!(burp, ("minecraft:entity.player.burp", 0.5, 0.5 * 0.1 + 0.9));
    }

    #[test]
    fn a_random_effect_is_picked_uniformly_and_nothing_is_picked_from_no_effects() {
        let effects = ["minecraft:poison".to_owned(), "minecraft:slowness".to_owned(), "minecraft:speed".to_owned()];
        assert_eq!(pick_effect(&effects, 0.0).map(String::as_str), Some("minecraft:poison"));
        assert_eq!(pick_effect(&effects, 0.5).map(String::as_str), Some("minecraft:slowness"));
        assert_eq!(pick_effect(&effects, 0.999).map(String::as_str), Some("minecraft:speed"));
        assert_eq!(pick_effect(&effects, 1.0).map(String::as_str), Some("minecraft:speed"), "a roll of exactly 1 stays in range");
        assert_eq!(pick_effect(&[], 0.3), None);
    }

    #[test]
    fn a_holder_condition_is_recognised_only_when_given() {
        assert!(!has_holder(json!({}).as_object().unwrap()));
        assert!(!has_holder(json!({"holder":null}).as_object().unwrap()));
        assert!(has_holder(json!({"holder":{"type":"mantle:sneaking"}}).as_object().unwrap()));
    }
}
