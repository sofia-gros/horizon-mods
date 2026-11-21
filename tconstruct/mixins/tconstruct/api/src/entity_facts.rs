//! EntityVariable (SlimeKnights, MIT; see LICENSE): a number read from a living entity, over the facts PlutoLib's `EntityState` reports
//! with a hit: health, maximum health, the biome temperature at the entity, whether it is in water or in the rain, the share of its armor
//! slots filled, how many of a list of equipment slots hold an item and the level of an effect on it. A variable of another entity
//! property (an attribute other than the maximum health, height, light) is not decoded, so its module stays unbound.
use crate::melee_damage::{state_in_tag, TargetCondition};
use pluto_lib::EntityState;
use serde_json::{Map, Value};

/// Which entity of a hit a variable reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Whose { Attacker, Target }

thread_local! {
    /// The player holding the tool that is being worn down, for the formulas of tool damage modules (set around the wear by `with_holder`).
    static HOLDER: std::cell::RefCell<Option<EntityState>> = const { std::cell::RefCell::new(None) };
}

/// Runs `run` with `holder` as the holder `holder()` reports (what PlutoLib said of the player as their tool wore).
pub(crate) fn with_holder<R>(holder: Option<&EntityState>, run: impl FnOnce() -> R) -> R {
    let before = HOLDER.with(|slot| slot.replace(holder.cloned()));
    let result = run();
    HOLDER.with(|slot| *slot.borrow_mut() = before);
    result
}

/// The player holding the tool being worn down, when PlutoLib named them.
pub(crate) fn holder() -> Option<EntityState> { HOLDER.with(|slot| slot.borrow().clone()) }

/// The attacker (the holder of a tool) and the target of a hit, as PlutoLib reported them; None for one it could not report.
#[derive(Clone, Copy, Default)]
pub(crate) struct Entities<'a> { pub attacker: Option<&'a EntityState>, pub target: Option<&'a EntityState> }

/// LightLayer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LightLayer { Sky, Block }

/// EntityVariable.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum EntityFact {
    /// LivingEntity.getHealth.
    Health,
    /// The `minecraft:max_health` attribute.
    MaxHealth,
    /// The temperature of the biome containing the entity.
    BiomeTemperature,
    /// 2 when the entity is in water or has its eyes in it, 1 when rain falls on it, else 0.
    Water,
    /// LivingEntity.getArmorCoverPercentage.
    ArmorCoverage,
    /// EquipmentCountEntityVariable: how many of the named equipment slots hold an item.
    EquipmentCount(Vec<String>),
    /// The level of an effect on the entity, 0 without it.
    EffectLevel(String),
    /// EntityLightVariable: the light at the entity's block, in one layer or both (the larger). Sky light has the time of day and the weather
    /// taken off, as `Level.getSkyDarken` does on a server.
    Light(Option<LightLayer>),
    /// ConditionalEntityVariable.
    Conditional { condition: Box<TargetCondition>, if_true: Box<EntityFact>, if_false: Box<EntityFact> },
    Constant(f32),
}

impl EntityFact {
    /// A variable of the loader type `kind` (`tconstruct:health`) with its `fields`; `role` is whose state a condition inside it reads
    /// (`attacker` or `target`). None for a variable this port cannot read.
    pub fn decode(kind: &str, fields: &Map<String, Value>, role: &str) -> Result<Option<Self>, String> {
        Ok(Some(match kind {
            "tconstruct:health" => Self::Health,
            "tconstruct:biome_temperature" => Self::BiomeTemperature,
            "tconstruct:water" => Self::Water,
            "tconstruct:armor_coverage" => Self::ArmorCoverage,
            "tconstruct:equipment_count" => {
                let slots = fields.get("slots").and_then(Value::as_array).ok_or("slots must be an array")?;
                let mut names = Vec::new();
                for slot in slots { names.push(slot.as_str().ok_or("equipment slot must be a name")?.to_owned()); }
                if names.is_empty() { return Ok(None); }
                Self::EquipmentCount(names)
            }
            "tconstruct:light" => Self::Light(match fields.get("light_layer").filter(|value| !value.is_null()).map(Value::as_str) {
                None => None,
                Some(Some("sky")) => Some(LightLayer::Sky),
                Some(Some("block")) => Some(LightLayer::Block),
                Some(_) => return Ok(None),
            }),
            "tconstruct:effect_level" => {
                let effect = fields.get("effect").and_then(Value::as_str).ok_or("effect must be an effect id")?;
                let (namespace, path) = effect.split_once(':').unwrap_or(("minecraft", effect));
                Self::EffectLevel(format!("{namespace}:{path}"))
            }
            "tconstruct:attribute" => {
                let attribute = fields.get("attribute").and_then(Value::as_str).ok_or("attribute must be an attribute id")?;
                match attribute.strip_prefix("minecraft:").map(|name| name.strip_prefix("generic.").unwrap_or(name)) {
                    Some("max_health") => Self::MaxHealth,
                    _ => return Ok(None),
                }
            }
            "tconstruct:conditional" => {
                let condition = fields.get("condition_type").ok_or("condition_type missing")?;
                let Some(condition) = TargetCondition::decode_for(role, Some(condition))? else { return Ok(None); };
                let branch = |key: &str| -> Result<Option<Box<Self>>, String> { Ok(Self::decode_value(fields.get(key).ok_or_else(|| format!("{key} missing"))?, role)?.map(Box::new)) };
                let (Some(if_true), Some(if_false)) = (branch("if_true")?, branch("if_false")?) else { return Ok(None); };
                Self::Conditional { condition: Box::new(condition), if_true, if_false }
            }
            _ => return Ok(None),
        }))
    }

