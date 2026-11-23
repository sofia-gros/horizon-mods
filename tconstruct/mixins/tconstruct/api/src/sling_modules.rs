//! The sling modifier modules (SlimeKnights, MIT; see LICENSE): SlingForceModule and the
//! SlingModule family (SlingKnockbackModule, SlingLeapModule, SlingTeleportModule).
//!
//! A sling module charges like a bow and, on release, spends the charge as a force: the power
//! is half the tool's projectile damage times its velocity, each sling force module of the
//! tool may reshape that force through its formula (SlingForceModifierHook), and the kind
//! decides where the force lands — on a target in front of the holder (knockback), on the
//! holder (leap), or as a teleport along the look (teleport). The launch computation lives in
//! `sling_hook`; this file is the data: the decoded modules and the two predicate languages
//! they carry (Mantle's living entity predicates over the target, and the modifier predicate
//! over which modifier is doing the slinging).
use crate::mining_speed::{EntityScope, StatFormula, ToolView};
use crate::modifier_entries::Entry;
use pluto_lib::Identifier;
use serde_json::{Map, Value};

fn number(fields: &Map<String, Value>, key: &str) -> Result<f32, String> {
    match fields.get(key).filter(|value| !value.is_null()) {
        None => Ok(0.0),
        Some(value) => value.as_f64().map(|value| value as f32).ok_or_else(|| format!("{key} must be a number")),
    }
}

fn identifier(fields: &Map<String, Value>, key: &str) -> Result<Identifier, String> {
    Identifier::parse(fields.get(key).and_then(Value::as_str).ok_or_else(|| format!("{key} must be an ID string"))?).map_err(|e| e.to_string())
}

fn boolean(fields: &Map<String, Value>, key: &str, fallback: bool) -> Result<bool, String> {
    match fields.get(key).filter(|value| !value.is_null()) {
        None => Ok(fallback),
        Some(Value::Bool(value)) => Ok(*value),
        Some(_) => Err(format!("{key} must be a boolean")),
    }
}

/// A leveling value under its own key (LevelingValue.LOADABLE): a plain number is the flat
/// part, an object names `flat` and `each_level`; absent, both parts are zero. (flat, each_level).
fn leveling(fields: &Map<String, Value>, key: &str) -> Result<(f32, f32), String> {
    match fields.get(key).filter(|value| !value.is_null()) {
        None => Ok((0.0, 0.0)),
        Some(Value::Number(value)) => Ok((value.as_f64().ok_or_else(|| format!("{key} must be a number"))? as f32, 0.0)),
        Some(Value::Object(object)) => Ok((number(object, "flat")?, number(object, "each_level")?)),
        Some(_) => Err(format!("{key} must be a leveling value")),
    }
}

/// LevelingValue.compute: `flat + each_level * level`.
fn compute(leveling: (f32, f32), entry: &Entry) -> f32 {
    leveling.0 + leveling.1 * entry.effective_level()
}

/// The facts of a living entity a sling predicate reads. A fact no event reported is false:
/// the release telling names neither the holder's fall flying nor a target's state, so an
/// elytra predicate reads not-flying and a target's ground reads airborne (the shipped
/// knockback and teleport modules carry no target predicate, and the leap modules predicate
/// on the holder, whose ground the player tick reports — see `sling_hook`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct LivingState {
    pub on_ground: bool,
    pub elytra_flying: bool,
    pub targeting_block: bool,
}

/// Mantle's LivingEntityPredicate, in the forms the shipped sling data uses: a bare string
/// names a leaf predicate, an object names a combinator (`mantle:and`, `mantle:or`,
/// `mantle:inverted`) or a leaf by its `type`. Any other form leaves the module unbound.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum LivingPredicate {
    Any,
    OnGround,
    ElytraFlying,
    TargetingBlock,
    And(Vec<LivingPredicate>),
    Or(Vec<LivingPredicate>),
    Inverted(Box<LivingPredicate>),
}

