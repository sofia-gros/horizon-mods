// Shared source fluid-effect selection for tool modifiers.
// Unsupported effects remain unknown and do not authorize a transaction.
use std::collections::{BTreeMap, HashMap, HashSet};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MobEffect {
    pub id: String,
    pub time: i32,
    pub amplifier: i32,
    pub action: TimeAction,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TimeAction { Add, Set }
#[derive(Clone, Debug, PartialEq)]
pub(super) enum EntityEffect {
    Mob(MobEffect),
    Remove(String),
    Hunger { hunger: i32, saturation: f32, always: bool },
    Push { flat: f32, each_level: f32 },
}
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Effects {
    pub amount: u32,
    pub entity: Vec<EntityEffect>,
}

/// Compute the source duration and consumed strength for a living target.
/// `existing` is the active duration and amplifier for the same effect.
pub(super) fn mob_duration(effect: &MobEffect, strength: f32, level: f32, existing: Option<(i32, i32)>) -> Option<(i32, f32)> {
    let infinite = effect.time == -1;
    if infinite && strength < 1.0 { return None; }
    let extra = if infinite { -1 } else { (effect.time as f32 * strength) as i32 };
    if !infinite && extra <= 0 { return None; }
    let result = if infinite { (-1, 1.0) } else { match existing {
        Some((_, amplifier)) if amplifier > effect.amplifier => return None,
        Some((duration, amplifier)) if amplifier == effect.amplifier && duration < 0 => return None,
        Some((duration, amplifier)) if amplifier == effect.amplifier => match effect.action {
            TimeAction::Add => (duration.checked_add(extra)?, strength),
            TimeAction::Set => {
                let previous = duration as f32 / effect.time as f32;
                let effective = (strength + previous).min(level);
                if effective < previous { return None; }
                ((effect.time as f32 * effective) as i32, effective - previous)
            }
        },
        _ => (extra, strength),
    }};
    (result.0 != 0).then_some(result)
}

/// Choose the first matching source definition independently of effect domain.
pub(super) fn select(
    layers: &BTreeMap<String, Vec<Vec<u8>>>, fluid: &str,
    fluid_tags: &HashMap<String, HashSet<String>>,
    conditions: &impl Fn(&[Value]) -> Option<bool>,
) -> Option<Option<Value>> {
    for (id, layers) in layers {
        if !id.contains(":tinkering/fluid_effects/") { continue; }
        let value: Value = serde_json::from_slice(layers.last()?).ok()?;
        let spec = value.get("fluid")?.as_object()?;
        let matched = match (spec.get("fluid"), spec.get("tag")) {
            (Some(id), None) => id.as_str()? == fluid,
            (None, Some(tag)) => fluid_tags.get(tag.as_str()?).is_some_and(|items| items.contains(fluid)),
            _ => return None,
        };
        if !matched { continue; }
        if let Some(raw) = value.get("conditions") {
            let condition = conditions(raw.as_array()?)?;
            if !condition { continue; }
        }
        return Some(Some(value));
    }
    Some(None)
}

/// Distinguish unavailable/unsupported data from no matching recipe; callers
/// must not skip an unsupported first matching effect and consume fluid later.
pub(super) fn resolve(
    layers: &BTreeMap<String, Vec<Vec<u8>>>, fluid: &str,
    fluid_tags: &HashMap<String, HashSet<String>>,
    conditions: &impl Fn(&[Value]) -> Option<bool>,
) -> Option<Option<Effects>> {
    let Some(value) = select(layers, fluid, fluid_tags, conditions)? else { return Some(None); };
    parse_entity(&value).map(Some)
}

pub(super) fn parse_entity(value: &Value) -> Option<Effects> {
    let spec = value.get("fluid")?.as_object()?;
        let amount = u32::try_from(spec.get("amount")?.as_u64()?).ok().filter(|n| *n > 0)?;
        let effects = match value.get("entity_effects") {
            None => Vec::new(),
            Some(Value::Array(effects)) => effects.iter().map(|effect| {
                let obj = effect.as_object()?;
                match obj.get("type")?.as_str()? {
                    "tconstruct:mob_effect" => {
                        let action = match obj.get("action")?.as_str()? { "add" => TimeAction::Add, "set" => TimeAction::Set, _ => return None };
                        if obj.keys().any(|k| !matches!(k.as_str(), "type" | "action" | "effect" | "time" | "level")) { return None; }
                        let time = i32::try_from(obj.get("time").map_or(Some(-1), Value::as_i64)?).ok().filter(|n| *n > 0 || *n == -1)?;
                        let level = i32::try_from(obj.get("level").map_or(Some(1), Value::as_i64)?).ok().filter(|n| (1..=256).contains(n))?;
                        Some(EntityEffect::Mob(MobEffect { id: obj.get("effect")?.as_str()?.to_owned(), time, amplifier: level - 1, action }))
                    }
                    "tconstruct:remove_effect" => {
                        if obj.keys().any(|k| !matches!(k.as_str(), "type" | "effect")) { return None; }
                        Some(EntityEffect::Remove(obj.get("effect")?.as_str()?.to_owned()))
                    }
                    "tconstruct:push_entity" => {
                        if obj.keys().any(|key| !matches!(key.as_str(), "type" | "flat" | "each_level")) { return None; }
                        let flat = obj.get("flat").map_or(Some(0.0), Value::as_f64)? as f32;
                        let each_level = obj.get("each_level").map_or(Some(0.0), Value::as_f64)? as f32;
                        if !flat.is_finite() || !each_level.is_finite() { return None; }
                        Some(EntityEffect::Push { flat, each_level })
                    }
                    "tconstruct:restore_hunger" => {
                        if obj.keys().any(|k| !matches!(k.as_str(), "type" | "hunger" | "saturation" | "can_always_eat" | "representative_item")) { return None; }
                        let hunger = i32::try_from(obj.get("hunger")?.as_i64()?).ok().filter(|n| *n > 0)?;
                        let saturation = obj.get("saturation")?.as_f64()? as f32;
                        if !saturation.is_finite() || saturation < 0.0 { return None; }
                        let always = match obj.get("can_always_eat") { None => false, Some(v) => v.as_bool()? };
                        if obj.get("representative_item").is_some_and(|item| !item.is_string()) { return None; }
                        Some(EntityEffect::Hunger { hunger, saturation, always })
                    }
                    _ => None,
                }
            }).collect::<Option<Vec<_>>>()?,
            _ => return None,
        };
    Some(Effects { amount, entity: effects })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn honey_preserves_both_entity_actions_in_source_order() {
        let raw = br#"{"fluid":{"tag":"forge:honey","amount":50},"entity_effects":[{"type":"tconstruct:restore_hunger","hunger":1,"saturation":0.12,"can_always_eat":false,"representative_item":"minecraft:honey_bottle"},{"type":"tconstruct:remove_effect","effect":"minecraft:poison"}]}"#.to_vec();
        let layers = BTreeMap::from([("tconstruct:tinkering/fluid_effects/honey".into(), vec![raw])]);
        let tags = HashMap::from([("forge:honey".into(), HashSet::from(["tconstruct:honey".into()]))]);
        let effect = resolve(&layers, "tconstruct:honey", &tags, &|_| Some(true)).unwrap().unwrap();
        assert_eq!(effect.amount, 50);
        assert!(matches!(&effect.entity[0], EntityEffect::Hunger { hunger: 1, always: false, .. }));
        assert!(matches!(&effect.entity[1], EntityEffect::Remove(id) if id == "minecraft:poison"));
    }

    #[test]
    fn first_match_is_not_skipped_when_its_effect_is_unsupported() {
        let venom = br#"{"fluid":{"tag":"tconstruct:venom","amount":50},"entity_effects":[{"type":"tconstruct:mob_effect","action":"add","effect":"minecraft:poison","level":1,"time":100}]}"#.to_vec();
        let unsupported = br#"{"fluid":{"tag":"tconstruct:venom","amount":30},"entity_effects":[{"type":"example:unknown"}]}"#.to_vec();
        let tags = HashMap::from([("tconstruct:venom".into(), HashSet::from(["tconstruct:venom".into()]))]);
        let mut files = BTreeMap::from([("tconstruct:tinkering/fluid_effects/b".into(), vec![venom])]);
        let result = resolve(&files, "tconstruct:venom", &tags, &|_| Some(true)).unwrap().unwrap();
        assert_eq!(result.amount, 50);
        assert!(matches!(&result.entity[0], EntityEffect::Mob(effect) if effect.time == 100));
        files.insert("tconstruct:tinkering/fluid_effects/a".into(), vec![unsupported]);
        assert_eq!(resolve(&files, "tconstruct:venom", &tags, &|_| Some(true)), None);
        assert_eq!(resolve(&files, "minecraft:water", &tags, &|_| Some(true)), Some(None));
    }
}
