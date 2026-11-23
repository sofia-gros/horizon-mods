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
    /// hold. The knockback modifier's own sling force names two of the holder's
    /// attributes: the knockback resistance, which no report carries and so reads its
    /// fallback, and `tconstruct:generic.knockback_multiplier`, an attribute Tinkers
    /// declares itself, which reads the holder's value as of their last tick.
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

/// A SlingModule: charges like a bow and spends the charge as a force on release.
/// SlingKnockbackModule bonks a target in front of the holder, SlingLeapModule launches
/// the holder, SlingTeleportModule moves the holder along the look. `force` is the
/// force multiplier as a (flat, each_level) leveling value; the launch itself is computed
/// in `sling_hook`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Sling {
    Knockback { force: (f32, f32), drawtime_multiplier: f32, damage_multiplier: f32, target: LivingPredicate },
    Leap { force: (f32, f32), drawtime_multiplier: f32, leave_ground: bool, height_reduction: f32, bias_upwards: bool, target: LivingPredicate },
    Teleport { force: (f32, f32), drawtime_multiplier: f32, target: LivingPredicate },
}

/// A required number field (the original's loaders fail the module without it).
fn required_number(fields: &Map<String, Value>, key: &str) -> Result<f32, String> {
    match fields.get(key).filter(|value| !value.is_null()) {
        Some(value) => value.as_f64().map(|value| value as f32).ok_or_else(|| format!("{key} must be a number")),
        None => Err(format!("{key} is required")),
    }
}

/// A required leveling value field (LevelingValue in the original's loaders).
fn required_leveling(fields: &Map<String, Value>, key: &str) -> Result<(f32, f32), String> {
    if fields.get(key).filter(|value| !value.is_null()).is_none() { return Err(format!("{key} is required")); }
    leveling(fields, key)
}

/// A required boolean field.
fn required_boolean(fields: &Map<String, Value>, key: &str) -> Result<bool, String> {
    match fields.get(key).filter(|value| !value.is_null()) {
        Some(Value::Bool(value)) => Ok(*value),
        Some(_) => Err(format!("{key} must be a boolean")),
        None => Err(format!("{key} is required")),
    }
}

impl Sling {
    /// SlingKnockbackModule's record: force multiplier, drawtime multiplier and damage
    /// multiplier are all required; the target predicate defaults to any.
    pub(crate) fn decode_knockback(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        let Some(target) = LivingPredicate::decode(fields.get("target"))? else { return Ok(None) };
        Ok(Some(Self::Knockback {
            force: required_leveling(fields, "force_multiplier")?,
            drawtime_multiplier: required_number(fields, "drawtime_multiplier")?,
            damage_multiplier: required_number(fields, "damage_multiplier")?,
            target,
        }))
    }

    /// SlingLeapModule's record: every field is required in the original's loader, under
    /// the shared SlingModule field names (`force_multiplier`, `drawtime_multiplier`).
    pub(crate) fn decode_leap(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        let Some(target) = LivingPredicate::decode(fields.get("target"))? else { return Ok(None) };
        Ok(Some(Self::Leap {
            force: required_leveling(fields, "force_multiplier")?,
            drawtime_multiplier: required_number(fields, "drawtime_multiplier")?,
            leave_ground: required_boolean(fields, "leave_ground")?,
            height_reduction: required_number(fields, "height_reduction")?,
            bias_upwards: required_boolean(fields, "bias_upwards")?,
            target,
        }))
    }

    /// SlingTeleportModule's record: force multiplier and drawtime multiplier, both
    /// required, under the same shared SlingModule field names as the leap module.
    pub(crate) fn decode_teleport(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        let Some(target) = LivingPredicate::decode(fields.get("target"))? else { return Ok(None) };
        Ok(Some(Self::Teleport {
            force: required_leveling(fields, "force_multiplier")?,
            drawtime_multiplier: required_number(fields, "drawtime_multiplier")?,
            target,
        }))
    }

    /// LevelingValue.compute of the force multiplier at the entry's level.
    pub(crate) fn force_multiplier(&self, entry: &Entry) -> f32 {
        match self {
            Self::Knockback { force, .. } | Self::Leap { force, .. } | Self::Teleport { force, .. } => compute(*force, entry),
        }
    }

    /// SlingModule's drawtime multiplier.
    pub(crate) fn drawtime_multiplier(&self) -> f32 {
        match self {
            Self::Knockback { drawtime_multiplier, .. } | Self::Leap { drawtime_multiplier, .. } | Self::Teleport { drawtime_multiplier, .. } => *drawtime_multiplier,
        }
    }

