//! ConditionalMiningSpeedModule and its block, holder and variable inputs, from SlimeKnights
//! (MIT; see LICENSE). A module changes the dig speed of a block as PlutoLib's break-speed event
//! reports it: the block and its tags, the holder's ground and water state, and the facts of the
//! dig itself — the light at the block and the temperature of its biome — which the block light
//! and biome temperature variables read. Inputs over data this port does not hold stay unbound
//! instead of being half-installed: a block property variable (`tconstruct:block` over blast
//! resistance or hardness — PlutoLib reports no block properties), the `mantle:requires_tool`
//! block predicate (the same missing block property: whether the block needs the correct tool
//! for drops), and a variable or condition over the holder as an entity beyond ground and water
//! (the break-speed event reports no holder state to read one from, and `mantle:has_enchantment`
//! — the shipped hydraulic's other branch — has no surface anywhere in the port yet).
use crate::{formula::Formula, modifier_entries::{nbt_int, Entry}, tool_stats::Stat};
use pluto_lib::{Identifier, Nbt, NbtCompound};
use serde_json::{Map, Value};

/// IJsonPredicate over the block being dug.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BlockCondition { Any, Tag(Identifier), Set(Vec<Identifier>), Not(Box<BlockCondition>), And(Vec<BlockCondition>), Or(Vec<BlockCondition>) }

/// IJsonPredicate over the player who digs: only ground and water state is known.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum HolderCondition { Any, OnGround, EyesInWater, Not(Box<HolderCondition>), And(Vec<HolderCondition>), Or(Vec<HolderCondition>) }

/// MiningSpeedVariable, limited to the tool-based inputs.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Variable {
    /// IToolStackView.getCurrentDurability.
    CurrentDurability,
    /// IToolStackView.getDamage.
    LostDurability,
    /// The tool's stored value for a stat.
    ToolStat(Stat),
    /// IToolStackView.getMultiplier.
    StatMultiplier(Stat),
    /// A float from the tool's persistent or volatile data (0 when absent).
    ModData { key: String, persistent: bool },
    /// EntityMeleeVariable and its relatives: a fact of the attacker (the holder) or the target of the hit, `fallback` when it was not reported.
    Entity { fact: crate::entity_facts::EntityFact, whose: crate::entity_facts::Whose, fallback: f32 },
    /// BlockLightVariable: the light at the block being dug, in one layer or both (the larger), the sky's with the time of
    /// day and the weather taken off, as `EntityLightVariable.getLightLevel` works it out; `fallback` when the game did not
    /// report it. The original reads the light at the block on the side that was hit; the break-speed event names no side,
    /// so this port reads the dug block's own light.
    BlockLight { layer: Option<crate::entity_facts::LightLayer>, fallback: f32 },
    /// BlockTemperatureVariable: the temperature of the biome at the block being dug; `fallback` when it was not reported.
    BlockTemperature { fallback: f32 },
}

/// Which modules read entity variables, and so where `tconstruct:entity` variables are decoded.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum EntityScope {
    /// No entities are known: an entity variable stays unbound.
    None,
    /// A melee hit: `which` names the attacker or the target.
    Hit,
    /// The holder of a tool: no `which`; the holder is the attacker of the state.
    Holder,
}

#[derive(Clone, Debug, PartialEq)]
enum Shape { Boost { flat: f32, each: f32 }, Percent { flat: f32, each: f32 }, PostFix(Formula, Vec<Variable>) }

/// A stat module's formula (VariableFormula over `level`, the running value, `multiplier` and the base value):
/// either the simple boost/percent fallback or a post fix formula with tool variables. Shared by the
/// conditional mining speed and melee damage modules.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StatFormula { percent: bool, shape: Shape, arity: usize }

