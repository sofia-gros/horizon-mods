//! ProtectionModule from SlimeKnights (MIT; see LICENSE): armor modifiers that add protection against
//! damage sources, and the way their sum changes the damage taken (ToolEvents.livingHurt, ArmorUtil).
//! Only sources and formulas over data this port holds are decoded: a module that names the wearing or attacking
//! entity, or a formula variable about one (temperature, light, health, equipment), stays unbound.
use crate::{damage_tags::DamageTags, mining_speed::StatFormula};
use pluto_lib::Identifier;
use serde_json::{Map, Value};

/// The damage a protection module is asked about.
pub(crate) struct DamageSource<'a> {
    pub damage_type: &'a Identifier,
    /// The entity that caused the damage differs from the one that dealt it (an arrow's shooter).
    pub indirect: bool,
    pub tags: &'a DamageTags,
}

/// IJsonPredicate over the damage source.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SourceCondition { Any, CanProtect, Indirect, Tag(Identifier), Not(Box<SourceCondition>), And(Vec<SourceCondition>), Or(Vec<SourceCondition>) }

impl SourceCondition {
    /// `None` when the predicate needs state this port does not hold (the attacker's type, say).
    pub fn decode(value: &Value) -> Result<Option<Self>, String> {
        let kind = match value {
            Value::String(text) => text.as_str(),
            Value::Object(fields) => fields.get("type").and_then(Value::as_str).ok_or("damage source predicate needs a type")?,
            _ => return Err("damage source predicate must be a string or object".into()),
        };
        let fields = value.as_object();
        let list = |key: &str| -> Result<Option<Vec<Self>>, String> {
            let entries = fields.and_then(|f| f.get(key)).and_then(Value::as_array).ok_or_else(|| format!("{kind} needs {key}"))?;
            let mut decoded = Vec::new();
            for entry in entries { match Self::decode(entry)? { Some(condition) => decoded.push(condition), None => return Ok(None) } }
            Ok(Some(decoded))
        };
        Ok(Some(match kind {
            "mantle:any" => Self::Any,
            "mantle:can_protect" => Self::CanProtect,
            "mantle:is_indirect" => Self::Indirect,
            "mantle:tag" => {
                let tag = fields.and_then(|f| f.get("tag")).and_then(Value::as_str).ok_or("tag predicate needs a tag")?;
                Self::Tag(Identifier::parse(tag).map_err(|e| e.to_string())?)
            }
            "mantle:inverted" => {
                let inner = fields.and_then(|f| f.get("inverted_type")).ok_or("inverted predicate needs inverted_type")?;
                match Self::decode(inner)? { Some(condition) => Self::Not(Box::new(condition)), None => return Ok(None) }
            }
            "mantle:and" => match list("predicates")? { Some(all) => Self::And(all), None => return Ok(None) },
            "mantle:or" => match list("predicates")? { Some(any) => Self::Or(any), None => return Ok(None) },
            _ => return Ok(None),
        }))
    }

    pub fn matches(&self, source: &DamageSource) -> bool {
        match self {
            Self::Any => true,
            // Vanilla's enchantment protection ignores damage that bypasses invulnerability, and so does this.
            Self::CanProtect => !source.tags.contains_vanilla("bypasses_invulnerability", source.damage_type),
            Self::Indirect => source.indirect,
            Self::Tag(tag) => source.tags.contains(tag, source.damage_type),
            Self::Not(inner) => !inner.matches(source),
            Self::And(all) => all.iter().all(|condition| condition.matches(source)),
            Self::Or(any) => any.iter().any(|condition| condition.matches(source)),
        }
    }
}

/// The module's damage source and protection formula (`value` is the running protection, `level` the modifier level).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Protection { source: SourceCondition, formula: StatFormula }

impl Protection {
    /// `None` for a module this port cannot evaluate.
    pub fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        // The wearing entity and the attacker are conditions on live entities.
        if fields.get("wearing_entity").is_some_and(|value| !value.is_null()) || fields.get("attacker").is_some_and(|value| !value.is_null()) { return Ok(None); }
        let source = match fields.get("damage_source") { None | Some(Value::Null) => SourceCondition::Any, Some(value) => match SourceCondition::decode(value)? { Some(source) => source, None => return Ok(None) } };
        let Some(formula) = StatFormula::decode(fields, &["level", "protection", "multiplier", "base"])? else { return Ok(None); };
        Ok(Some(Self { source, formula }))
    }

    pub fn source(&self) -> &SourceCondition { &self.source }
    pub fn formula(&self) -> &StatFormula { &self.formula }
}

/// Protection that stops counting: vanilla's cap on the sum, 20 points of the 25 that would stop all damage.
pub(crate) const PROTECTION_CAP: f32 = 20.0;

/// ArmorUtil.getDamageForEvent without the armor inverse: the event here fires after vanilla armor and enchantment
/// protection took their share, so the damage is returned to what the vanilla protection alone left (`vanilla` points of
/// the 25), then reduced by the modifiers' `total` (which includes the vanilla points) capped at `cap`.
pub(crate) fn damage_after_protection(damage: f32, vanilla: f32, total: f32, cap: f32) -> f32 {
    if vanilla == total && cap == PROTECTION_CAP { return damage; }
    let mut damage = damage;
    if vanilla > 0.0 { damage /= 1.0 - vanilla.clamp(0.0, 20.0) / 25.0; }
    if total != 0.0 && damage > 0.0 { damage *= 1.0 - total.clamp(-20.0, cap) / 25.0; }
    damage
}
