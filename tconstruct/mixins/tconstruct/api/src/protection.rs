//! ProtectionModule from SlimeKnights (MIT; see LICENSE): armor modifiers that add protection against
//! damage sources, and the way their sum changes the damage taken (ToolEvents.livingHurt, ArmorUtil).
//! Only sources, wearer conditions and formulas over data this port holds are decoded: the wearer is the `target` of the hit, as PlutoLib
//! reported it as of their last tick (health, temperature, light, height, crouching, water, armor coverage), so a condition or formula variable
//! on anything else stays unbound. The attacking entity is asked by its entity type tags and what it was doing (burning, in the air), as for
//! melee damage, and a formula variable of the attacker reads the state PlutoLib reported for it with the hit, as
//! `EntityProtectionVariable` reads the damage source's entity (health, light, the equipment it holds, and the rest of `entity_facts.rs`);
//! a hit that brought no attacker (a fall, a block) leaves those variables at their fallback, as the original leaves them.
use crate::{damage_tags::DamageTags, mining_speed::StatFormula};
use pluto_lib::Identifier;
use serde_json::{Map, Value};

/// The damage a protection module is asked about.
pub(crate) struct DamageSource<'a> {
    pub damage_type: &'a Identifier,
    /// The entity that caused the damage differs from the one that dealt it (an arrow's shooter).
    pub indirect: bool,
    pub tags: &'a DamageTags,
    /// Answers the attacker's entity type tags and, as pseudo tags, what it was doing; false for everything when the attacker is unknown.
    pub attacker_in_tag: &'a dyn Fn(&Identifier) -> bool,
    /// What the attacker and the wearer (as the `target`) were doing, as PlutoLib reported them; both None for a source with no entities.
    pub entities: crate::entity_facts::Entities<'a>,
}

impl DamageSource<'_> {
    /// Answers the wearer's pseudo state tags (`tconstruct_state:target/crouching`) from what was reported of them.
    pub fn wearer_in_tag(&self, tag: &Identifier) -> bool { crate::melee_damage::state_in_tag(tag, self.entities.attacker, self.entities.target) }
}

/// IJsonPredicate over the damage source.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SourceCondition { Any, CanProtect, Indirect, Tag(Identifier), Type(Identifier), Not(Box<SourceCondition>), And(Vec<SourceCondition>), Or(Vec<SourceCondition>), Attacker(crate::melee_damage::TargetCondition), HasEntity }

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
            "mantle:has_entity" => Self::HasEntity,
            "mantle:tag" => {
                let tag = fields.and_then(|f| f.get("tag")).and_then(Value::as_str).ok_or("tag predicate needs a tag")?;
                Self::Tag(Identifier::parse(tag).map_err(|e| e.to_string())?)
            }
            "mantle:damage_type" => {
                let name = fields.and_then(|f| f.get("name")).and_then(Value::as_str).ok_or("damage_type predicate needs a name")?;
                Self::Type(Identifier::parse(name).map_err(|e| e.to_string())?)
            }
            "mantle:inverted" => {
                let inner = fields.and_then(|f| f.get("inverted_type")).ok_or("inverted predicate needs inverted_type")?;
                // A named inverted type reads the rest of this object as its own fields
                // (enderdodging's inverted tag names its tag beside the kind); an object
                // inverted type is decoded as it stands.
                let decoded = match inner {
                    Value::String(name) => {
                        let mut object = fields.cloned().unwrap_or_default();
                        object.remove("inverted_type");
                        object.insert("type".into(), Value::String(name.clone()));
                        Self::decode(&Value::Object(object))?
                    }
                    _ => Self::decode(inner)?,
                };
                match decoded { Some(condition) => Self::Not(Box::new(condition)), None => return Ok(None) }
            }
            // The entity that caused the damage, by its type tags and what it was doing; the direct entity of an arrow is not reported.
            "mantle:attacker" => {
                if fields.and_then(|f| f.get("which")).and_then(Value::as_str) != Some("causing") { return Ok(None); }
                match crate::melee_damage::TargetCondition::decode_for("attacker", fields.and_then(|f| f.get("entity_type")))? { Some(condition) => Self::Attacker(condition), None => return Ok(None) }
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
            Self::HasEntity => source.entities.attacker.is_some(),
            Self::Tag(tag) => source.tags.contains(tag, source.damage_type),
            Self::Type(name) => source.damage_type == name,
            Self::Not(inner) => !inner.matches(source),
            Self::And(all) => all.iter().all(|condition| condition.matches(source)),
            Self::Or(any) => any.iter().any(|condition| condition.matches(source)),
            Self::Attacker(condition) => condition.matches(source.attacker_in_tag),
        }
    }
}