impl LivingPredicate {
    /// `None` for a predicate form this port cannot evaluate (the module stays unbound).
    pub(crate) fn decode(value: Option<&Value>) -> Result<Option<Self>, String> {
        match value {
            None | Some(Value::Null) => Ok(Some(Self::Any)),
            Some(Value::String(kind)) => Self::leaf(kind),
            Some(Value::Object(object)) => {
                let kind = object.get("type").and_then(Value::as_str).ok_or("living predicate type missing")?;
                match kind {
                    "mantle:and" | "mantle:or" => {
                        let list = object.get("predicates").and_then(Value::as_array).ok_or("living predicate list missing")?;
                        let mut parts = Vec::with_capacity(list.len());
                        for part in list {
                            match Self::decode(Some(part))? {
                                Some(part) => parts.push(part),
                                None => return Ok(None),
                            }
                        }
                        Ok(Some(if kind == "mantle:and" { Self::And(parts) } else { Self::Or(parts) }))
                    }
                    "mantle:inverted" => match Self::decode(object.get("inverted_type"))? {
                        Some(inner) => Ok(Some(Self::Inverted(Box::new(inner)))),
                        None => Ok(None),
                    },
                    _ => Self::leaf(kind),
                }
            }
            Some(_) => Err("living predicate must be a string or an object".into()),
        }
    }

    fn leaf(kind: &str) -> Result<Option<Self>, String> {
        Ok(Some(match kind {
            "mantle:any" => Self::Any,
            "mantle:on_ground" => Self::OnGround,
            "mantle:elytra_flying" => Self::ElytraFlying,
            "tconstruct:targeting_block" => Self::TargetingBlock,
            _ => return Ok(None),
        }))
    }

    pub(crate) fn matches(&self, state: &LivingState) -> bool {
        match self {
            Self::Any => true,
            Self::OnGround => state.on_ground,
            Self::ElytraFlying => state.elytra_flying,
            Self::TargetingBlock => state.targeting_block,
            Self::And(parts) => parts.iter().all(|part| part.matches(state)),
            Self::Or(parts) => parts.iter().any(|part| part.matches(state)),
            Self::Inverted(inner) => !inner.matches(state),
        }
    }
}

/// The modifier predicate over the modifier doing the slinging (ModifierPredicate): a tag
/// the sling's modifier must be in, or one named modifier. Any other form leaves the
/// module unbound.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ModifierPredicate {
    Any,
    Id(Identifier),
    Tag(Identifier),
    And(Vec<ModifierPredicate>),
    Or(Vec<ModifierPredicate>),
    Not(Box<ModifierPredicate>),
}

impl ModifierPredicate {
    /// `None` for a predicate form this port cannot evaluate (the module stays unbound).
    pub(crate) fn decode(value: Option<&Value>) -> Result<Option<Self>, String> {
        match value {
            None | Some(Value::Null) => Ok(Some(Self::Any)),
            Some(Value::String(text)) => Ok(Some(Self::Id(Identifier::parse(text).map_err(|e| e.to_string())?))),
            Some(Value::Object(object)) => {
                let kind = object.get("type").and_then(Value::as_str).ok_or("modifier predicate type missing")?;
                match kind {
                    "tconstruct:tag" => Ok(Some(Self::Tag(identifier(object, "tag")?))),
                    "tconstruct:single" | "tconstruct:modifier" => Ok(Some(Self::Id(identifier(object, "modifier")?))),
                    "tconstruct:and" | "tconstruct:or" => {
                        let list = object.get("predicates").and_then(Value::as_array).ok_or("modifier predicate list missing")?;
                        let mut parts = Vec::with_capacity(list.len());
                        for part in list {
                            match Self::decode(Some(part))? {
                                Some(part) => parts.push(part),
                                None => return Ok(None),
                            }
                        }
                        Ok(Some(if kind == "tconstruct:and" { Self::And(parts) } else { Self::Or(parts) }))
                    }
                    "tconstruct:not" => match Self::decode(object.get("predicate"))? {
                        Some(inner) => Ok(Some(Self::Not(Box::new(inner)))),
                        None => Ok(None),
                    },
                    _ => Ok(None),
                }
            }
            Some(_) => Err("modifier predicate must be a string or an object".into()),
        }
    }

    /// `in_tag` answers modifier tag membership (the registry's reverse tags).
    pub(crate) fn matches(&self, id: &Identifier, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> bool {
        match self {
            Self::Any => true,
            Self::Id(wanted) => wanted == id,
            Self::Tag(tag) => in_tag(id, tag),
            Self::And(parts) => parts.iter().all(|part| part.matches(id, in_tag)),
            Self::Or(parts) => parts.iter().any(|part| part.matches(id, in_tag)),
            Self::Not(inner) => !inner.matches(id, in_tag),
        }
    }
}

/// SlingForceModule: reshapes the force of a sling through its formula, filtered on the
/// entity receiving the force, the holder, and the modifier doing the slinging. The
/// formula is a ConditionalStatFormula over `level`, `value` (the force so far) and
/// `multiplier`; its entity variables read the holder, as a conditional stat module's do.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SlingForce {
    target: LivingPredicate,
    holder: LivingPredicate,
    sling: ModifierPredicate,
    formula: StatFormula,
}

