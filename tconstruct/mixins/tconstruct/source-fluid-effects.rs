//! Shared source fluid-effect selection for tool modifiers.
//! Unsupported effects remain unknown and do not authorize a transaction.
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Effects {
    pub amount: u32,
    pub entity: Vec<MobEffect>,
}

/// Distinguish unavailable/unsupported data from no matching recipe; callers
/// must not skip an unsupported first matching effect and consume fluid later.
pub(super) fn resolve(
    layers: &BTreeMap<String, Vec<Vec<u8>>>, fluid: &str,
    fluid_tags: &HashMap<String, HashSet<String>>,
    conditions: &impl Fn(&[Value]) -> Option<bool>,
) -> Option<Option<Effects>> {
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
        let amount = u32::try_from(spec.get("amount")?.as_u64()?).ok().filter(|n| *n > 0)?;
        let effects = match value.get("entity_effects") {
            None => Vec::new(),
            Some(Value::Array(effects)) => effects.iter().map(|effect| {
                let obj = effect.as_object()?;
                if obj.get("type")?.as_str()? != "tconstruct:mob_effect" { return None; }
                let action = match obj.get("action")?.as_str()? { "add" => TimeAction::Add, "set" => TimeAction::Set, _ => return None };
                if obj.keys().any(|k| !matches!(k.as_str(), "type" | "action" | "effect" | "time" | "level")) { return None; }
                let time = i32::try_from(obj.get("time").map_or(Some(-1), Value::as_i64)?).ok().filter(|n| *n > 0 || *n == -1)?;
                let level = i32::try_from(obj.get("level").map_or(Some(1), Value::as_i64)?).ok().filter(|n| (1..=256).contains(n))?;
                Some(MobEffect { id: obj.get("effect")?.as_str()?.to_owned(), time, amplifier: level - 1, action })
            }).collect::<Option<Vec<_>>>()?,
            _ => return None,
        };
        return Some(Some(Effects { amount, entity: effects }));
    }
    Some(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn first_match_is_not_skipped_when_its_effect_is_unsupported() {
        let venom = br#"{"fluid":{"tag":"tconstruct:venom","amount":50},"entity_effects":[{"type":"tconstruct:mob_effect","action":"add","effect":"minecraft:poison","level":1,"time":100}]}"#.to_vec();
        let unsupported = br#"{"fluid":{"tag":"tconstruct:venom","amount":30},"entity_effects":[{"type":"example:unknown"}]}"#.to_vec();
        let tags = HashMap::from([("tconstruct:venom".into(), HashSet::from(["tconstruct:venom".into()]))]);
        let mut files = BTreeMap::from([("tconstruct:tinkering/fluid_effects/b".into(), vec![venom])]);
        let result = resolve(&files, "tconstruct:venom", &tags, &|_| Some(true)).unwrap().unwrap();
        assert_eq!(result.amount, 50);
        assert_eq!(result.entity[0].time, 100);
        files.insert("tconstruct:tinkering/fluid_effects/a".into(), vec![unsupported]);
        assert_eq!(resolve(&files, "tconstruct:venom", &tags, &|_| Some(true)), None);
        assert_eq!(resolve(&files, "minecraft:water", &tags, &|_| Some(true)), Some(None));
    }
}