/// The module's damage source and protection formula (`value` is the running protection, `level` the modifier level).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Protection { source: SourceCondition, wearer: crate::melee_damage::TargetCondition, attacker: crate::melee_damage::TargetCondition, formula: StatFormula }

impl Protection {
    /// `None` for a module this port cannot evaluate.
    pub fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        let Some(wearer) = crate::melee_damage::TargetCondition::decode_for("target", fields.get("wearing_entity"))? else { return Ok(None); };
        let Some(attacker) = crate::melee_damage::TargetCondition::decode_for("attacker", fields.get("attacker"))? else { return Ok(None); };
        let source = match fields.get("damage_source") { None | Some(Value::Null) => SourceCondition::Any, Some(value) => match SourceCondition::decode(value)? { Some(source) => source, None => return Ok(None) } };
        let Some(formula) = StatFormula::decode_in(fields, &["level", "protection", "multiplier", "base"], crate::mining_speed::EntityScope::Hit)? else { return Ok(None); };
        Ok(Some(Self { source, wearer, attacker, formula }))
    }

    pub fn source(&self) -> &SourceCondition { &self.source }
    /// Whether the wearer passes the module's `wearing_entity` condition.
    pub fn wearer_matches(&self, source: &DamageSource) -> bool { self.wearer.matches(&|tag| source.wearer_in_tag(tag)) }
    /// Whether the attacker passes the module's condition on it.
    pub fn attacker_matches(&self, source: &DamageSource) -> bool { self.attacker.matches(source.attacker_in_tag) }
    pub fn formula(&self) -> &StatFormula { &self.formula }
}

/// AdjustDamageModule: changes the damage a wearer is about to take, while the wearer (`holder`, read as the `target` of the hit) and the
/// damage source pass their conditions. The formula runs over the level and the damage so far (`protection`), as the original's does.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AdjustDamage { holder: crate::melee_damage::TargetCondition, source: SourceCondition, formula: StatFormula }

impl AdjustDamage {
    /// `None` when the holder, the source or a formula variable needs a fact this port does not hold.
    pub fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        let Some(holder) = crate::melee_damage::TargetCondition::decode_for("target", fields.get("holder"))? else { return Ok(None); };
        let source = match fields.get("damage_source") { None | Some(Value::Null) => SourceCondition::Any, Some(value) => match SourceCondition::decode(value)? { Some(source) => source, None => return Ok(None) } };
        let formula = StatFormula::decode_in(fields, &["level", "protection", "multiplier", "base"], crate::mining_speed::EntityScope::Hit)?;
        Ok(formula.map(|formula| Self { holder, source, formula }))
    }
    /// ModifyDamageModifierHook.modifyDamageTaken: the damage after the module; `holder_in_tag` answers the wearer's pseudo state tags.
    pub fn apply(&self, entry: &crate::modifier_entries::Entry, tool: &crate::mining_speed::ToolView, source: &DamageSource, holder_in_tag: &dyn Fn(&Identifier) -> bool, amount: f32) -> f32 {
        if !self.source.matches(source) || !self.holder.matches(holder_in_tag) { return amount; }
        self.formula.eval(entry, tool, amount, 1.0, 0.0)
    }
}

/// Protection that stops counting: vanilla's cap on the sum, 20 points of the 25 that would stop all damage.
pub(crate) const PROTECTION_CAP: f32 = 20.0;

/// ProtectionModifierHook.getProtectionCap: the cap attribute (0.8 by default, 25 points to the unit) with what the armor adds,
/// never above 95% of the 25.
pub(crate) fn protection_cap(added: f32) -> f32 { ((0.8 + added) * 25.0).min(25.0 * 0.95) }