impl SlingForce {
    /// `None` when a predicate or a formula variable names an input this port does not
    /// hold — shipped: the knockback modifier's own sling force names the holder's
    /// `tconstruct:generic.knockback_multiplier` attribute, which the host does not have,
    /// so that one instance stays unbound while the target instance (a plain formula)
    /// binds.
    pub(crate) fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        let Some(target) = LivingPredicate::decode(fields.get("target"))? else { return Ok(None) };
        let Some(holder) = LivingPredicate::decode(fields.get("holder"))? else { return Ok(None) };
        let Some(sling) = ModifierPredicate::decode(fields.get("sling"))? else { return Ok(None) };
        let Some(formula) = StatFormula::decode_in(fields, &["level", "value", "multiplier"], EntityScope::Holder)? else { return Ok(None) };
        Ok(Some(Self { target, holder, sling, formula }))
    }

    /// SlingForceModifierHook.modifySlingForce: the formula applied to the force when the
    /// module's condition and all three predicates match, else the force unchanged.
    pub(crate) fn apply(&self, entry: &Entry, condition: bool, tool: &ToolView, holder: &LivingState, target: &LivingState, sling: &Identifier, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool, force: f32, multiplier: f32) -> f32 {
        if !condition || !self.target.matches(target) || !self.holder.matches(holder) || !self.sling.matches(sling, in_tag) {
            return force;
        }
        self.formula.eval(entry, tool, force, multiplier, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fields(value: Value) -> Map<String, Value> { value.as_object().unwrap().clone() }

    #[test]
    fn living_predicates_decode_the_shipped_forms() {
        let and = LivingPredicate::decode(Some(&json!({"type": "mantle:and", "predicates": ["mantle:on_ground", "tconstruct:targeting_block"]}))).unwrap().unwrap();
        assert!(and.matches(&LivingState { on_ground: true, elytra_flying: false, targeting_block: true }));
        assert!(!and.matches(&LivingState { on_ground: true, elytra_flying: false, targeting_block: false }));
        let inverted = LivingPredicate::decode(Some(&json!({"type": "mantle:inverted", "inverted_type": "mantle:elytra_flying"}))).unwrap().unwrap();
        assert!(inverted.matches(&LivingState::default()));
        assert!(!inverted.matches(&LivingState { elytra_flying: true, ..Default::default() }));
        assert_eq!(LivingPredicate::decode(None).unwrap(), Some(LivingPredicate::Any));
        assert!(LivingPredicate::decode(Some(&json!({"type": "mantle:in_water"}))).unwrap().is_none(), "an unknown leaf stays unbound");
    }

    #[test]
    fn modifier_predicates_match_tags_and_ids() {
        let tag = ModifierPredicate::decode(Some(&json!({"type": "tconstruct:tag", "tag": "tconstruct:knockback_slings/self"}))).unwrap().unwrap();
        let bonking = Identifier::parse("tconstruct:bonking").unwrap();
        let in_tag = |id: &Identifier, wanted: &Identifier| id == &bonking && wanted.to_string() == "tconstruct:knockback_slings/self";
        assert!(tag.matches(&bonking, &in_tag));
        assert!(!tag.matches(&Identifier::parse("tconstruct:flinging").unwrap(), &in_tag));
    }

    #[test]
    fn the_plain_sling_force_formula_binds_and_the_attribute_one_does_not() {
        // padded's instance: a formula over $value and $level only.
        let plain = SlingForce::decode(&fields(json!({
            "formula": ["$value", 2.0, "$level", "^", "/"], "percent": false,
            "sling": {"type": "tconstruct:tag", "tag": "tconstruct:knockback_slings"}
        }))).unwrap();
        assert!(plain.is_some());
        // knockback's own instance: its variables read the knockback multiplier attribute
        // the host does not have, so the formula cannot decode and the module stays unbound.
        let attributed = SlingForce::decode(&fields(json!({
            "formula": ["$value"], "percent": false,
            "variables": {"knockback_multiplier": {"type": "tconstruct:entity", "attribute": "tconstruct:generic.knockback_multiplier", "entity_type": "tconstruct:attribute", "fallback": 1.0}}
        }))).unwrap();
        assert!(attributed.is_none());
    }
}
