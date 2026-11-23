//! ConditionalMeleeDamageModule, from SlimeKnights (MIT; see LICENSE), for the modules whose conditions this
//! port can evaluate: the tool's own state, the target's entity type, mob type and entity tags, and what the attacker and the
//! target were doing as the hit landed (sprinting, in the air, burning, fire-immune, in water, an effect on them, full health, a health or
//! temperature range), which PlutoLib reports with the hit. Those state conditions are asked as pseudo entity type tags in the
//! `STATE_NAMESPACE` (`tconstruct_state:attacker/sprinting`), so a condition tree is the one the entity type tags already use. A condition
//! on a fact PlutoLib does not report stays unbound rather than being half-installed.
use crate::{mining_speed::{StatFormula, ToolView}, modifier_entries::Entry, tool_stats::Stat};
use pluto_lib::Identifier;
use serde_json::{Map, Value};

/// MeleeFormula.VARIABLES.
const NAMES: [&str; 4] = ["level", "damage", "multiplier", "base_damage"];

/// MobType as the entity type tags the game keeps for it.
fn mob_type_tag(mobs: &str) -> Option<&'static str> {
    Some(match mobs {
        "minecraft:undead" => "minecraft:undead",
        "minecraft:arthropod" => "minecraft:arthropod",
        "minecraft:illager" => "minecraft:illager",
        "minecraft:water" => "minecraft:aquatic",
        _ => return None,
    })
}

/// The namespace of the pseudo tags that stand for what the attacker or the target was doing: `attacker/sprinting`,
/// `target/on_fire`, `attacker/effect/minecraft/poison`.
pub(crate) const STATE_NAMESPACE: &str = "tconstruct_state";

/// The pseudo tag for a state condition of `role` (`attacker` or `target`).
fn state_tag(role: &str, what: &str) -> Result<Identifier, String> {
    Identifier::new(STATE_NAMESPACE, &format!("{role}/{what}")).map_err(|e| e.to_string())
}

/// A bare predicate name over an entity state, as the original's `mantle:` and `tconstruct:` predicates name them.
fn named_state(role: &str, name: &str) -> Result<Option<Identifier>, String> {
    if role.is_empty() { return Ok(None); }
    let what = match name {
        "mantle:sprinting" => "sprinting",
        "mantle:on_fire" => "on_fire",
        "mantle:fire_immune" => "fire_immune",
        "mantle:underwater" => "in_water",
        "tconstruct:airborne" => "airborne",
        "tconstruct:full_health" => "full_health",
        // What a mob that hurt a wearer was doing is reported without its crouching, so that is asked of the target (the wearer,
        // or what a weapon hit) only. The holder of a dig is the exception: the break-speed event reports the digging player's own eyes
        // and ground, so a condition over the holder as an entity (role `holder`, carried in the attacker slot) may ask them too.
        "mantle:crouching" if role == "target" || role == "holder" => "crouching",
        // The water at the eyes and the feet is reported for the attacker too: the hurt report fills the attacker's eyes from the
        // mob's own fluid state (as read_surroundings fills them for a weapon hit's parties), and the feet are answered from the
        // water the mob stands in.
        "mantle:eyes_in_water" if role == "target" || role == "holder" || role == "attacker" => "eyes_in_water",
        "mantle:feet_in_water" if role == "target" || role == "holder" || role == "attacker" => "feet_in_water",
        // Rain on the entity is reported for both roles (read_surroundings fills it from the weather,
        // the sky light and the biome), unlike the crouching and eye facts above.
        "mantle:raining_at" => "raining",
        _ => return Ok(None),
    };
    state_tag(role, what).map(Some)
}

