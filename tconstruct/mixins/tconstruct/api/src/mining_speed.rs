//! ConditionalMiningSpeedModule and its block, holder and variable inputs, from SlimeKnights
//! (MIT; see LICENSE). A module changes the dig speed of a block as PlutoLib's break-speed event
//! reports it: the block and its tags, the holder's ground and water state, the facts of the
//! dig itself — the light at the block and the temperature of its biome — which the block light
//! and biome temperature variables read, and the digging player as an entity (their effects, the
//! rain on them and the enchantments on the tool they dig with), which the entity variables and
//! `mantle:has_enchantment` read. The block property variable (`tconstruct:block` over blast
//! resistance) and the `mantle:requires_tool` block predicate read the same dig report: a fact
//! the game did not report is None, the variable takes its fallback and the predicate's leaf
//! does not match, exactly as an unreported light or temperature behaves. A block property this
//! port does not hold (hardness) stays unbound instead of being half-installed.
use crate::{formula::Formula, modifier_entries::{nbt_int, Entry}, tool_stats::Stat};
use pluto_lib::{Identifier, Nbt, NbtCompound};
use serde_json::{Map, Value};

/// IJsonPredicate over the block being dug.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BlockCondition { Any, Tag(Identifier), Set(Vec<Identifier>), RequiresTool, Not(Box<BlockCondition>), And(Vec<BlockCondition>), Or(Vec<BlockCondition>) }

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
    /// BlockPropertyVariable over blast resistance (`tconstruct:block` with block type
    /// `tconstruct:blast_resistance`): the explosion resistance of the block being dug;
    /// `fallback` when the game did not report it.
    BlastResistance { fallback: f32 },
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
    /// The block's explosion resistance, as the break-speed event carries it; None when unreported.
    pub blast_resistance: Option<f32>,
    /// Whether the block needs the correct tool for drops, as the break-speed event carries it; None when unreported.
    pub requires_tool: Option<bool>,
    /// The y of the block being dug, for the modules that scale with depth (dwarven); 0 when no dig reported it.
    pub pos_y: i32,
    /// The build range of the level the dig is in, [min, max), for the modules that read the level's
    /// max build height (dwarven); None when the game did not report it.
    pub build_height: Option<[i32; 2]>,
}

/// The build range of a dimension, [min, max), for the modules that read the level's max build height
/// (dwarven) when no dig reported it: the vanilla dimensions' ranges, the overworld's for anything else.
/// A data pack dimension's range is not known here — the same reduction as the port's crop_max_age table.
/// This is only the fallback for when the host has not answered for the dimension (outside a callback,
/// or a level that never ticked); see `level_build_height`.
pub(crate) fn build_height_for(dimension: &str) -> [i32; 2] {
    match dimension {
        "minecraft:the_nether" | "minecraft:the_end" => [0, 256],
        _ => [-64, 320],
    }
}

/// The build range of the level `dimension` stands in, [min, max), for the modules that read the level's
/// max build height (dwarven): the host's own range for the dimension when the game has published it —
/// any dimension, custom ones included — else the vanilla table above.
pub(crate) fn level_build_height(dimension: &str) -> [i32; 2] {
    pluto_lib::World::new(dimension).build_height().unwrap_or_else(|| build_height_for(dimension))
}