/// ToolEvents.getProtectionModifier: a victim in `forge:small_armor` counts its protection fourfold — the
/// enchantment points and the modifiers' together, as they were accumulated into the one total.
pub(crate) fn small_armor_total(total: f32) -> f32 { total * 4.0 }

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

/// Enchantment.getDamageProtection for the protection enchantments, the points the game's own enchantment protection gives a level
/// against a damage source (0 for any other enchantment and for damage that bypasses invulnerability).
pub(crate) fn enchantment_points(enchantment: &str, level: i32, source: &DamageSource) -> f32 {
    if level <= 0 || source.tags.contains_vanilla("bypasses_invulnerability", source.damage_type) { return 0.0; }
    let (factor, tag) = match enchantment {
        "minecraft:protection" => return level as f32,
        "minecraft:fire_protection" => (2, "is_fire"),
        "minecraft:feather_falling" => (3, "is_fall"),
        "minecraft:blast_protection" => (2, "is_explosion"),
        "minecraft:projectile_protection" => (2, "is_projectile"),
        _ => return 0.0,
    };
    if source.tags.contains_vanilla(tag, source.damage_type) { (level * factor) as f32 } else { 0.0 }
}

/// The points the game's enchantment protection gave the damage from the worn pieces' enchantments (each piece's levels per enchantment,
/// as the modifiers give them): none for damage that bypasses enchantments.
pub(crate) fn host_enchantment_points<'a>(pieces: impl IntoIterator<Item = &'a std::collections::BTreeMap<String, i32>>, source: &DamageSource) -> f32 {
    if source.tags.contains_vanilla("bypasses_enchantments", source.damage_type) { return 0.0; }
    pieces.into_iter().flat_map(|levels| levels.iter()).map(|(name, level)| enchantment_points(name, *level, source)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{damage_tags::DamageTags, entity_facts::Entities};
    use pluto_lib::EntityState;
    use serde_json::json;

    fn with_source<R>(damage_type: &str, attacker: Option<&EntityState>, run: impl FnOnce(&DamageSource) -> R) -> R {
        let tags = DamageTags::load(&[]).0;
        let damage_type = Identifier::parse(damage_type).unwrap();
        let none = |_: &Identifier| false;
        run(&DamageSource { damage_type: &damage_type, indirect: false, tags: &tags, attacker_in_tag: &none, entities: Entities { attacker, target: None } })
    }

    #[test]
    fn has_entity_matches_only_a_source_with_a_causing_entity() {
        // mantle:has_entity (enderdodging's second module): the bare and object forms decode
        // alike, and the condition holds exactly when the source reports a causing entity.
        let bare = SourceCondition::decode(&json!("mantle:has_entity")).unwrap().unwrap();
        let object = SourceCondition::decode(&json!({"type": "mantle:has_entity"})).unwrap().unwrap();
        assert_eq!(bare, object);
        let attacker = EntityState::default();
        assert!(with_source("minecraft:mob_attack", Some(&attacker), |source| bare.matches(source)));
        assert!(!with_source("minecraft:fall", None, |source| bare.matches(source)));
    }

    #[test]
    fn inverted_reads_the_sibling_fields_of_a_named_inverted_type() {
        // enderdodging's second module names the inverted tag beside the kind; the tag is a
        // field of the same object, not of the inverted_type value.
        let condition = SourceCondition::decode(&json!({"type": "mantle:inverted", "inverted_type": "mantle:tag", "tag": "minecraft:is_projectile"})).unwrap().unwrap();
        assert!(!with_source("minecraft:arrow", None, |source| condition.matches(source)), "a projectile is not inverted-matching");
        assert!(with_source("minecraft:mob_attack", None, |source| condition.matches(source)));
        // The field-less named form keeps its meaning (melee protection's indirect inversion).
        let indirect = SourceCondition::decode(&json!({"type": "mantle:inverted", "inverted_type": "mantle:is_indirect"})).unwrap().unwrap();
        assert!(with_source("minecraft:mob_attack", None, |source| indirect.matches(source)), "a direct source is not indirect");
    }
}
