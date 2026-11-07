// Shared source fluid-effect selection for tool modifiers.
// Unsupported effects remain unknown and do not authorize a transaction.
use std::collections::{BTreeMap, HashMap, HashSet};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct MobEffect {
    pub id: String,
    pub time: i32,
    pub amplifier: i32,
    pub action: TimeAction,
    pub time_scale: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TimeAction { Add, Set }
#[derive(Clone, Debug, PartialEq)]
pub(super) enum EntityEffect {
    Mob(MobEffect),
    Remove(String),
    Hunger { hunger: i32, saturation: f32, always: bool },
    Push { flat: f32, each_level: f32 },
    CureMilk,
    Calcified,
    WaterSensitiveDamage,
    Extinguish,
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
    let base = effect.time as f32 * effect.time_scale;
    if !infinite && (!base.is_finite() || base <= 0.0) { return None; }
    let extra = if infinite { -1 } else { (base * strength) as i32 };
    if !infinite && extra <= 0 { return None; }
    let result = if infinite { (-1, 1.0) } else { match existing {
        Some((_, amplifier)) if amplifier > effect.amplifier => return None,
        Some((duration, amplifier)) if amplifier == effect.amplifier && duration < 0 => return None,
        Some((duration, amplifier)) if amplifier == effect.amplifier => match effect.action {
            TimeAction::Add => (duration.checked_add(extra)?, strength),
            TimeAction::Set => {
                let previous = duration as f32 / base;
                let effective = (strength + previous).min(level);
                if effective < previous { return None; }
                ((base * effective) as i32, effective - previous)
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
        // FluidEffectManager applies conditions before deserializing the
        // definition. A disabled optional definition may name absent types.
        // Invalid JSON/resources are isolated by the reload listener rather
        // than poisoning every other fluid's selection.
        let Some(bytes) = layers.last() else { continue; };
        let Ok(value) = serde_json::from_slice::<Value>(bytes) else { continue; };
        if !value.is_object() { continue; }
        let condition = if let Some(raw) = value.get("conditions") {
            let Some(raw) = raw.as_array() else { continue; };
            conditions(raw)
        } else { Some(true) };
        if condition == Some(false) { continue; }
        let Some(spec) = value.get("fluid") else { continue; };
        // Complex/unknown ingredients remain unsupported, not silently absent.
        let spec = spec.as_object()?;
        let matched = match (spec.get("fluid"), spec.get("tag")) {
            (Some(id), None) => {
                let Some(id) = id.as_str() else { continue; };
                id == fluid
            },
            (None, Some(tag)) => {
                let Some(tag) = tag.as_str() else { continue; };
                fluid_tags.get(tag).is_some_and(|items| items.contains(fluid))
            },
            _ => return None,
        };
        if !matched { continue; }
        // Unknown source condition types remain unresolved for matching
        // fluids; an unrelated definition must not disable this fluid.
        condition?;
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
                        Some(EntityEffect::Mob(MobEffect { id: obj.get("effect")?.as_str()?.to_owned(), time, amplifier: level - 1, action, time_scale: 1.0 }))
                    }
                    "tconstruct:remove_effect" => {
                        if obj.keys().any(|k| !matches!(k.as_str(), "type" | "effect")) { return None; }
                        Some(EntityEffect::Remove(obj.get("effect")?.as_str()?.to_owned()))
                    }
                    "tconstruct:cure_effects" if obj.len() == 2 && obj.get("item").and_then(Value::as_str) == Some("minecraft:milk_bucket") => Some(EntityEffect::CureMilk),
                    "tconstruct:calcified" if obj.len() == 1 => Some(EntityEffect::Calcified),
                    "tconstruct:extinguish" if obj.len() == 1 => Some(EntityEffect::Extinguish),
                    "tconstruct:conditional" if obj.len() == 3
                        && obj.get("target").and_then(Value::as_str) == Some("mantle:water_sensitive")
                        && effect.pointer("/effect/type").and_then(Value::as_str) == Some("tconstruct:damage")
                        && effect.pointer("/effect/damage").and_then(Value::as_f64) == Some(2.0)
                        && effect.pointer("/effect/damage_type/melee").and_then(Value::as_str) == Some("tconstruct:water_melee")
                        && effect.pointer("/effect/damage_type/ranged").and_then(Value::as_str) == Some("tconstruct:water_ranged")
                        && obj.get("effect").and_then(Value::as_object).is_some_and(|effect| effect.len() == 3)
                        && effect.pointer("/effect/damage_type").and_then(Value::as_object).is_some_and(|types| types.len() == 2)
                        => Some(EntityEffect::WaterSensitiveDamage),
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

/// Resolve the bundled potion effect using its fluid NBT. Only known,
/// non-instant vanilla base effects are supported; unknowns stay rejected.
pub(super) fn parse_tagged_entity(value: &Value, tag: Option<&horizon_world::nbt::Tag>) -> Option<Effects> {
    use horizon_world::nbt::Tag;
    let entries = value.get("entity_effects")?.as_array()?;
    if !entries.iter().any(|entry| entry.get("type").and_then(Value::as_str) == Some("tconstruct:potion")) {
        return parse_entity(value);
    }
    let tag = tag?;
    let Tag::String(id) = tag.get("Potion")? else { return None; };
    let id = id.to_string_lossy();
    if !id.starts_with("minecraft:") { return None; }
    let potion = horizon_client::pluto_potions::potion(&id)?;
    let bottle = tag.get("Bottle").and_then(|value| if let Tag::String(value) = value { Some(value.to_string_lossy().to_string()) } else { None });
    let mut chosen = None;
    for entry in entries {
        let obj = entry.as_object()?;
        if obj.get("type")?.as_str()? != "tconstruct:potion" || obj.keys().any(|key| !matches!(key.as_str(), "type" | "scale" | "nbt")) { return None; }
        let matches = if let Some(predicate) = obj.get("nbt") {
            let predicate = predicate.as_object()?;
            if predicate.len() != 1 { return None; }
            bottle.as_deref() == Some(predicate.get("Bottle")?.as_str()?)
        } else { true };
        if matches {
            let scale = obj.get("scale")?.as_f64()? as f32;
            if !scale.is_finite() || scale <= 0.0 || chosen.replace(scale).is_some() { return None; }
        }
    }
    let scale = chosen?;
    let amount = u32::try_from(value.pointer("/fluid/amount")?.as_u64()?).ok().filter(|amount| *amount > 0)?;
    let mut entity = Vec::new();
    for &(id, time, amplifier) in potion.effects {
        let effect = horizon_client::pluto_potions::effect(id)?;
        if effect.instant || horizon_client::pluto_potions::has_custom_namespace(effect.raw_id) || time <= 0 { return None; }
        entity.push(EntityEffect::Mob(MobEffect { id: id.to_owned(), time, amplifier, action: TimeAction::Set, time_scale: scale }));
    }
    Some(Effects { amount, entity })
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
    fn valid_water() -> Vec<u8> {
        br#"{"fluid":{"fluid":"minecraft:water","amount":50},"entity_effects":[]}"#.to_vec()
    }
    #[test]
    fn malformed_resources_do_not_disable_other_fluids() {
        for invalid in [b"{".to_vec(), b"null".to_vec(), b"[]".to_vec(),
            br#"{"conditions":false}"#.to_vec(), b"{}".to_vec(),
            br#"{"fluid":{"fluid":7}}"#.to_vec(), br#"{"fluid":{"tag":7}}"#.to_vec()] {
            let files = BTreeMap::from([
                ("x:tinkering/fluid_effects/a".into(), vec![invalid]),
                ("x:tinkering/fluid_effects/z".into(), vec![valid_water()]),
            ]);
            assert_eq!(resolve(&files,"minecraft:water",&HashMap::new(),&|_|Some(true)).unwrap().unwrap().amount,50);
        }
    }
    #[test]
    fn disabled_conditions_precede_unsupported_ingredients_and_effects() {
        let files = BTreeMap::from([
            ("x:tinkering/fluid_effects/a".into(),vec![br#"{"conditions":[{"type":"forge:mod_loaded","modid":"absent"}],"fluid":[],"entity_effects":[{"type":"absent:custom"}]}"#.to_vec()]),
            ("x:tinkering/fluid_effects/z".into(),vec![valid_water()]),
        ]);
        let condition = |values: &[Value]| Some(values.first().and_then(|v|v.get("modid")).and_then(Value::as_str) != Some("absent"));
        assert_eq!(resolve(&files,"minecraft:water",&HashMap::new(),&condition).unwrap().unwrap().amount,50);
        assert!(resolve(&files,"minecraft:water",&HashMap::new(),&|_|None).is_none());
    }
    #[test]
    fn empty_layers_skip_and_invalid_top_layer_never_revives_lower_pack() {
        let files = BTreeMap::from([
            ("x:tinkering/fluid_effects/a".into(),Vec::new()),
            ("x:tinkering/fluid_effects/b".into(),vec![valid_water(),b"{".to_vec()]),
        ]);
        assert_eq!(resolve(&files,"minecraft:water",&HashMap::new(),&|_|Some(true)),Some(None));
    }

    #[test]
    fn unknown_condition_only_blocks_its_matching_fluid() {
        let files = BTreeMap::from([
            ("x:tinkering/fluid_effects/a".into(), vec![br#"{"conditions":[{"type":"x:unknown"}],"fluid":{"fluid":"minecraft:lava","amount":50}}"#.to_vec()]),
            ("x:tinkering/fluid_effects/z".into(), vec![valid_water()]),
        ]);
        assert_eq!(resolve(&files,"minecraft:water",&HashMap::new(),&|_|None).unwrap().unwrap().amount,50);
        assert!(resolve(&files,"minecraft:lava",&HashMap::new(),&|_|None).is_none());
    }

}