    /// A variable written as a number, a loader type or an object with a `type`.
    pub fn decode_value(value: &Value, role: &str) -> Result<Option<Self>, String> {
        match value {
            Value::Number(number) => Ok(Some(Self::Constant(number.as_f64().ok_or("variable must be a number")? as f32))),
            Value::String(kind) => Self::decode(kind, &Map::new(), role),
            Value::Object(fields) => Self::decode(fields.get("type").and_then(Value::as_str).ok_or("variable type missing")?, fields, role),
            _ => Err("variable must be a number, a type or an object".into()),
        }
    }

    /// The value for the entity `whose` of `entities`; None when that entity or the fact was not reported.
    pub fn value(&self, entities: &Entities, whose: Whose) -> Option<f32> {
        if let Self::Constant(value) = self { return Some(*value); }
        let state = match whose { Whose::Attacker => entities.attacker, Whose::Target => entities.target }?;
        match self {
            Self::Health => state.health,
            Self::MaxHealth => state.max_health,
            Self::BiomeTemperature => state.biome_temperature,
            Self::Water => Some(if state.in_water || state.eyes_in_water { 2.0 } else if state.rain_exposed { 1.0 } else { 0.0 }),
            Self::ArmorCoverage => state.armor_coverage,
            Self::EquipmentCount(slots) => state.equipment.as_ref().map(|held| slots.iter().filter(|slot| held.iter().any(|name| &name == slot)).count() as f32),
            Self::Light(layer) => {
                let sky = state.sky_light.map(|light| (i32::from(light) - i32::from(state.sky_darken)).max(0));
                let block = state.block_light.map(i32::from);
                match layer { Some(LightLayer::Sky) => sky, Some(LightLayer::Block) => block, None => sky.zip(block).map(|(sky, block)| sky.max(block)) }.map(|light| light as f32)
            }
            // A state that names effects without levels says nothing about the level.
            Self::EffectLevel(effect) => (state.effect_levels.len() >= state.effects.len()).then(|| state.effect_level(effect) as f32),
            Self::Conditional { condition, if_true, if_false } => {
                let holds = condition.matches(&|tag| state_in_tag(tag, entities.attacker, entities.target));
                (if holds { if_true } else { if_false }).value(entities, whose)
            }
            Self::Constant(value) => Some(*value),
        }
    }

    /// The name this variable has in a range condition's pseudo tag: only the variables with nothing to configure have one.
    pub fn range_name(&self) -> Option<&'static str> {
        Some(match self { Self::Health => "health", Self::MaxHealth => "max_health", Self::BiomeTemperature => "biome_temperature", Self::Water => "water", Self::ArmorCoverage => "armor_coverage", _ => return None })
    }

    /// The variable a range condition's pseudo tag names.
    pub fn from_range_name(name: &str) -> Option<Self> {
        Some(match name { "health" => Self::Health, "max_health" => Self::MaxHealth, "biome_temperature" => Self::BiomeTemperature, "water" => Self::Water, "armor_coverage" => Self::ArmorCoverage, _ => return None })
    }
}

