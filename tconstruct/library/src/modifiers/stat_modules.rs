//! Modifier modules that change a tool's stats: `StatBoostModule` (`tconstruct:stat_boost`) and the pieces it needs (`LevelingValue`,
//! `StatOperation`, the `tool` and `modifier_level` conditions).
use std::any::Any;
use std::sync::Arc;

use serde_json::{Map, Value};

use super::{ModifierEntry, ModifierModule, ModuleLoader};
use crate::predicate;
use crate::tools::stat::{ModifierStatsBuilder, ToolStatId, ToolStats};

/// `LevelingValue`: a flat amount plus an amount for each level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LevelingValue {
    pub flat: f32,
    pub each_level: f32,
}

impl LevelingValue {
    /// `LevelingValue.LOADABLE`: `flat` and `each_level`, both 0 when missing.
    pub fn from_json(object: &Map<String, Value>) -> Result<Self, String> {
        let number = |key: &str| match object.get(key) {
            None => Ok(0.0),
            Some(value) => value.as_f64().map(|v| v as f32).ok_or_else(|| format!("'{key}' must be a number")),
        };
        Ok(Self { flat: number("flat")?, each_level: number("each_level")? })
    }

    /// `compute(level)`.
    pub fn compute(&self, level: f32) -> f32 {
        self.flat + self.each_level * level
    }
}

/// `StatBoostModule.StatOperation`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatOperation {
    Add,
    Percent,
    MultiplyBase,
    MultiplyConditional,
    MultiplyAll,
}

impl StatOperation {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "add" => Some(Self::Add),
            "percent" => Some(Self::Percent),
            "multiply_base" => Some(Self::MultiplyBase),
            "multiply_conditional" => Some(Self::MultiplyConditional),
            "multiply_all" => Some(Self::MultiplyAll),
            _ => None,
        }
    }

    /// `apply`.
    pub fn apply(self, builder: &mut ModifierStatsBuilder<'_>, stat: &ToolStatId, value: f32) {
        let value = f64::from(value);
        match self {
            Self::Add => builder.add(stat, value),
            Self::Percent => builder.percent(stat, value),
            Self::MultiplyBase => builder.multiply(stat, 1.0 + value),
            Self::MultiplyConditional => builder.add_multiplier(stat, 1.0 + value),
            Self::MultiplyAll => builder.multiply_all(stat, 1.0 + value),
        }
    }
}

/// What a module may ask about the tool it works on (`IToolContext`).
pub trait ToolContext {
    /// Whether the tool's item is in the item tag.
    fn item_in_tag(&self, tag: &str) -> bool;
}

/// `ModifierCondition` as `stat_boost` uses it: an optional `tool` predicate and an optional `modifier_level` range.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Condition {
    pub tool: Option<Value>,
    pub min_level: Option<i32>,
    pub max_level: Option<i32>,
}

impl Condition {
    pub fn from_json(object: &Map<String, Value>) -> Result<Self, String> {
        let mut condition = Self { tool: object.get("tool").cloned(), ..Self::default() };
        if let Some(range) = object.get("modifier_level") {
            let range = range.as_object().ok_or("'modifier_level' must be an object with 'min' and 'max'")?;
            condition.min_level = range.get("min").and_then(Value::as_i64).map(|v| v as i32);
            condition.max_level = range.get("max").and_then(Value::as_i64).map(|v| v as i32);
        }
        Ok(condition)
    }

    /// `matches(context, modifier)`.
    pub fn matches(&self, context: &dyn ToolContext, entry: &ModifierEntry) -> bool {
        if self.min_level.is_some_and(|min| entry.level < min) || self.max_level.is_some_and(|max| entry.level > max) {
            return false;
        }
        match &self.tool {
            None => true,
            Some(tool) => predicate::test(tool, &|kind, object| match kind {
                "mantle:tag" => Ok(object.get("tag").and_then(Value::as_str).is_some_and(|tag| context.item_in_tag(tag))),
                other => Err(format!("unsupported item predicate {other}")),
            })
            .unwrap_or(false),
        }
    }
}

/// `StatBoostModule`.
#[derive(Clone, Debug, PartialEq)]
pub struct StatBoostModule {
    pub stat: ToolStatId,
    pub operation: StatOperation,
    pub amount: LevelingValue,
    pub condition: Condition,
}

impl StatBoostModule {
    /// `addToolStats(context, modifier, builder)`; `supports` says whether the stat applies to the tool's item.
    pub fn add_tool_stats(&self, context: &dyn ToolContext, entry: &ModifierEntry, stats: &ToolStats, builder: &mut ModifierStatsBuilder<'_>) {
        let Some(stat) = stats.get(&self.stat) else { return };
        if self.condition.matches(context, entry) && stat.supports(&|tag| context.item_in_tag(tag)) {
            self.operation.apply(builder, &self.stat, self.amount.compute(entry.level as f32));
        }
    }
}

impl ModifierModule for StatBoostModule {
    fn kind(&self) -> &str {
        "tconstruct:stat_boost"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The loader of `tconstruct:stat_boost`; the stat must be a numeric one the registry knows.
pub fn stat_boost_loader(stats: Arc<ToolStats>) -> ModuleLoader {
    Arc::new(move |json| {
        let name = json.get("stat").and_then(Value::as_str).ok_or("stat_boost needs a 'stat'")?;
        let id = ToolStatId::try_parse(name).ok_or_else(|| format!("bad stat id '{name}'"))?;
        let stat = stats.get(&id).ok_or_else(|| format!("Unknown stat type {name}"))?;
        if !stat.is_numeric() {
            return Err(format!("Invalid tool stat {name}, must be a numeric stat"));
        }
        let operation = json.get("operation").and_then(Value::as_str).and_then(StatOperation::from_name).ok_or("stat_boost needs a valid 'operation'")?;
        Ok(Arc::new(StatBoostModule { stat: id, operation, amount: LevelingValue::from_json(json)?, condition: Condition::from_json(json)? }) as Arc<dyn ModifierModule>)
    })
}