/// Answers the pseudo tags from what the attacker and the target were doing; false for a state that was not reported.
pub(crate) fn state_in_tag(tag: &Identifier, attacker: Option<&pluto_lib::EntityState>, target: Option<&pluto_lib::EntityState>) -> bool {
    let path = tag.path();
    let (state, what) = match path.split_once('/') {
        Some(("attacker", what)) => (attacker, what),
        // A condition over the holder of a dig names the holder; the dig's view carries them in the attacker slot.
        Some(("holder", what)) => (attacker, what),
        Some(("target", what)) => (target, what),
        _ => return false,
    };
    let Some(state) = state else { return false; };
    match what {
        "sprinting" => state.sprinting,
        "on_fire" => state.fire_ticks > 0,
        "fire_immune" => state.fire_immune,
        "in_water" => state.in_water,
        "crouching" => state.crouching,
        // The eyes are in water, as opposed to a touch of it; the feet are in water when the entity is.
        "eyes_in_water" => state.eyes_in_water,
        "feet_in_water" => state.in_water,
        "raining" => state.rain_exposed,
        // AIRBORNE: off the ground and not in water (climbing and riding are not reported).
        "airborne" => !state.on_ground && !state.in_water,
        // FULL_HEALTH: the ceiling of the health, as the health bar draws it, reaches the maximum; false when either was not reported.
        "full_health" => matches!((state.health, state.max_health), (Some(health), Some(max)) if health.ceil() >= max),
        // `range/<variable>/<min>/<max>/<interval>`: EntityVariableRangePredicate over a variable of the entity.
        _ if what.starts_with("range/") => {
            let mut parts = what["range/".len()..].split('/');
            let (Some(variable), Some(min), Some(max), Some(interval)) = (parts.next(), parts.next(), parts.next(), parts.next()) else { return false; };
            let (Some(variable), Ok(min), Ok(max)) = (crate::entity_facts::EntityFact::from_range_name(variable), min.parse::<f32>(), max.parse::<f32>()) else { return false; };
            let entities = crate::entity_facts::Entities { attacker, target };
            let whose = if path.starts_with("attacker/") { crate::entity_facts::Whose::Attacker } else { crate::entity_facts::Whose::Target };
            variable.value(&entities, whose).is_some_and(|value| crate::entity_facts::in_interval(interval, value, min, max))
        }
        // `enchantment/<namespace>/<path>`: an enchantment on the entity's equipment, as
        // mantle:has_enchantment asks (the levels ride along unasked; presence is the question).
        _ if what.starts_with("enchantment/") => {
            let effect = &what["enchantment/".len()..];
            let (namespace, path) = effect.split_once('/').unwrap_or(("minecraft", effect));
            state.enchantments.iter().any(|id| id == &format!("{namespace}:{path}"))
        }
        _ => what.strip_prefix("effect/").is_some_and(|effect| {
            let (namespace, path) = effect.split_once('/').unwrap_or(("minecraft", effect));
            state.effects.iter().any(|id| id == &format!("{namespace}:{path}"))
        }),
    }
}

/// IJsonPredicate over an entity of the hit, by its entity type tags and what it was doing.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TargetCondition { Any, Tag(Identifier), Not(Box<TargetCondition>), And(Vec<TargetCondition>), Or(Vec<TargetCondition>) }

impl TargetCondition {
    /// Entity type conditions only: a state condition leaves the module unbound (for callers that cannot answer one).
    pub(crate) fn decode(value: Option<&Value>) -> Result<Option<Self>, String> { Self::decode_for("", value) }