/// VariableRangePredicate.IntervalType.test.
pub(crate) fn in_interval(interval: &str, value: f32, min: f32, max: f32) -> bool {
    match interval {
        "open" => min < value && value < max,
        "left_open" => min < value && value <= max,
        "right_open" => min <= value && value < max,
        _ => min <= value && value <= max,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn fields(value: Value) -> Map<String, Value> { value.as_object().unwrap().clone() }

    #[test]
    fn variables_read_what_was_reported_and_say_nothing_about_what_was_not() {
        let state = EntityState { health: Some(6.0), max_health: Some(20.0), biome_temperature: Some(0.8), rain_exposed: true, armor_coverage: Some(0.5),
            effects: vec!["minecraft:poison".into()], effect_levels: vec![("minecraft:poison".into(), 2)], ..EntityState::default() };
        let entities = Entities { attacker: Some(&state), target: None };
        let read = |kind: &str, extra: Value| EntityFact::decode(kind, &fields(extra), "attacker").unwrap().unwrap().value(&entities, Whose::Attacker);
        assert_eq!(read("tconstruct:health", json!({})), Some(6.0));
        assert_eq!(read("tconstruct:attribute", json!({"attribute": "minecraft:generic.max_health"})), Some(20.0));
        assert_eq!(read("tconstruct:biome_temperature", json!({})), Some(0.8));
        assert_eq!(read("tconstruct:water", json!({})), Some(1.0), "rain without water is 1");
        assert_eq!(read("tconstruct:armor_coverage", json!({})), Some(0.5));
        assert_eq!(read("tconstruct:effect_level", json!({"effect": "minecraft:poison"})), Some(3.0), "amplifier 2 is level III");
        assert_eq!(read("tconstruct:effect_level", json!({"effect": "minecraft:speed"})), Some(0.0));
        assert_eq!(EntityFact::Health.value(&entities, Whose::Target), None, "no target was reported");
        assert!(EntityFact::decode("tconstruct:attribute", &fields(json!({"attribute": "minecraft:generic.armor"})), "attacker").unwrap().is_none());
        // Equipment counts read the slots the state names as filled; a state that says nothing of equipment says nothing here.
        let counting = EntityFact::decode("tconstruct:equipment_count", &fields(json!({"slots": ["mainhand", "offhand"]})), "attacker").unwrap().unwrap();
        let armed = EntityState { equipment: Some(vec!["mainhand".into(), "head".into()]), ..EntityState::default() };
        assert_eq!(counting.value(&Entities { attacker: Some(&armed), target: None }, Whose::Attacker), Some(1.0));
        assert_eq!(counting.value(&entities, Whose::Attacker), None);
        assert!(EntityFact::decode("tconstruct:equipment_count", &fields(json!({"slots": []})), "attacker").unwrap().is_none());
        let unleveled = EntityState { effects: vec!["minecraft:poison".into()], ..EntityState::default() };
        assert_eq!(EntityFact::EffectLevel("minecraft:poison".into()).value(&Entities { attacker: Some(&unleveled), target: None }, Whose::Attacker), None);
    }

    #[test]
    fn light_takes_the_time_of_day_off_the_sky_and_reads_both_layers_together() {
        let read = |layer: Value, state: &EntityState| {
            let fact = EntityFact::decode("tconstruct:light", &fields(json!({"light_layer": layer})), "target").unwrap().unwrap();
            fact.value(&Entities { attacker: None, target: Some(state) }, Whose::Target)
        };
        let dusk = EntityState { sky_light: Some(15), sky_darken: 11, block_light: Some(4), ..EntityState::default() };
        assert_eq!(read(json!("sky"), &dusk), Some(4.0), "15 less the 11 the night takes");
        assert_eq!(read(json!("block"), &dusk), Some(4.0));
        let torchlit = EntityState { sky_light: Some(3), sky_darken: 11, block_light: Some(9), ..EntityState::default() };
        assert_eq!(read(json!("sky"), &torchlit), Some(0.0), "never below zero");
        assert_eq!(read(json!(null), &torchlit), Some(9.0), "no layer: the brighter of the two");
        let unreported = EntityState { sky_light: Some(15), ..EntityState::default() };
        assert_eq!(read(json!(null), &unreported), None, "both layers are needed");
        assert!(EntityFact::decode("tconstruct:light", &fields(json!({"light_layer": "moon"})), "target").unwrap().is_none());
    }

    #[test]
    fn a_conditional_variable_picks_by_full_health() {
        let fact = EntityFact::decode("tconstruct:conditional", &fields(json!({"condition_type": "tconstruct:full_health", "if_true": 1.0, "if_false": 0.0})), "target").unwrap().unwrap();
        let hurt = EntityState { health: Some(5.0), max_health: Some(20.0), ..EntityState::default() };
        let whole = EntityState { health: Some(19.5), max_health: Some(20.0), ..EntityState::default() };
        let of = |target: &EntityState| fact.value(&Entities { attacker: None, target: Some(target) }, Whose::Target);
        assert_eq!((of(&hurt), of(&whole)), (Some(0.0), Some(1.0)), "the ceiling of the health counts, as the health bar draws it");
    }
}