    /// The predicate over the entity the sling acts on: the found target for a knockback,
    /// the holder for a leap or a teleport.
    pub(crate) fn target(&self) -> &LivingPredicate {
        match self {
            Self::Knockback { target, .. } | Self::Leap { target, .. } | Self::Teleport { target, .. } => target,
        }
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
    fn the_sling_kinds_decode_their_shipped_data() {
        // bonking: the knockback module's own field names.
        let bonking = Sling::decode_knockback(&fields(json!({
            "damage_multiplier": 1.5, "drawtime_multiplier": 1.5,
            "force_multiplier": {"flat": 1.0, "each_level": 2.0}
        }))).unwrap().unwrap();
        assert_eq!(bonking.drawtime_multiplier(), 1.5);
        // flinging: the leap module's shipped data (the shared SlingModule field names),
        // a negative force.
        let flinging = Sling::decode_leap(&fields(json!({
            "bias_upwards": false, "drawtime_multiplier": 1.5,
            "force_multiplier": {"each_level": -2.0, "flat": -2.0},
            "height_reduction": 3.0, "leave_ground": false,
            "target": {"type": "mantle:and", "predicates": ["mantle:on_ground", "tconstruct:targeting_block"]}
        }))).unwrap().unwrap();
        assert_eq!(flinging, Sling::Leap {
            force: (-2.0, -2.0), drawtime_multiplier: 1.5, leave_ground: false,
            height_reduction: 3.0, bias_upwards: false,
            target: LivingPredicate::And(vec![LivingPredicate::OnGround, LivingPredicate::TargetingBlock]),
        });
        // springing: the other shipped leap data, force carried by each_level alone.
        let springing = Sling::decode_leap(&fields(json!({
            "bias_upwards": true, "drawtime_multiplier": 1.0,
            "force_multiplier": {"each_level": 1.05},
            "height_reduction": 2.0, "leave_ground": true,
            "target": {"type": "mantle:inverted", "inverted_type": "mantle:elytra_flying"}
        }))).unwrap().unwrap();
        assert_eq!(springing, Sling::Leap {
            force: (0.0, 1.05), drawtime_multiplier: 1.0, leave_ground: true,
            height_reduction: 2.0, bias_upwards: true,
            target: LivingPredicate::Inverted(Box::new(LivingPredicate::ElytraFlying)),
        });
        // warping: the teleport module's shipped data.
        let warping = Sling::decode_teleport(&fields(json!({
            "drawtime_multiplier": 1.5, "force_multiplier": {"each_level": 4.0, "flat": 2.0}
        }))).unwrap().unwrap();
        assert_eq!(warping, Sling::Teleport {
            force: (2.0, 4.0), drawtime_multiplier: 1.5, target: LivingPredicate::Any,
        });
        // A missing required field is a data error, as the original's loader fails.
        assert!(Sling::decode_knockback(&fields(json!({"damage_multiplier": 1.5}))).is_err());
    }

    #[test]
    fn the_plain_sling_force_formula_binds() {
        // padded's instance: a formula over $value and $level only.
        let plain = SlingForce::decode(&fields(json!({
            "formula": ["$value", 2.0, "$level", "^", "/"], "percent": false,
            "sling": {"type": "tconstruct:tag", "tag": "tconstruct:knockback_slings"}
        }))).unwrap();
        assert!(plain.is_some());
    }

    #[test]
    fn the_knockback_sling_force_binds_and_reads_the_holders_attributes() {
        // knockback's own instance, as shipped: its variables read the holder's knockback
        // resistance (a fact no report carries, so its fallback of 0 stands) and the
        // knockback multiplier, an attribute Tinkers declares itself, read as of the
        // holder's last tick (its default of 1 while nothing bends it).
        let module = SlingForce::decode(&fields(json!({
            "formula": [0.25, "$level", "*", "$multiplier", "*", 1.0, "$knockback_resistance", "-", "*", "$knockback_multiplier", "*", "$value", "+"],
            "percent": false,
            "sling": {"type": "tconstruct:tag", "tag": "tconstruct:knockback_slings/self"},
            "variables": {
                "knockback_multiplier": {"type": "tconstruct:entity", "attribute": "tconstruct:generic.knockback_multiplier", "entity_type": "tconstruct:attribute", "fallback": 1.0},
                "knockback_resistance": {"type": "tconstruct:entity", "attribute": "minecraft:generic.knockback_resistance", "entity_type": "tconstruct:attribute", "fallback": 0.0}
            }
        }))).unwrap().expect("the shipped knockback sling force binds");
        // 0.25 * level 2 * multiplier 3 * (1 - 0) * 1 + value 10 = 11.5.
        let entry = Entry::new(Identifier::parse("tconstruct:knockback").unwrap(), 2);
        let nbt = pluto_lib::NbtCompound::new();
        let holder_state = pluto_lib::EntityState::default();
        let entities = crate::entity_facts::Entities { attacker: Some(&holder_state), target: None };
        let tool = ToolView::with(&nbt, entities);
        let bonking = Identifier::parse("tconstruct:bonking").unwrap();
        let in_tag = |id: &Identifier, tag: &Identifier| id == &bonking && tag.to_string() == "tconstruct:knockback_slings/self";
        let force = crate::entity_facts::with_holder_entity(Some(7), || module.apply(&entry, true, &tool, &LivingState::default(), &LivingState::default(), &bonking, &in_tag, 10.0, 3.0));
        assert_eq!(force, 11.5);
    }
}