    /// `role` is `attacker` or `target`: whose state the state conditions ask about; empty for none.
    pub(crate) fn decode_for(role: &str, value: Option<&Value>) -> Result<Option<Self>, String> {
        let Some(value) = value.filter(|value| !value.is_null()) else { return Ok(Some(Self::Any)); };
        // A bare string is a predicate type with no fields; `mantle:any` is the one that asks nothing.
        if let Some(name) = value.as_str() {
            if name == "mantle:any" { return Ok(Some(Self::Any)); }
            return Ok(named_state(role, name)?.map(Self::Tag));
        }
        let Some(object) = value.as_object() else { return Ok(None); };
        let kind = object.get("type").and_then(Value::as_str).ok_or("target type missing")?;
        let list = || -> Result<Option<Vec<Self>>, String> {
            let mut out = Vec::new();
            for child in object.get("predicates").and_then(Value::as_array).ok_or("predicates must be an array")? {
                match Self::decode_for(role, Some(child))? { Some(child) => out.push(child), None => return Ok(None) }
            }
            Ok(Some(out))
        };
        Ok(Some(match kind {
            "mantle:any" => Self::Any,
            "mantle:has_effect" => {
                if role.is_empty() { return Ok(None); }
                let effect = object.get("effect").and_then(Value::as_str).ok_or("effect must be an effect id")?;
                let (namespace, path) = effect.split_once(':').unwrap_or(("minecraft", effect));
                Self::Tag(state_tag(role, &format!("effect/{namespace}/{path}"))?)
            }
            "mantle:has_enchantment" => {
                if role.is_empty() { return Ok(None); }
                let enchantment = object.get("enchantment").and_then(Value::as_str).ok_or("enchantment must be an enchantment id")?;
                let (namespace, path) = enchantment.split_once(':').unwrap_or(("minecraft", enchantment));
                Self::Tag(state_tag(role, &format!("enchantment/{namespace}/{path}"))?)
            }
            "mantle:sprinting" | "mantle:on_fire" | "mantle:fire_immune" | "mantle:underwater" | "tconstruct:airborne" | "tconstruct:full_health" | "mantle:crouching" | "mantle:eyes_in_water" | "mantle:feet_in_water" => match named_state(role, kind)? { Some(tag) => Self::Tag(tag), None => return Ok(None) },
            "tconstruct:variable_range" => {
                if role.is_empty() { return Ok(None); }
                let variable = object.get("variable").ok_or("variable missing")?;
                let Some(fact) = crate::entity_facts::EntityFact::decode_value(variable, role)? else { return Ok(None); };
                let Some(name) = fact.range_name() else { return Ok(None); };
                let bound = |key: &str, default: f32| -> Result<f32, String> { match object.get(key) { None | Some(Value::Null) => Ok(default), Some(value) => value.as_f64().map(|v| v as f32).ok_or_else(|| format!("{key} must be a number")) } };
                let interval = match object.get("interval").and_then(Value::as_str) { None => "closed", Some(interval @ ("open" | "closed" | "left_open" | "right_open")) => interval, Some(other) => return Err(format!("unknown interval {other}")) };
                Self::Tag(state_tag(role, &format!("range/{name}/{}/{}/{interval}", bound("min", f32::NEG_INFINITY)?, bound("max", f32::INFINITY)?))?)
            }
            "mantle:mob_type" => {
                let mobs = object.get("mobs").and_then(Value::as_str).ok_or("mobs must be a mob type")?;
                match mob_type_tag(mobs) { Some(tag) => Self::Tag(Identifier::parse(tag).map_err(|e| e.to_string())?), None => return Ok(None) }
            }
            "mantle:tag" => Self::Tag(Identifier::parse(object.get("tag").and_then(Value::as_str).ok_or("tag missing")?).map_err(|e| e.to_string())?),
            "mantle:inverted" => {
                let mut inner = object.clone();
                let inner_kind = inner.remove("inverted_type").ok_or("inverted_type missing")?;
                inner.insert("type".into(), inner_kind);
                match Self::decode_for(role, Some(&Value::Object(inner)))? { Some(inner) => Self::Not(Box::new(inner)), None => return Ok(None) }
            }
            "mantle:and" => match list()? { Some(children) => Self::And(children), None => return Ok(None) },
            "mantle:or" => match list()? { Some(children) => Self::Or(children), None => return Ok(None) },
            _ => return Ok(None),
        }))
    }
    pub(crate) fn matches(&self, in_tag: &dyn Fn(&Identifier) -> bool) -> bool {
        match self {
            Self::Any => true,
            Self::Tag(tag) => in_tag(tag),
            Self::Not(inner) => !inner.matches(in_tag),
            Self::And(children) => children.iter().all(|child| child.matches(in_tag)),
            Self::Or(children) => children.iter().any(|child| child.matches(in_tag)),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MeleeDamage { target: TargetCondition, attacker: TargetCondition, formula: StatFormula }

impl MeleeDamage {
    /// `None` when the module names an attacker condition, another target condition or a variable this port does not hold.
    pub fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        let Some(attacker) = TargetCondition::decode_for("attacker", fields.get("attacker"))? else { return Ok(None); };
        let Some(target) = TargetCondition::decode_for("target", fields.get("target"))? else { return Ok(None); };
        Ok(StatFormula::decode_in(fields, &NAMES, crate::mining_speed::EntityScope::Hit)?.map(|formula| Self { target, attacker, formula }))
    }
    /// A percent module runs later in the original (priority 75).
    pub fn percent(&self) -> bool { self.formula.percent() }
    /// MeleeDamageModifierHook.getMeleeDamage: `base` is the attack damage before modifiers, `damage` the running value,
    /// `target_in_tag` answers the target's entity type tags and, as pseudo tags, what the attacker and the target were doing.
    pub fn apply(&self, entry: &Entry, condition: bool, tool: &ToolView, base: f32, damage: f32, target_in_tag: &dyn Fn(&Identifier) -> bool) -> f32 {
        if !condition || !self.target.matches(target_in_tag) || !self.attacker.matches(target_in_tag) { return damage; }
        self.formula.eval(entry, tool, damage, tool.multiplier(Stat::AttackDamage), base)
    }
}
