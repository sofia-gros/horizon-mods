//! ConditionalMeleeDamageModule, from SlimeKnights (MIT; see LICENSE), for the modules that depend only on the
//! tool: a module that conditions on the target or the attacker (mob type, sprinting, effects, biome) stays
//! unbound until PlutoLib reports that state, rather than being half-installed.
use crate::{mining_speed::{StatFormula, ToolView}, modifier_entries::Entry, tool_stats::Stat};
use serde_json::{Map, Value};

/// MeleeFormula.VARIABLES.
const NAMES: [&str; 4] = ["level", "damage", "multiplier", "base_damage"];

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MeleeDamage { formula: StatFormula }

impl MeleeDamage {
    /// `None` when the module names a target or attacker condition or a variable this port does not hold.
    pub fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        if ["target", "attacker"].iter().any(|key| fields.get(*key).is_some_and(|value| !value.is_null())) { return Ok(None); }
        Ok(StatFormula::decode(fields, &NAMES)?.map(|formula| Self { formula }))
    }
    /// A percent module runs later in the original (priority 75).
    pub fn percent(&self) -> bool { self.formula.percent() }
    /// MeleeDamageModifierHook.getMeleeDamage: `base` is the attack damage before modifiers, `damage` the running value.
    pub fn apply(&self, entry: &Entry, condition: bool, tool: &ToolView, base: f32, damage: f32) -> f32 {
        if !condition { return damage; }
        self.formula.eval(entry, tool, damage, tool.multiplier(Stat::AttackDamage), base)
    }
}
