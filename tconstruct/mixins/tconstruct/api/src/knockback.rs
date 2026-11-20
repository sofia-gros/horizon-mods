//! KnockbackModule, from SlimeKnights (MIT; see LICENSE): adds knockback to a melee hit, filtered on the target's entity
//! type tags. The game's knockback strength is half the attack knockback the original's formula works on, so the
//! formula runs over twice the strength and the result is halved again. A module that conditions the target on state this
//! port cannot read stays unbound, as for melee damage.
use crate::{melee_damage::TargetCondition, mining_speed::{StatFormula, ToolView}, modifier_entries::Entry};
use pluto_lib::Identifier;
use serde_json::{Map, Value};

/// FormulaLoadable(ADD, "level", "knockback"): the two unused argument slots take names no formula can write.
const NAMES: [&str; 4] = ["level", "knockback", "\u{0}multiplier", "\u{0}base"];

/// ProjectilePredicate over whether the hit is a projectile's.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Allow { Always, Projectile, Melee }

impl Allow {
    fn test(self, projectile: bool) -> bool { self == Self::Always || projectile == (self == Self::Projectile) }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Knockback { target: TargetCondition, formula: StatFormula, allow: Allow }

impl Knockback {
    pub fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        let allow = match fields.get("allow") {
            None | Some(Value::Null) => Allow::Always,
            Some(Value::String(text)) => match text.as_str() {
                "always" => Allow::Always,
                "projectile" => Allow::Projectile,
                "melee" => Allow::Melee,
                _ => return Err(format!("unknown allow {text}")),
            },
            Some(_) => return Err("allow must be a string".into()),
        };
        let Some(target) = TargetCondition::decode(fields.get("entity"))? else { return Ok(None); };
        Ok(StatFormula::decode(fields, &NAMES)?.map(|formula| Self { target, formula, allow }))
    }
    /// MeleeHitModifierHook.beforeMeleeHit: the strength after the module, for a hit that is a projectile's or not, on a
    /// target whose entity type tags `target_in_tag` answers.
    pub fn apply(&self, entry: &Entry, condition: bool, tool: &ToolView, strength: f32, projectile: bool, target_in_tag: &dyn Fn(&Identifier) -> bool) -> f32 {
        if !condition || !self.allow.test(projectile) || !self.target.matches(target_in_tag) { return strength; }
        self.formula.eval(entry, tool, strength * 2.0, 1.0, 0.0) * 0.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn module(value: Value) -> Knockback { Knockback::decode(value.as_object().unwrap()).unwrap().unwrap() }
    fn tool() -> pluto_lib::NbtCompound { pluto_lib::NbtCompound::new() }

    #[test]
    fn each_level_adds_to_the_attack_knockback_and_half_of_it_to_the_strength() {
        let nbt = tool();
        let entry = Entry::new(Identifier::parse("tconstruct:springy").unwrap(), 2);
        let springy = module(json!({"each_level": 0.25}));
        let strength = springy.apply(&entry, true, &ToolView::of(&nbt), 0.5, false, &|_| false);
        assert!((strength - 0.75).abs() < 1e-6, "{strength}");
    }

    #[test]
    fn a_projectile_only_module_leaves_a_melee_hit_alone() {
        let nbt = tool();
        let entry = Entry::new(Identifier::parse("tconstruct:knockback").unwrap(), 1);
        let knockback = module(json!({"allow": "projectile", "each_level": 0.5}));
        assert_eq!(knockback.apply(&entry, true, &ToolView::of(&nbt), 0.5, false, &|_| false), 0.5);
        assert!(knockback.apply(&entry, true, &ToolView::of(&nbt), 0.5, true, &|_| false) > 0.5);
        assert_eq!(module(json!({"allow": "melee", "each_level": 0.5})).apply(&entry, false, &ToolView::of(&nbt), 0.5, false, &|_| false), 0.5, "a failed condition changes nothing");
    }

    #[test]
    fn the_entity_filter_follows_the_target_tags() {
        let nbt = tool();
        let entry = Entry::new(Identifier::parse("tconstruct:knockback").unwrap(), 1);
        let filtered = module(json!({"entity": {"type": "mantle:tag", "tag": "minecraft:undead"}, "flat": 1.0}));
        assert_eq!(filtered.apply(&entry, true, &ToolView::of(&nbt), 0.5, false, &|_| false), 0.5);
        assert_eq!(filtered.apply(&entry, true, &ToolView::of(&nbt), 0.5, false, &|_| true), 1.0);
    }

    #[test]
    fn an_unknown_filter_or_allow_is_not_bound() {
        assert!(Knockback::decode(json!({"entity": {"type": "mantle:on_fire"}, "flat": 1.0}).as_object().unwrap()).unwrap().is_none());
        assert!(Knockback::decode(json!({"allow": "sometimes", "flat": 1.0}).as_object().unwrap()).is_err());
    }
}