impl StatFormula {
    /// `names` are the four default variable names; `None` when a variable names an input this port does not hold.
    pub fn decode(fields: &Map<String, Value>, names: &[&str]) -> Result<Option<Self>, String> { Self::decode_in(fields, names, EntityScope::None) }
    /// As `decode`, with the entity variables `scope` allows.
    pub fn decode_in(fields: &Map<String, Value>, names: &[&str], scope: EntityScope) -> Result<Option<Self>, String> {
        let percent = match fields.get("percent") { None | Some(Value::Null) => false, Some(Value::Bool(value)) => *value, Some(_) => return Err("percent must be boolean".into()) };
        let shape = if let Some(variables) = fields.get("variables") {
            let formula = fields.get("formula").ok_or("Cannot set variables when not using formula")?;
            let variables = variables.as_object().ok_or("variables must be an object")?;
            let mut all: Vec<&str> = names.to_vec();
            let mut decoded = Vec::new();
            for (name, value) in variables {
                if names.contains(&name.as_str()) { return Err(format!("Variable {name} is already defined for this module")); }
                match Variable::decode_in(value, scope)? { Some(variable) => decoded.push(variable), None => return Ok(None) }
                all.push(name);
            }
            Shape::PostFix(Formula::decode(formula, &all)?, decoded)
        } else if let Some(formula) = fields.get("formula") {
            Shape::PostFix(Formula::decode(formula, names)?, Vec::new())
        } else if percent {
            Shape::Percent { flat: float(fields, "flat")?, each: float(fields, "each_level")? }
        } else {
            Shape::Boost { flat: float(fields, "flat")?, each: float(fields, "each_level")? }
        };
        Ok(Some(Self { percent, shape, arity: names.len() }))
    }
    /// A percent module runs later in the original (priority 75).
    pub fn percent(&self) -> bool { self.percent }
    /// The value after the formula: `value` is the running value, `base` the value before any modifier.
    pub fn eval(&self, entry: &Entry, tool: &ToolView, value: f32, multiplier: f32, base: f32) -> f32 {
        match &self.shape {
            Shape::Boost { flat, each } => value + (flat + each * entry.effective_level()) * multiplier,
            Shape::Percent { flat, each } => value * (1.0 + (flat + each * entry.effective_level())),
            Shape::PostFix(formula, variables) => {
                // Three names (level, value, multiplier) for the modules that have no base value, four otherwise.
                let mut arguments = vec![entry.effective_level(), value, multiplier, base];
                arguments.truncate(self.arity);
                arguments.extend(variables.iter().map(|variable| variable.value(tool)));
                formula.apply(&arguments).unwrap_or(value)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MiningSpeed { blocks: BlockCondition, holder: HolderCondition, require_effective: bool, formula: StatFormula }

/// What the world reports about the block being dug: the light at it and the temperature of its biome, as the
/// break-speed event carries them. A fact the game did not report is None, and a variable over it takes its fallback.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct BlockFacts {
    pub block_light: Option<u8>,
    pub sky_light: Option<u8>,
    /// How much of the sky light the time of day and the weather take off (`Level.getSkyDarken`).
    pub sky_darken: u8,
    pub biome_temperature: Option<f32>,
}

/// What the dig is, for one evaluation.
pub(crate) struct Context<'a> {
    pub block: &'a Identifier,
    pub block_in_tag: &'a dyn Fn(&Identifier) -> bool,
    /// The speed vanilla computes, before modifiers.
    pub original: f32,
    /// BreakSpeedContext.miningSpeedMultiplier: haste, fatigue, water and air as vanilla applied them.
    pub multiplier: f32,
    /// IsEffectiveToolHook for the block.
    pub effective: bool,
    pub on_ground: Option<bool>,
    pub eyes_in_water: Option<bool>,
    /// The facts of the dig itself, for the formula's block variables.
    pub facts: BlockFacts,
}

/// Tool state a variable reads.
pub(crate) struct ToolView<'a> { pub nbt: &'a NbtCompound, pub entities: crate::entity_facts::Entities<'a>, pub facts: BlockFacts }
impl<'a> ToolView<'a> {
    /// A view of a tool with no entities known.
    pub fn of(nbt: &'a NbtCompound) -> Self { Self { nbt, entities: crate::entity_facts::Entities::default(), facts: BlockFacts::default() } }
    /// A view of a tool in the hands of `entities.attacker`, over `entities.target`.
    pub fn with(nbt: &'a NbtCompound, entities: crate::entity_facts::Entities<'a>) -> Self { Self { nbt, entities, facts: BlockFacts::default() } }
}
impl ToolView<'_> {
    /// IToolStackView.getMultiplier.
    pub fn multiplier(&self, stat: Stat) -> f32 { Variable::StatMultiplier(stat).value(self) }
}

fn float(fields: &Map<String, Value>, key: &str) -> Result<f32, String> {
    match fields.get(key) {
        None | Some(Value::Null) => Ok(0.0),
        Some(Value::Number(number)) => number.as_f64().map(|value| value as f32).ok_or_else(|| format!("{key} must be a number")),
        Some(_) => Err(format!("{key} must be a number")),
    }
}
fn identifier(value: &Value) -> Result<Identifier, String> { Identifier::parse(value.as_str().ok_or("expected an ID string")?).map_err(|e| e.to_string()) }

impl BlockCondition {
    pub(crate) fn decode(value: &Value) -> Result<Option<Self>, String> {
        if let Value::String(tag) = value { return Ok(Some(Self::Tag(Identifier::parse(tag).map_err(|e| e.to_string())?))); }
        let object = value.as_object().ok_or("block predicate must be an object or a tag")?;
        let kind = object.get("type").and_then(Value::as_str).ok_or("block predicate type missing")?;
        let list = |key: &str| -> Result<Option<Vec<Self>>, String> {
            let mut out = Vec::new();
            for child in object.get(key).and_then(Value::as_array).ok_or_else(|| format!("{key} must be an array"))? {
                match Self::decode(child)? { Some(child) => out.push(child), None => return Ok(None) }
            }
            Ok(Some(out))
        };
        Ok(Some(match kind {
            "mantle:any" => Self::Any,
            "mantle:tag" => Self::Tag(identifier(object.get("tag").ok_or("tag missing")?)?),
            "mantle:set" => {
                let blocks = object.get("blocks").and_then(Value::as_array).ok_or("blocks must be an array")?;
                Self::Set(blocks.iter().map(identifier).collect::<Result<_, _>>()?)
            }
            "mantle:inverted" => {
                let inner = Self::decode(&Value::Object(retyped(object, "inverted_type")?))?;
                match inner { Some(inner) => Self::Not(Box::new(inner)), None => return Ok(None) }
            }
            "mantle:and" => match list("predicates")? { Some(children) => Self::And(children), None => return Ok(None) },
            "mantle:or" => match list("predicates")? { Some(children) => Self::Or(children), None => return Ok(None) },
            _ => return Ok(None),
        }))
    }
    pub(crate) fn matches(&self, block: &Identifier, in_tag: &dyn Fn(&Identifier) -> bool) -> bool {
        match self {
            Self::Any => true,
            Self::Tag(tag) => in_tag(tag),
            Self::Set(blocks) => blocks.contains(block),
            Self::Not(inner) => !inner.matches(block, in_tag),
            Self::And(children) => children.iter().all(|child| child.matches(block, in_tag)),
            Self::Or(children) => children.iter().any(|child| child.matches(block, in_tag)),
        }
    }
}

/// A mantle `inverted` predicate names its inner type under `inverted_type` with the same fields beside it.
fn retyped(object: &Map<String, Value>, key: &str) -> Result<Map<String, Value>, String> {
    let mut inner = object.clone();
    let kind = inner.remove(key).ok_or_else(|| format!("{key} missing"))?;
    inner.insert("type".into(), kind);
    Ok(inner)
}

impl HolderCondition {
    fn decode(value: Option<&Value>) -> Result<Option<Self>, String> {
        let Some(value) = value.filter(|value| !value.is_null()) else { return Ok(Some(Self::Any)); };
        let object = value.as_object().ok_or("entity predicate must be an object")?;
        let kind = object.get("type").and_then(Value::as_str).ok_or("entity predicate type missing")?;
        let list = |key: &str| -> Result<Option<Vec<Self>>, String> {
            let mut out = Vec::new();
            for child in object.get(key).and_then(Value::as_array).ok_or_else(|| format!("{key} must be an array"))? {
                match Self::decode(Some(child))? { Some(child) => out.push(child), None => return Ok(None) }
            }
            Ok(Some(out))
        };
        Ok(Some(match kind {
            "mantle:any" => Self::Any,
            "mantle:on_ground" => Self::OnGround,
            "mantle:eyes_in_water" => Self::EyesInWater,
            "mantle:inverted" => match Self::decode(Some(&Value::Object(retyped(object, "inverted_type")?)))? { Some(inner) => Self::Not(Box::new(inner)), None => return Ok(None) },
            "mantle:and" => match list("predicates")? { Some(children) => Self::And(children), None => return Ok(None) },
            "mantle:or" => match list("predicates")? { Some(children) => Self::Or(children), None => return Ok(None) },
            _ => return Ok(None),
        }))
    }
    /// An unknown ground or water state does not match the condition it would decide.
    fn matches(&self, context: &Context) -> bool {
        match self {
            Self::Any => true,
            Self::OnGround => context.on_ground == Some(true),
            Self::EyesInWater => context.eyes_in_water == Some(true),
            Self::Not(inner) => !inner.matches(context),
            Self::And(children) => children.iter().all(|child| child.matches(context)),
            Self::Or(children) => children.iter().any(|child| child.matches(context)),
        }
    }
}

impl Variable {
    fn decode_in(value: &Value, scope: EntityScope) -> Result<Option<Self>, String> {
        let kind = match value {
            Value::String(kind) => kind.as_str(),
            Value::Object(object) => object.get("type").and_then(Value::as_str).ok_or("variable type missing")?,
            _ => return Err("variable must be a type or an object".into()),
        };
        let object = value.as_object();
        let stat = || -> Result<Stat, String> {
            let id = identifier(object.and_then(|object| object.get("stat")).ok_or("stat missing")?)?;
            Stat::from_id(&id).ok_or_else(|| format!("unsupported stat {id}"))
        };
        Ok(Some(match kind {
            "tconstruct:tool_durability" => Self::CurrentDurability,
            "tconstruct:tool_lost_durability" => Self::LostDurability,
            "tconstruct:tool_stat" => match stat() { Ok(stat) => Self::ToolStat(stat), Err(_) => return Ok(None) },
            "tconstruct:stat_multiplier" => match stat() { Ok(stat) => Self::StatMultiplier(stat), Err(_) => return Ok(None) },
            "tconstruct:mod_data" => {
                let object = object.ok_or("mod_data needs fields")?;
                let key = identifier(object.get("key").ok_or("key missing")?)?.to_string();
                let persistent = match object.get("source").and_then(Value::as_str) { Some("persistent") => true, Some("volatile") => false, _ => return Ok(None) };
                Self::ModData { key, persistent }
            }
            "tconstruct:block_light" => {
                let object = object.ok_or("block light variable needs fields")?;
                let fallback = object.get("fallback").and_then(Value::as_f64).ok_or("fallback missing")? as f32;
                let layer = match object.get("light_layer").filter(|value| !value.is_null()).map(Value::as_str) {
                    None => None,
                    Some(Some("sky")) => Some(crate::entity_facts::LightLayer::Sky),
                    Some(Some("block")) => Some(crate::entity_facts::LightLayer::Block),
                    Some(_) => return Ok(None),
                };
                Self::BlockLight { layer, fallback }
            }
            "tconstruct:biome_temperature" => {
                let object = object.ok_or("biome temperature variable needs fields")?;
                let fallback = object.get("fallback").and_then(Value::as_f64).ok_or("fallback missing")? as f32;
                Self::BlockTemperature { fallback }
            }
            "tconstruct:entity" if scope != EntityScope::None => {
                use crate::entity_facts::{EntityFact, Whose};
                let object = object.ok_or("entity variable needs fields")?;
                let whose = match (scope, object.get("which").and_then(Value::as_str)) {
                    (EntityScope::Holder, None) | (_, Some("attacker")) => Whose::Attacker,
                    (EntityScope::Hit, Some("target")) => Whose::Target,
                    (EntityScope::Hit, None) => return Err("which missing".into()),
                    _ => return Ok(None),
                };
                let fallback = object.get("fallback").and_then(Value::as_f64).ok_or("fallback missing")? as f32;
                let role = if whose == Whose::Attacker { "attacker" } else { "target" };
                // The variable's own fields (effect, attribute, condition) sit beside its `entity_type`.
                let fact = match object.get("entity_type").ok_or("entity_type missing")? {
                    Value::String(kind) => EntityFact::decode(kind, object, role)?,
                    number => EntityFact::decode_value(number, role)?,
                };
                match fact { Some(fact) => Self::Entity { fact, whose, fallback }, None => return Ok(None) }
            }
            _ => return Ok(None),
        }))
    }
    fn value(&self, tool: &ToolView) -> f32 {
        let compound = |tag: &str| match tool.nbt.get(tag) { Some(Nbt::Compound(data)) => Some(data.clone()), _ => None };
        match self {
            Self::CurrentDurability => (crate::tool_stack::durability(tool.nbt) - crate::tool_stack::damage(tool.nbt)).max(0) as f32,
            Self::LostDurability => crate::tool_stack::damage(tool.nbt) as f32,
            Self::ToolStat(stat) => crate::tool_stack::stat(tool.nbt, *stat),
            Self::StatMultiplier(stat) => match compound(crate::tool_stack::TAG_MULTIPLIERS).and_then(|data| data.get(&format!("tconstruct:{}", stat.path())).cloned()) {
                Some(Nbt::Float(value)) => value,
                Some(other) => nbt_int(Some(&other)) as f32,
                None => 1.0,
            },
            Self::Entity { fact, whose, fallback } => fact.value(&tool.entities, *whose).unwrap_or(*fallback),
            Self::BlockLight { layer, fallback } => {
                let sky = tool.facts.sky_light.map(|light| (i32::from(light) - i32::from(tool.facts.sky_darken)).max(0));
                let block = tool.facts.block_light.map(i32::from);
                match layer { Some(crate::entity_facts::LightLayer::Sky) => sky, Some(crate::entity_facts::LightLayer::Block) => block, None => sky.zip(block).map(|(sky, block)| sky.max(block)) }.map(|light| light as f32).unwrap_or(*fallback)
            }
            Self::BlockTemperature { fallback } => tool.facts.biome_temperature.unwrap_or(*fallback),
            Self::ModData { key, persistent } => {
                let tag = if *persistent { crate::tool_stack::TAG_PERSISTENT } else { crate::tool_stack::TAG_VOLATILE };
                match compound(tag).and_then(|data| data.get(key).cloned()) {
                    Some(Nbt::Float(value)) => value,
                    Some(Nbt::Double(value)) => value as f32,
                    Some(other) => nbt_int(Some(&other)) as f32,
                    None => 0.0,
                }
            }
        }
    }
}

/// MiningSpeedFormula.VARIABLES.
const NAMES: [&str; 4] = ["level", "speed", "multiplier", "original_speed"];

impl MiningSpeed {
    /// `None` when the module names an input this port does not hold.
    pub fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        let blocks = match fields.get("blocks").filter(|value| !value.is_null()) { None => BlockCondition::Any, Some(value) => match BlockCondition::decode(value)? { Some(blocks) => blocks, None => return Ok(None) } };
        let Some(holder) = HolderCondition::decode(fields.get("entity"))? else { return Ok(None); };
        let require_effective = match fields.get("require_effective") { None | Some(Value::Null) => true, Some(Value::Bool(value)) => *value, Some(_) => return Err("require_effective must be boolean".into()) };
        let Some(formula) = StatFormula::decode(fields, &NAMES)? else { return Ok(None); };
        Ok(Some(Self { blocks, holder, require_effective, formula }))
    }

    /// Whether the module is a percent module, which the original runs later (priority 75).
    pub fn percent(&self) -> bool { self.formula.percent() }

    /// BreakSpeedModifierHook.modifyBreakSpeed. `condition` is the module's ModifierCondition result.
    pub fn apply(&self, entry: &Entry, condition: bool, tool: &ToolView, context: &Context, speed: f32) -> f32 {
        if (self.require_effective && !context.effective) || !condition || !self.blocks.matches(context.block, context.block_in_tag) || !self.holder.matches(context) { return speed; }
        let tool_multiplier = Variable::StatMultiplier(Stat::MiningSpeed).value(tool);
        let multiplier = context.multiplier * tool_multiplier;
        // The formula's block variables read the facts of this dig, over the tool's own state.
        let tool = &ToolView { nbt: tool.nbt, entities: tool.entities, facts: context.facts };
        self.formula.eval(entry, tool, speed, multiplier, context.original)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fields(value: Value) -> Map<String, Value> { value.as_object().unwrap().clone() }

    #[test]
    fn block_variables_read_the_facts_of_the_dig() {
        let nbt = NbtCompound::new();
        let mut view = ToolView::of(&nbt);
        view.facts = BlockFacts { block_light: Some(9), sky_light: Some(15), sky_darken: 11, biome_temperature: Some(0.8) };
        let read = |value: &Value| Variable::decode_in(value, EntityScope::None).unwrap().unwrap().value(&view);
        assert_eq!(read(&json!({"type": "tconstruct:block_light", "fallback": 15.0, "light_layer": "block"})), 9.0);
        assert_eq!(read(&json!({"type": "tconstruct:block_light", "fallback": 15.0, "light_layer": "sky"})), 4.0, "15 less the 11 the night takes");
        assert_eq!(read(&json!({"type": "tconstruct:block_light", "fallback": 15.0})), 9.0, "no layer: the brighter of the two");
        assert_eq!(read(&json!({"type": "tconstruct:biome_temperature", "fallback": -0.5})), 0.8);
        // Nothing reported: the fallbacks stand in, as the original's do without an event.
        let unreported = ToolView::of(&nbt);
        let read = |value: &Value| Variable::decode_in(value, EntityScope::None).unwrap().unwrap().value(&unreported);
        assert_eq!(read(&json!({"type": "tconstruct:block_light", "fallback": 15.0, "light_layer": "block"})), 15.0);
        assert_eq!(read(&json!({"type": "tconstruct:biome_temperature", "fallback": -0.5})), -0.5);
    }

    #[test]
    fn the_shipped_temperate_and_lightspeed_modules_decode_and_the_block_property_forms_do_not() {
        // temperate: |temperature - 0.75| * level * 6 * multiplier over the speed.
        let temperate = MiningSpeed::decode(&fields(json!({
            "formula": ["$temperature", 0.75, "!-", "non-negative", "$level", "*", 6.0, "*", "$multiplier", "*", "$speed", "+"],
            "percent": false, "require_effective": true,
            "variables": {"temperature": {"type": "tconstruct:biome_temperature", "fallback": -0.5}},
        }))).unwrap().unwrap();
        let block = Identifier::parse("minecraft:stone").unwrap();
        let no_tags = |_: &Identifier| false;
        let nbt = NbtCompound::new();
        let view = ToolView::of(&nbt);
        let entry = Entry::new(Identifier::parse("tconstruct:temperate").unwrap(), 1);
        let dig = |temperature: Option<f32>| Context { block: &block, block_in_tag: &no_tags, original: 2.0, multiplier: 1.0, effective: true, on_ground: None, eyes_in_water: None, facts: BlockFacts { biome_temperature: temperature, ..BlockFacts::default() } };
        // A cold biome (0.0): |0 - 0.75| * 1 * 6 * 1 = 4.5 over the speed of 2.
        assert_eq!(temperate.apply(&entry, true, &view, &dig(Some(0.0)), 2.0), 6.5);
        // Unreported: the fallback -0.5 gives |-0.5 - 0.75| * 6 = 7.5 over the speed.
        assert_eq!(temperate.apply(&entry, true, &view, &dig(None), 2.0), 9.5);
        // lightspeed's formula over the block light decodes too (its other module is another lane's kind).
        assert!(MiningSpeed::decode(&fields(json!({
            "formula": [3.0, "$light", 5.0, "-", 5.0, "/", "^", "$level", "*", "$multiplier", "*", "$speed", "+"],
            "percent": false, "require_effective": true,
            "variables": {"light": {"type": "tconstruct:block_light", "fallback": 15.0, "light_layer": "block"}},
        }))).unwrap().is_some());
        // blasting reads the block's blast resistance, a block property this port does not hold.
        assert!(MiningSpeed::decode(&fields(json!({
            "formula": ["$resistance", "$level", "*", "$multiplier", "*", "$speed", "+"],
            "percent": false, "require_effective": true,
            "variables": {"resistance": {"type": "tconstruct:block", "block_type": "tconstruct:blast_resistance", "fallback": 3.0}},
        }))).unwrap().is_none());
        // crumbling asks whether the block requires the correct tool for drops: the same missing property.
        assert!(MiningSpeed::decode(&fields(json!({
            "blocks": {"type": "mantle:inverted", "inverted_type": "mantle:requires_tool"},
            "each_level": 1.0, "percent": false, "require_effective": false,
        }))).unwrap().is_none());
    }
}
