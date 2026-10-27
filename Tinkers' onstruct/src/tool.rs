use crate::{
    Catalog,
    catalog::{fields, number, text},
    stats::Stats,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

type Result<T> = std::result::Result<T, String>;

/// Authoritative core state, not a Minecraft item or a host slot handle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolState {
    pub schema: u32,
    pub tool: String,
    pub materials: [String; 3],
    pub damage: u32,
    pub broken: bool,
}

impl ToolState {
    pub fn pickaxe(materials: [&str; 3]) -> Self {
        Self {
            schema: 1,
            tool: "tconstruct:pickaxe".into(),
            materials: materials.map(|v| {
                if v.contains(':') {
                    v.into()
                } else {
                    format!("tconstruct:{v}")
                }
            }),
            damage: 0,
            broken: false,
        }
    }
    pub fn validate(&self) -> Result<()> {
        if self.schema != 1 {
            return Err(format!("unsupported tool schema {}", self.schema));
        }
        if self.tool != "tconstruct:pickaxe" {
            return Err(format!("unsupported tool {}", self.tool));
        }
        if self
            .materials
            .iter()
            .any(|v| v.len() > 128 || !v.starts_with("tconstruct:"))
        {
            return Err("invalid material identifier".into());
        }
        Ok(())
    }
    /// Raw damage points. Creative/unbreaking and host permissions are not simulated.
    pub fn damage_points(&mut self, catalog: &Catalog, points: u32) -> Result<()> {
        let max = catalog.evaluate(self)?.stats.durability()?;
        self.damage = self.damage.saturating_add(points).min(max);
        self.broken = self.damage == max;
        Ok(())
    }
    /// Apply the base wear for the currently supported pickaxe after a
    /// successful survival block break. Modifier hooks are not yet modeled.
    pub fn damage_after_mining(&mut self, catalog: &Catalog, hardness: f32) -> Result<()> {
        if !hardness.is_finite() {
            return Err("block hardness must be finite".into());
        }
        // Validate even when there is no wear; malformed states must not pass
        // through the transaction merely because the block breaks instantly.
        catalog.evaluate(self)?;
        if !self.broken && hardness != 0.0 {
            self.damage_points(catalog, 1)?;
        }
        Ok(())
    }
    /// Raw, already-resolved repair points; recipe costs/fractions remain future work.
    pub fn repair_points(&mut self, catalog: &Catalog, points: u32) -> Result<u32> {
        let max = catalog.evaluate(self)?.stats.durability()?;
        let repaired = self.damage.min(points);
        self.damage -= repaired;
        self.broken = self.damage == max;
        Ok(repaired)
    }
    pub fn to_json(&self) -> Result<String> {
        self.validate()?;
        serde_json::to_string(self).map_err(|e| e.to_string())
    }
    pub fn from_json(catalog: &Catalog, source: &str) -> Result<Self> {
        if source.len() > 4096 {
            return Err("tool state exceeds 4096 bytes".into());
        }
        let state: Self = serde_json::from_str(source).map_err(|e| e.to_string())?;
        catalog.evaluate(&state)?;
        Ok(state)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ModifierLevel {
    pub id: String,
    pub level: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Evaluation {
    pub stats: Stats,
    pub modifiers: Vec<ModifierLevel>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HitEffect {
    pub effect: String,
    pub level: f32,
    pub ticks: f32,
    pub before_melee: bool,
}

impl Evaluation {
    pub fn repair_factor(&self, catalog: &Catalog) -> Result<f32> {
        let mut factor = 1.0_f32;
        for entry in &self.modifiers {
            for module in catalog.modifier_modules(&entry.id)? {
                if module["type"] == "tconstruct:repair" {
                    factor *= 1.0
                        + number(&module["each_level"], "repair/each_level")? * entry.level as f32;
                }
            }
        }
        Ok(factor)
    }

    pub fn mining_speed(
        &self,
        catalog: &Catalog,
        state: &ToolState,
        effective: bool,
    ) -> Result<f32> {
        self.conditional(
            catalog,
            state,
            "tconstruct:conditional_mining_speed",
            self.stats.mining_speed,
            effective,
        )
    }
    pub fn melee_damage(&self, catalog: &Catalog, state: &ToolState) -> Result<f32> {
        self.conditional(
            catalog,
            state,
            "tconstruct:conditional_melee_damage",
            self.stats.attack_damage,
            true,
        )
    }
    fn conditional(
        &self,
        catalog: &Catalog,
        state: &ToolState,
        kind: &str,
        mut value: f32,
        effective: bool,
    ) -> Result<f32> {
        // Do not allow a cached evaluation from another tool/state to drive hooks.
        if catalog.evaluate(state)? != *self {
            return Err("stale or mismatched evaluation".into());
        }
        if state.broken {
            return Ok(0.0);
        }
        for entry in &self.modifiers {
            for module in catalog.modifier_modules(&entry.id)? {
                if module["type"] == kind
                    && (!module["require_effective"].as_bool().unwrap_or(true) || effective)
                {
                    // In upstream stat_multiplier is the GLOBAL MULTIPLIER (1 in
                    // this subset), NOT max durability. Preserve this distinction.
                    value = formula(&module["formula"], |name| match name {
                        "$level" => Ok(entry.level as f32),
                        "$lost" => Ok(state.damage as f32),
                        "$max" => Ok(1.0),
                        "$speed" | "$damage" => Ok(value),
                        other => Err(format!("unsupported formula variable {other}")),
                    })?;
                }
            }
        }
        Ok(value)
    }

    /// Declarative effect only. The current client diagnostic does not apply it.
    pub fn hit_effects(&self, catalog: &Catalog) -> Result<Vec<HitEffect>> {
        let mut effects = Vec::new();
        for entry in &self.modifiers {
            for module in catalog.modifier_modules(&entry.id)? {
                if module["type"] == "tconstruct:weapon_mob_effect" {
                    effects.push(HitEffect {
                        effect: text(&module["effect"], "effect")?.into(),
                        level: number(&module["level"]["per_level"], "effect/level")?
                            * entry.level as f32,
                        ticks: number(&module["time"]["flat"], "effect/time")?,
                        before_melee: module["apply_before_melee"]
                            .as_bool()
                            .ok_or("expected effect phase")?,
                    });
                }
            }
        }
        Ok(effects)
    }
}

pub(crate) fn validate_modifier_module(module: &Value) -> Result<()> {
    let kind = text(&module["type"], "modifier/type")?;
    match kind {
        "tconstruct:repair" => {
            fields(module, &["type", "each_level"], kind)?;
            number(&module["each_level"], kind)?;
        }
        "tconstruct:stat_boost" => {
            fields(module, &["type", "each_level", "operation", "stat"], kind)?;
            if module["operation"] != "add" || module["stat"] != "tconstruct:attack_damage" {
                return Err("unsupported stat boost configuration".into());
            }
            number(&module["each_level"], kind)?;
        }
        "tconstruct:conditional_mining_speed" | "tconstruct:conditional_melee_damage" => {
            fields(
                module,
                &[
                    "type",
                    "formula",
                    "percent",
                    "require_effective",
                    "variables",
                ],
                kind,
            )?;
            if module["percent"] != true {
                return Err("unsupported conditional priority".into());
            }
            if let Some(v) = module.get("require_effective")
                && !v.is_boolean()
            {
                return Err("expected require_effective boolean".into());
            }
            let vars = serde_json::json!({"lost": "tconstruct:tool_lost_durability", "max": {"type": "tconstruct:stat_multiplier", "stat": "tconstruct:durability"}});
            if module["variables"] != vars {
                return Err("unsupported conditional variables".into());
            }
            formula(&module["formula"], |name| match name {
                "$level" | "$lost" | "$max" | "$speed" | "$damage" => Ok(1.0),
                other => Err(format!("unsupported variable {other}")),
            })?;
        }
        "tconstruct:weapon_mob_effect" => {
            fields(
                module,
                &["type", "apply_before_melee", "effect", "level", "time"],
                kind,
            )?;
            if module["effect"] != "tconstruct:pierce" || module["apply_before_melee"] != true {
                return Err("unsupported hit effect".into());
            }
            fields(&module["level"], &["per_level"], kind)?;
            fields(&module["time"], &["flat"], kind)?;
            number(&module["level"]["per_level"], kind)?;
            number(&module["time"]["flat"], kind)?;
        }
        _ => return Err(format!("unsupported modifier module {kind}")),
    }
    Ok(())
}

/// Bounded postfix interpreter for the selected upstream conditional formulas.
pub fn formula(source: &Value, mut variable: impl FnMut(&str) -> Result<f32>) -> Result<f32> {
    let tokens = source.as_array().ok_or("formula must be an array")?;
    if tokens.is_empty() || tokens.len() > 64 {
        return Err("formula length must be 1..64".into());
    }
    let mut stack = Vec::<f32>::new();
    for token in tokens {
        if token.is_number() {
            stack.push(number(token, "formula")?);
        } else {
            let word = text(token, "formula token")?;
            if word.starts_with('$') {
                stack.push(variable(word)?);
            } else if word == "sqrt" {
                let x = stack.pop().ok_or("formula stack underflow")?;
                stack.push((f64::from(x).sqrt()) as f32);
            } else if ["+", "-", "*", "/"].contains(&word) {
                let right = stack.pop().ok_or("formula stack underflow")?;
                let left = stack.pop().ok_or("formula stack underflow")?;
                stack.push(match word {
                    "+" => left + right,
                    "-" => left - right,
                    "*" => left * right,
                    _ => {
                        if right == 0.0 {
                            0.0
                        } else {
                            left / right
                        }
                    }
                });
            } else {
                return Err(format!("unsupported formula operator {word}"));
            }
        }
        if stack.last().is_some_and(|v| !v.is_finite()) {
            return Err("formula produced non-finite value".into());
        }
    }
    if stack.len() != 1 {
        return Err("formula must leave exactly one result".into());
    }
    Ok(stack[0])
}
