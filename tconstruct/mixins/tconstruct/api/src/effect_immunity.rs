//! EffectImmunityModule, from SlimeKnights (MIT; see LICENSE): armor that makes its wearer immune to a mob effect. Each worn piece adds its
//! `max_level` (a flat part plus a part for each level of the modifier, 255 when not given) to the wearer's count for the effect, and an
//! effect is refused while that count is above its amplifier (`ModifierEvents.isPotionApplicable`). The game gives the mods no call that
//! refuses an effect, so the effect is taken off the wearer at their next tick instead.
use crate::modifier_entries::Entry;
use pluto_lib::Identifier;
use serde_json::{Map, Value};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EffectImmunity { effect: Identifier, flat: i32, each: i32 }

impl EffectImmunity {
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Self, String> {
        let effect = Identifier::parse(fields.get("effect").and_then(Value::as_str).ok_or("effect missing")?).map_err(|e| e.to_string())?;
        let (flat, each) = match fields.get("max_level").filter(|value| !value.is_null()) {
            None => (255, 0),
            Some(Value::Object(level)) => {
                let int = |name: &str| level.get(name).and_then(Value::as_f64).map_or(0, |value| value as i32);
                (int("flat"), int("each_level"))
            }
            Some(_) => return Err("max_level must be an object".into()),
        };
        Ok(Self { effect, flat, each })
    }

    /// The effect and the count this piece adds to the wearer's, when the modifier's conditions hold.
    pub(crate) fn immunity(&self, entry: &Entry, condition: bool) -> Option<(Identifier, i32)> {
        condition.then(|| (self.effect.clone(), self.flat.wrapping_add(self.each.wrapping_mul(entry.level))))
    }
}

/// Whether an effect of `amplifier` is refused given the wearer's count for it (the effect's level is the amplifier plus one).
pub(crate) fn refuses(count: i32, amplifier: i32) -> bool { count > amplifier }

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn immunity(value: Value) -> EffectImmunity { EffectImmunity::decode(value.as_object().unwrap()).unwrap() }
    fn entry(level: i32) -> Entry { Entry::new(Identifier::parse("tconstruct:shell_gut").unwrap(), level) }

    #[test]
    fn the_count_grows_with_the_level_and_defaults_to_every_level() {
        let slow = immunity(json!({"effect":"minecraft:slowness","max_level":{"each_level":1}}));
        assert_eq!(slow.immunity(&entry(2), true), Some((Identifier::parse("minecraft:slowness").unwrap(), 2)));
        let hunger = immunity(json!({"effect":"minecraft:hunger","max_level":{"each_level":1,"flat":1}}));
        assert_eq!(hunger.immunity(&entry(2), true).unwrap().1, 3);
        let poison = immunity(json!({"effect":"minecraft:poison"}));
        assert_eq!(poison.immunity(&entry(1), true).unwrap().1, 255, "no max_level is any level");
        assert!(poison.immunity(&entry(1), false).is_none(), "the modifier condition holds first");
    }

    #[test]
    fn an_effect_is_refused_while_the_count_is_above_its_amplifier() {
        assert!(refuses(1, 0), "level I is refused with a count of 1");
        assert!(!refuses(1, 1), "level II needs a count of 2");
        assert!(refuses(255, 254));
        assert!(!refuses(0, 0));
    }
}