/// DwarvenModifier.getBoost: the speed after the depth adjustment, for a dig or a shot at height
/// `y` in a level whose max build height is `max_build_height`. `bonus` is the per-level bonus at
/// full depth (the mining bonus for a dig, the velocity bonus for a shot), already scaled by the
/// caller; the boost can go negative nowhere, the debuff only scales the base down.
pub(crate) fn dwarven_boost(max_build_height: i32, y: f32, level: f32, base: f32, bonus: f32) -> f32 {
    // Grants 0 bonus at 64, 1x at -64, 2x at -128; clamped so tall worlds stay sane.
    if y < 64.0 {
        let scale = ((64.0 - y) / 64.0).clamp(0.0, 2.0);
        return base + level * scale * bonus;
    }
    // The debuff starts 192 blocks below the top, but at the earliest at 96, so short worlds
    // (the nether: between 256 and 96) keep their 64 blocks of neutral above the boost range.
    let baseline = (max_build_height - 192).max(96) as f32;
    if y > baseline {
        // A range of 128 blocks for the regular debuff; anything above is the full debuff.
        if y >= baseline + 128.0 { return base * 0.25; }
        return base * (1.0 - ((y - baseline) / 128.0 * 0.75));
    }
    base
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
    /// The player doing the digging, as the break-speed event reported them: their effects, the
    /// rain on them, and the enchantments on the tool they dig with (which is what
    /// `mantle:has_enchantment` asks of a digger — a worn piece's enchantments are not reported).
    /// None when the game did not name a digger.
    pub holder: Option<pluto_lib::EntityState>,
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
            "mantle:requires_tool" => Self::RequiresTool,
            "mantle:inverted" => {
                let inner = Self::decode(&Value::Object(retyped(object, "inverted_type")?))?;
                match inner { Some(inner) => Self::Not(Box::new(inner)), None => return Ok(None) }
            }
            "mantle:and" => match list("predicates")? { Some(children) => Self::And(children), None => return Ok(None) },
            "mantle:or" => match list("predicates")? { Some(children) => Self::Or(children), None => return Ok(None) },
            _ => return Ok(None),
        }))
    }
    pub(crate) fn matches(&self, block: &Identifier, in_tag: &dyn Fn(&Identifier) -> bool, facts: &BlockFacts) -> bool {
        match self {
            Self::Any => true,
            Self::Tag(tag) => in_tag(tag),
            Self::Set(blocks) => blocks.contains(block),
            // An unreported requirement does not match the leaf it would decide, as an unknown
            // ground or water state does not match the holder condition it would decide.
            Self::RequiresTool => facts.requires_tool == Some(true),
            Self::Not(inner) => !inner.matches(block, in_tag, facts),
            Self::And(children) => children.iter().all(|child| child.matches(block, in_tag, facts)),
            Self::Or(children) => children.iter().any(|child| child.matches(block, in_tag, facts)),
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
            "tconstruct:block" => {
                let object = object.ok_or("block variable needs fields")?;
                let fallback = object.get("fallback").and_then(Value::as_f64).ok_or("fallback missing")? as f32;
                // BlockPropertyVariable: the block type names the property read off the block being
                // dug. Blast resistance is the one the dig report carries; another property (the
                // block's hardness) is not reported and stays unbound.
                match object.get("block_type").and_then(Value::as_str) {
                    Some("tconstruct:blast_resistance") => Self::BlastResistance { fallback },
                    _ => return Ok(None),
                }
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
                // A condition inside the variable asks about the entity the variable reads: in the
                // holder scope that is the digging player themselves (role `holder`), whose own
                // eyes and ground the break-speed event reports.
                let role = match scope { EntityScope::Holder => "holder", _ if whose == Whose::Attacker => "attacker", _ => "target" };
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
            Self::BlastResistance { fallback } => tool.facts.blast_resistance.unwrap_or(*fallback),
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
        // The holder is the digging player the event reports, so entity variables over them decode
        // (EntityScope::Holder); a variable over anything else still stays unbound.
        let Some(formula) = StatFormula::decode_in(fields, &NAMES, crate::mining_speed::EntityScope::Holder)? else { return Ok(None); };
        Ok(Some(Self { blocks, holder, require_effective, formula }))
    }

    /// Whether the module is a percent module, which the original runs later (priority 75).
    pub fn percent(&self) -> bool { self.formula.percent() }

    /// BreakSpeedModifierHook.modifyBreakSpeed. `condition` is the module's ModifierCondition result.
    pub fn apply(&self, entry: &Entry, condition: bool, tool: &ToolView, context: &Context, speed: f32) -> f32 {
        if (self.require_effective && !context.effective) || !condition || !self.blocks.matches(context.block, context.block_in_tag, &context.facts) || !self.holder.matches(context) { return speed; }
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
        view.facts = BlockFacts { block_light: Some(9), sky_light: Some(15), sky_darken: 11, biome_temperature: Some(0.8), blast_resistance: None, requires_tool: None, pos_y: 0, build_height: None };
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
    fn build_height_for_names_the_vanilla_dimensions_ranges() {
        assert_eq!(build_height_for("minecraft:overworld"), [-64, 320]);
        assert_eq!(build_height_for("minecraft:the_nether"), [0, 256]);
        assert_eq!(build_height_for("minecraft:the_end"), [0, 256]);
        assert_eq!(build_height_for("example:custom"), [-64, 320], "an unknown dimension reads the overworld's range");
    }

    #[test]
    fn level_build_height_reads_the_host_range_and_falls_back_to_the_table() {
        use pluto_lib::world_api_v1::{with_world, MemoryWorldOps};
        // No scope, no publication: the table answers.
        assert_eq!(level_build_height("example:custom"), [-64, 320]);
        assert_eq!(level_build_height("minecraft:the_nether"), [0, 256]);
        // A host range published for the dimension wins over the table, for a custom dimension
        // and for a vanilla one alike; an unpublished dimension keeps the table.
        let mut ops = MemoryWorldOps::default();
        ops.build_heights.insert("example:custom".into(), [-32, 128]);
        ops.build_heights.insert("minecraft:the_nether".into(), [0, 192]);
        with_world(&mut ops, || {
            assert_eq!(level_build_height("example:custom"), [-32, 128]);
            assert_eq!(level_build_height("minecraft:the_nether"), [0, 192]);
            assert_eq!(level_build_height("minecraft:the_end"), [0, 256]);
        });
    }

    #[test]
    fn dwarven_boost_scales_with_depth_and_fades_with_height() {
        // At 64 and up to the debuff baseline nothing changes.
        assert_eq!(dwarven_boost(320, 64.0, 2.0, 10.0, 6.0), 10.0);
        assert_eq!(dwarven_boost(320, 100.0, 2.0, 10.0, 6.0), 10.0);
        // At 0 the scale is 1: base + level * bonus; at -64 it is 2, and no deeper than that.
        assert_eq!(dwarven_boost(320, 0.0, 2.0, 10.0, 6.0), 22.0);
        assert_eq!(dwarven_boost(320, -64.0, 1.0, 10.0, 6.0), 22.0);
        assert_eq!(dwarven_boost(320, -640.0, 1.0, 10.0, 6.0), 22.0);
        // The overworld debuff baseline is 320 - 192 = 128: halfway up its range the speed
        // keeps 1 - 0.375, and at 256 and above only a quarter is left.
        assert_eq!(dwarven_boost(320, 192.0, 1.0, 10.0, 6.0), 6.25);
        assert_eq!(dwarven_boost(320, 256.0, 1.0, 10.0, 6.0), 2.5);
        // A short world (max build height 256) starts its debuff at 96 instead.
        assert_eq!(dwarven_boost(256, 96.0, 1.0, 10.0, 6.0), 10.0);
        assert_eq!(dwarven_boost(256, 224.0, 1.0, 10.0, 6.0), 2.5);
    }

    #[test]
    fn the_shipped_temperate_lightspeed_and_block_property_modules_decode() {
        // hydraulic: the bonus is a conditional over the digging player (eyes in water, then the
        // aqua affinity on their tool, else the rain on them); it decodes over the reported holder.
        let bonus = json!({"type": "tconstruct:entity", "condition_type": "mantle:eyes_in_water", "entity_type": "tconstruct:conditional", "fallback": 8.0,
            "if_false": {"type": "tconstruct:conditional", "condition_type": "mantle:raining_at", "if_false": 0.0, "if_true": 4.0},
            "if_true": {"type": "tconstruct:conditional", "condition_type": "mantle:has_enchantment", "enchantment": "minecraft:aqua_affinity", "if_false": 40.0, "if_true": 8.0}});
        let fact = crate::entity_facts::EntityFact::decode("tconstruct:conditional", bonus.as_object().unwrap(), "holder");
        assert!(fact.unwrap().is_some(), "the conditional fact decodes for the holder");
        let hydraulic = MiningSpeed::decode(&fields(json!({
            "formula": ["$multiplier", "$bonus", "*", "$level", "*", "$speed", "+"],
            "percent": false, "require_effective": true,
            "variables": {"bonus": bonus},
        })));
        let hydraulic = hydraulic.unwrap();
        assert!(hydraulic.is_some(), "hydraulic decodes over the reported digging player");        // temperate: |temperature - 0.75| * level * 6 * multiplier over the speed.
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
        let dig = |temperature: Option<f32>| Context { block: &block, block_in_tag: &no_tags, original: 2.0, multiplier: 1.0, effective: true, on_ground: None, eyes_in_water: None, facts: BlockFacts { biome_temperature: temperature, ..BlockFacts::default() }, holder: None };
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
        // blasting reads the block's blast resistance off the dig report, its fallback standing in
        // when the game did not report the block's resistance.
        let blasting = MiningSpeed::decode(&fields(json!({
            "formula": [3.0, 6.0, "$resistance", "-", 1.5, "/", "^", 10.0, "min", "$level", "*", "$multiplier", "*", "$speed", "+"],
            "percent": false, "require_effective": true,
            "variables": {"resistance": {"type": "tconstruct:block", "block_type": "tconstruct:blast_resistance", "fallback": 3.0}},
        }))).unwrap().expect("blasting decodes over the reported blast resistance");
        let blasting_entry = Entry::new(Identifier::parse("tconstruct:blasting").unwrap(), 1);
        let dig_at = |resistance: Option<f32>| Context { block: &block, block_in_tag: &no_tags, original: 2.0, multiplier: 1.0, effective: true, on_ground: None, eyes_in_water: None, facts: BlockFacts { blast_resistance: resistance, ..BlockFacts::default() }, holder: None };
        // The formula is 3 ^ ((6 - resistance) / 1.5), capped at 10, times level and multiplier over
        // the speed: a reported resistance of 6 gives 3 ^ 0 = 1 over the speed of 2.
        assert_eq!(blasting.apply(&blasting_entry, true, &view, &dig_at(Some(6.0)), 2.0), 3.0);
        // Unreported: the fallback 3.0 gives 3 ^ ((6 - 3) / 1.5) = 9 over the speed of 2.
        assert_eq!(blasting.apply(&blasting_entry, true, &view, &dig_at(None), 2.0), 11.0);
        // A block property the dig report does not carry (hardness) still stays unbound.
        assert!(MiningSpeed::decode(&fields(json!({
            "formula": ["$hardness", "$level", "*", "$multiplier", "*", "$speed", "+"],
            "percent": false, "require_effective": true,
            "variables": {"hardness": {"type": "tconstruct:block", "block_type": "tconstruct:hardness", "fallback": 0.0}},
        }))).unwrap().is_none());
        // crumbling asks whether the block requires the correct tool for drops, inverted: it boosts
        // the speed on blocks that do not, by its level, with no effective-tool requirement.
        let crumbling = MiningSpeed::decode(&fields(json!({
            "blocks": {"type": "mantle:inverted", "inverted_type": "mantle:requires_tool"},
            "each_level": 1.0, "percent": false, "require_effective": false,
        }))).unwrap().expect("crumbling decodes over the reported tool requirement");
        let crumbling_entry = Entry::new(Identifier::parse("tconstruct:crumbling").unwrap(), 2);
        let dig_requiring = |requires: Option<bool>| Context { block: &block, block_in_tag: &no_tags, original: 2.0, multiplier: 1.0, effective: false, on_ground: None, eyes_in_water: None, facts: BlockFacts { requires_tool: requires, ..BlockFacts::default() }, holder: None };
        assert_eq!(crumbling.apply(&crumbling_entry, true, &view, &dig_requiring(Some(false)), 2.0), 4.0, "level 2 adds 2 to a block that needs no tool");
        assert_eq!(crumbling.apply(&crumbling_entry, true, &view, &dig_requiring(Some(true)), 2.0), 2.0, "a block that requires the tool gets nothing");
        assert_eq!(crumbling.apply(&crumbling_entry, true, &view, &dig_requiring(None), 2.0), 4.0, "unreported reads as the leaf not matching, so the inversion applies");
    }
}
