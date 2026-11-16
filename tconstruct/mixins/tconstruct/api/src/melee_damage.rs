//! ConditionalMeleeDamageModule, from SlimeKnights (MIT; see LICENSE), for the modules whose conditions this
//! port can evaluate: the tool's own state and the target's entity type, its mob type and its entity tags. A module
//! that conditions on the attacker (sprinting, effects, biome) or on other target state (fire, health) stays
//! unbound until PlutoLib reports that state, rather than being half-installed.
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

/// IJsonPredicate over the target of the hit, by its entity type tags.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TargetCondition { Any, Tag(Identifier), Not(Box<TargetCondition>), And(Vec<TargetCondition>), Or(Vec<TargetCondition>) }

impl TargetCondition {
    fn decode(value: Option<&Value>) -> Result<Option<Self>, String> {
        let Some(value) = value.filter(|value| !value.is_null()) else { return Ok(Some(Self::Any)); };
        // A bare string is a predicate type with no fields; none of those is known here.
        let Some(object) = value.as_object() else { return Ok(None); };
        let kind = object.get("type").and_then(Value::as_str).ok_or("target type missing")?;
        let list = || -> Result<Option<Vec<Self>>, String> {
            let mut out = Vec::new();
            for child in object.get("predicates").and_then(Value::as_array).ok_or("predicates must be an array")? {
                match Self::decode(Some(child))? { Some(child) => out.push(child), None => return Ok(None) }
            }
            Ok(Some(out))
        };
        Ok(Some(match kind {
            "mantle:any" => Self::Any,
            "mantle:mob_type" => {
                let mobs = object.get("mobs").and_then(Value::as_str).ok_or("mobs must be a mob type")?;
                match mob_type_tag(mobs) { Some(tag) => Self::Tag(Identifier::parse(tag).map_err(|e| e.to_string())?), None => return Ok(None) }
            }
            "mantle:tag" => Self::Tag(Identifier::parse(object.get("tag").and_then(Value::as_str).ok_or("tag missing")?).map_err(|e| e.to_string())?),
            "mantle:inverted" => {
                let mut inner = object.clone();
                let inner_kind = inner.remove("inverted_type").ok_or("inverted_type missing")?;
                inner.insert("type".into(), inner_kind);
                match Self::decode(Some(&Value::Object(inner)))? { Some(inner) => Self::Not(Box::new(inner)), None => return Ok(None) }
            }
            "mantle:and" => match list()? { Some(children) => Self::And(children), None => return Ok(None) },
            "mantle:or" => match list()? { Some(children) => Self::Or(children), None => return Ok(None) },
            _ => return Ok(None),
        }))
    }
    fn matches(&self, in_tag: &dyn Fn(&Identifier) -> bool) -> bool {
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
pub(crate) struct MeleeDamage { target: TargetCondition, formula: StatFormula }

impl MeleeDamage {
    /// `None` when the module names an attacker condition, another target condition or a variable this port does not hold.
    pub fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        if fields.get("attacker").is_some_and(|value| !value.is_null()) { return Ok(None); }
        let Some(target) = TargetCondition::decode(fields.get("target"))? else { return Ok(None); };
        Ok(StatFormula::decode(fields, &NAMES)?.map(|formula| Self { target, formula }))
    }
    /// A percent module runs later in the original (priority 75).
    pub fn percent(&self) -> bool { self.formula.percent() }
    /// MeleeDamageModifierHook.getMeleeDamage: `base` is the attack damage before modifiers, `damage` the running value,
    /// `target_in_tag` answers the target's entity type tags.
    pub fn apply(&self, entry: &Entry, condition: bool, tool: &ToolView, base: f32, damage: f32, target_in_tag: &dyn Fn(&Identifier) -> bool) -> f32 {
        if !condition || !self.target.matches(target_in_tag) { return damage; }
        self.formula.eval(entry, tool, damage, tool.multiplier(Stat::AttackDamage), base)
    }
}
