//! `library.tools.definition`: a tool as data (`ToolDefinitionData`), a list of modules loaded from
//! `data/<namespace>/tinkering/tool_definitions/<tool>.json`, and the modules that carry stats, slots, traits and actions
//! (`SetStatsModule`, `MultiplyStatsModule`, `ToolSlotsModule`, `ToolTraitsModule`, `ToolActionsModule`, `PartStatsModule`).
//!
//! The other module types the original registers load as raw data and are counted as not carried, as with modifiers.
use std::any::Any;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use serde_json::{Map, Value};

use super::stat::{ModifierStatsBuilder, StatKind, StatValue, ToolStatId, ToolStats};
use crate::materials::{MaterialId, MaterialRegistry, MaterialStatsId};
use crate::modifiers::ModifierEntry;
use crate::resource::ResourceId;

/// `ToolDefinitionLoader.FOLDER`.
pub const FOLDER: &str = "tinkering/tool_definitions";

/// The tool module types the original registers (`ToolModule.LOADER.register`), from its source and its data.
pub const ORIGINAL_TOOL_MODULES: [&str; 39] = [
    "attack_interaction",
    "base_stats",
    "box_aoe",
    "circle_aoe",
    "circle_melee",
    "conditional_aoe",
    "custom_material_name",
    "default_materials",
    "dual_option_interaction",
    "empty",
    "fixed_material_name",
    "is_effective",
    "item_name",
    "material_name",
    "material_repair",
    "material_stats",
    "material_traits",
    "max_tier",
    "melee_particle",
    "melting_fluid_effective",
    "mining_speed_modifier",
    "modifier_slots",
    "multiply_stats",
    "one_click_break",
    "part_stats",
    "preference_set_interaction",
    "remapping_materials",
    "stat_types_name",
    "statless_part_repair",
    "sweep_melee",
    "toggleable_set_interaction",
    "tool_actions",
    "tool_parts",
    "traits",
    "tree_aoe",
    "unique_material_name",
    "vein_aoe",
    "volatile_flag",
    "volatile_int",
];

/// One module of a tool definition (`ToolModule`).
pub trait ToolModule: Any + Send + Sync {
    fn kind(&self) -> &str;
    /// False for a module kept as data only.
    fn carried(&self) -> bool {
        true
    }
    fn as_any(&self) -> &dyn Any;
}

/// A module of a registered type whose behaviour is not ported.
#[derive(Clone, Debug, PartialEq)]
pub struct RawToolModule {
    pub kind: String,
    pub json: Map<String, Value>,
}

impl ToolModule for RawToolModule {
    fn kind(&self) -> &str {
        &self.kind
    }
    fn carried(&self) -> bool {
        false
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// `SetStatsModule` (`base_stats`): stats set outright, applied before the materials' as the tool's base.
#[derive(Clone, Debug, PartialEq)]
pub struct SetStatsModule {
    pub stats: BTreeMap<ToolStatId, StatValue>,
}

impl ToolModule for SetStatsModule {
    fn kind(&self) -> &str {
        "tconstruct:base_stats"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// `MultiplyStatsModule` (`multiply_stats`): each numeric stat multiplied (and its later additions too, `multiplyAll`).
#[derive(Clone, Debug, PartialEq)]
pub struct MultiplyStatsModule {
    pub multipliers: BTreeMap<ToolStatId, f32>,
}

impl ToolModule for MultiplyStatsModule {
    fn kind(&self) -> &str {
        "tconstruct:multiply_stats"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// `ToolSlotsModule` (`modifier_slots`): how many slots of each type the tool starts with.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolSlotsModule {
    pub slots: BTreeMap<String, i32>,
}

impl ToolModule for ToolSlotsModule {
    fn kind(&self) -> &str {
        "tconstruct:modifier_slots"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// `ToolTraitsModule` (`traits`): modifiers the tool always has.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolTraitsModule {
    pub traits: Vec<ModifierEntry>,
}

impl ToolModule for ToolTraitsModule {
    fn kind(&self) -> &str {
        "tconstruct:traits"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// `ToolActionsModule` (`tool_actions`): the actions (`axe_dig`, `shovel_flatten`) the tool can perform.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolActionsModule {
    pub actions: Vec<String>,
}

impl ToolModule for ToolActionsModule {
    fn kind(&self) -> &str {
        "tconstruct:tool_actions"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// `PartStatsModule` (`part_stats`): the parts of the tool, by the stat type each takes from its material, and which one counts most.
#[derive(Clone, Debug, PartialEq)]
pub struct PartStatsModule {
    pub parts: Vec<ToolPart>,
    pub primary_part: usize,
}

/// A part of a tool and how much of its stats count (`scale`, 1 for a whole part): written as an item id or `{"item": ..., "scale": ...}`.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolPart {
    pub item: ResourceId,
    pub scale: f32,
}

impl ToolModule for PartStatsModule {
    fn kind(&self) -> &str {
        "tconstruct:part_stats"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Reads a module of one type from its JSON object.
pub type ToolModuleLoader = Arc<dyn Fn(&Map<String, Value>, &ToolStats) -> Result<Arc<dyn ToolModule>, String> + Send + Sync>;

/// `ToolModule.LOADER`.
#[derive(Clone)]
pub struct ToolModuleRegistry {
    loaders: HashMap<String, ToolModuleLoader>,
}

fn numeric_stat<'a>(stats: &'a ToolStats, name: &str) -> Result<&'a super::stat::ToolStat, String> {
    let id = ToolStatId::try_parse(name).ok_or_else(|| format!("bad stat id '{name}'"))?;
    stats.get(&id).ok_or_else(|| format!("Unknown stat type {name}"))
}

impl ToolModuleRegistry {
    /// Every type of the original, each raw data except the ones ported here.
    pub fn with_original_types() -> Self {
        let mut loaders: HashMap<String, ToolModuleLoader> = HashMap::new();
        for name in ORIGINAL_TOOL_MODULES {
            let kind = format!("tconstruct:{name}");
            let kept = kind.clone();
            loaders.insert(kind, Arc::new(move |json, _| Ok(Arc::new(RawToolModule { kind: kept.clone(), json: json.clone() }) as Arc<dyn ToolModule>)));
        }
        loaders.insert(
            "tconstruct:base_stats".to_owned(),
            Arc::new(|json, stats| {
                let object = json.get("stats").and_then(Value::as_object).ok_or("base_stats needs 'stats'")?;
                let mut set = BTreeMap::new();
                for (name, value) in object {
                    let stat = numeric_stat(stats, name)?;
                    let value = match stat.kind {
                        StatKind::Tier => StatValue::Tier(value.as_str().and_then(ResourceId::try_parse).ok_or_else(|| format!("{name} must be a tier id"))?),
                        _ => StatValue::Float(value.as_f64().ok_or_else(|| format!("{name} must be a number"))? as f32),
                    };
                    set.insert(stat.id.clone(), value);
                }
                Ok(Arc::new(SetStatsModule { stats: set }) as Arc<dyn ToolModule>)
            }),
        );
        loaders.insert(
            "tconstruct:multiply_stats".to_owned(),
            Arc::new(|json, stats| {
                let object = json.get("multipliers").and_then(Value::as_object).ok_or("multiply_stats needs 'multipliers'")?;
                let mut multipliers = BTreeMap::new();
                for (name, value) in object {
                    let stat = numeric_stat(stats, name)?;
                    if !stat.is_numeric() {
                        return Err(format!("Invalid tool stat {name}, must be a numeric stat"));
                    }
                    multipliers.insert(stat.id.clone(), value.as_f64().ok_or_else(|| format!("{name} must be a number"))? as f32);
                }
                Ok(Arc::new(MultiplyStatsModule { multipliers }) as Arc<dyn ToolModule>)
            }),
        );
        loaders.insert(
            "tconstruct:modifier_slots".to_owned(),
            Arc::new(|json, _| {
                let object = json.get("slots").and_then(Value::as_object).ok_or("modifier_slots needs 'slots'")?;
                let mut slots = BTreeMap::new();
                for (name, value) in object {
                    slots.insert(name.clone(), value.as_i64().ok_or_else(|| format!("slot count of {name} must be a whole number"))? as i32);
                }
                Ok(Arc::new(ToolSlotsModule { slots }) as Arc<dyn ToolModule>)
            }),
        );
        loaders.insert(
            "tconstruct:traits".to_owned(),
            Arc::new(|json, _| {
                let list = json.get("traits").and_then(Value::as_array).ok_or("traits needs 'traits'")?;
                Ok(Arc::new(ToolTraitsModule { traits: list.iter().map(ModifierEntry::from_json).collect::<Result<_, _>>()? }) as Arc<dyn ToolModule>)
            }),
        );
        loaders.insert(
            "tconstruct:tool_actions".to_owned(),
            Arc::new(|json, _| {
                let list = json.get("tool_actions").and_then(Value::as_array).ok_or("tool_actions needs 'tool_actions'")?;
                let actions = list.iter().map(|a| a.as_str().map(str::to_owned).ok_or("a tool action must be a string")).collect::<Result<_, _>>()?;
                Ok(Arc::new(ToolActionsModule { actions }) as Arc<dyn ToolModule>)
            }),
        );
        loaders.insert(
            "tconstruct:part_stats".to_owned(),
            Arc::new(|json, _| {
                let list = json.get("parts").and_then(Value::as_array).ok_or("part_stats needs 'parts'")?;
                let parts = list
                    .iter()
                    .map(|p| match p {
                        Value::String(item) => ResourceId::try_parse(item).map(|item| ToolPart { item, scale: 1.0 }).ok_or("a part must be an item id"),
                        Value::Object(o) => {
                            let item = o.get("item").and_then(Value::as_str).and_then(ResourceId::try_parse).ok_or("a part needs an 'item'")?;
                            Ok(ToolPart { item, scale: o.get("scale").and_then(Value::as_f64).unwrap_or(1.0) as f32 })
                        }
                        _ => Err("a part must be an item id or an object"),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let primary_part = json.get("primary_part").and_then(Value::as_u64).unwrap_or(0) as usize;
                if primary_part >= parts.len().max(1) {
                    return Err(format!("primary_part {primary_part} is out of range for {} parts", parts.len()));
                }
                Ok(Arc::new(PartStatsModule { parts, primary_part }) as Arc<dyn ToolModule>)
            }),
        );
        Self { loaders }
    }

    pub fn load(&self, json: &Value, stats: &ToolStats) -> Result<Arc<dyn ToolModule>, String> {
        let object = json.as_object().ok_or("a module must be an object")?;
        let kind = object.get("type").and_then(Value::as_str).ok_or("a module needs a 'type'")?;
        let loader = self.loaders.get(kind).ok_or_else(|| format!("unknown module type '{kind}'"))?;
        loader(object, stats)
    }
}

/// `ToolDefinitionData`.
#[derive(Clone, Default)]
pub struct ToolDefinitionData {
    pub modules: Vec<Arc<dyn ToolModule>>,
}

impl ToolDefinitionData {
    /// `ToolDefinitionData.LOADABLE`: `{"modules": [...]}`.
    pub fn from_json(json: &Value, registry: &ToolModuleRegistry, stats: &ToolStats) -> Result<Self, String> {
        let mut modules = Vec::new();
        if let Some(list) = json.get("modules") {
            for module in list.as_array().ok_or("'modules' must be a list")? {
                modules.push(registry.load(module, stats)?);
            }
        }
        Ok(Self { modules })
    }

    fn all<T: 'static>(&self) -> impl Iterator<Item = &T> {
        self.modules.iter().filter_map(|m| m.as_any().downcast_ref::<T>())
    }

    /// `TOOL_STATS` hook: the base stats set, then the multipliers (`SetStatsModule.addToolStats`, `MultiplyStatsModule.addToolStats`).
    pub fn add_tool_stats(&self, builder: &mut ModifierStatsBuilder<'_>) {
        for module in self.all::<SetStatsModule>() {
            for (id, value) in &module.stats {
                builder.update(id, value);
            }
        }
        for module in self.all::<MultiplyStatsModule>() {
            for (id, factor) in &module.multipliers {
                builder.multiply_all(id, f64::from(*factor));
            }
        }
    }

    /// `VOLATILE_DATA` hook for slots: slots of each type, summed over the modules.
    pub fn slots(&self) -> BTreeMap<String, i32> {
        let mut total = BTreeMap::new();
        for module in self.all::<ToolSlotsModule>() {
            for (kind, count) in &module.slots {
                *total.entry(kind.clone()).or_insert(0) += count;
            }
        }
        total
    }

    /// `TOOL_TRAITS` hook, without the material traits: the modifiers the tool always has.
    pub fn traits(&self) -> Vec<ModifierEntry> {
        self.all::<ToolTraitsModule>().flat_map(|m| m.traits.clone()).collect()
    }

    /// `TOOL_ACTION` hook: whether any module lets the tool perform the action.
    pub fn can_perform(&self, action: &str) -> bool {
        self.all::<ToolActionsModule>().any(|m| m.actions.iter().any(|a| a == action))
    }

    /// `TOOL_PARTS` hook: the part items, in order.
    pub fn parts(&self) -> Vec<ToolPart> {
        self.all::<PartStatsModule>().flat_map(|m| m.parts.clone()).collect()
    }

    /// `MaterialStatsModule.addToolStats` for the part modules: every part's material gives the stats of the part's type, scaled. A
    /// material list shorter than the parts gives the stats of the parts it has.
    pub fn add_material_stats(&self, builder: &mut ModifierStatsBuilder<'_>, materials: &[MaterialId], registry: &MaterialRegistry) {
        for module in self.all::<PartStatsModule>() {
            for (index, part) in module.parts.iter().enumerate() {
                let (Some(material), Some(kind)) = (materials.get(index), part_stat_type(&part.item)) else { continue };
                if let Some(stats) = registry.stats_or_default(material, &kind) {
                    stats.apply(builder, part.scale);
                }
            }
        }
    }

    /// How many modules are kept as data only.
    pub fn uncarried_modules(&self) -> usize {
        self.modules.iter().filter(|m| !m.carried()).count()
    }
}

/// `IToolPart.getStatType` for the part items of `TinkerToolParts`: the kind of material stats each takes.
pub fn part_stat_type(part: &ResourceId) -> Option<MaterialStatsId> {
    if part.namespace() != "tconstruct" {
        return None;
    }
    let path = part.path();
    let name = match path {
        "pick_head" | "hammer_head" | "small_axe_head" | "broad_axe_head" | "small_blade" | "broad_blade" | "adze_head" | "large_plate" => "head",
        "bow_limb" => "limb",
        "bow_grip" => "grip",
        "tool_handle" | "tough_handle" => "handle",
        "tool_binding" | "tough_binding" => "binding",
        "bowstring" | "arrow_head" | "arrow_shaft" | "fletching" | "maille" | "shield_core" | "ribcage" | "shell" | "laces" => path,
        _ if path.ends_with("_plating") => return MaterialStatsId::try_parse(&format!("tconstruct:plating_{}", path.trim_end_matches("_plating"))),
        _ => return None,
    };
    MaterialStatsId::try_parse(&format!("tconstruct:{name}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::stat::HarvestTiers;
    use serde_json::json;

    fn stats() -> ToolStats {
        ToolStats::tinkers(HarvestTiers::vanilla())
    }

    fn id(name: &str) -> ToolStatId {
        ToolStatId::tconstruct(name)
    }

    fn data(modules: Vec<Value>) -> ToolDefinitionData {
        ToolDefinitionData::from_json(&json!({"modules": modules}), &ToolModuleRegistry::with_original_types(), &stats()).unwrap()
    }

    fn built(stats: &ToolStats, builder: &ModifierStatsBuilder<'_>, name: &str) -> f32 {
        builder.build().get(&id(name)).and_then(StatValue::float).unwrap_or_else(|| stats.get(&id(name)).unwrap().default)
    }

    #[test]
    fn empty_data_contains_nothing() {
        let data = ToolDefinitionData::default();
        assert!(data.parts().is_empty() && data.traits().is_empty() && data.slots().is_empty() && data.modules.is_empty());
        let stats = stats();
        let mut builder = ModifierStatsBuilder::new(&stats);
        data.add_tool_stats(&mut builder);
        assert!(builder.build().is_empty());
    }

    #[test]
    fn base_stats_set_and_multipliers_multiply_the_later_additions_too() {
        let stats = stats();
        // the original's data_buildStats_withData: mining speed (1 + 5) * 5, durability set to 100, attack speed 1 * 2
        let data = data(vec![json!({"type": "tconstruct:multiply_stats", "multipliers": {"tconstruct:mining_speed": 5.0, "tconstruct:attack_speed": 2.0}})]);
        let mut builder = ModifierStatsBuilder::new(&stats);
        builder.add(&id("mining_speed"), 5.0);
        builder.update(&id("durability"), &StatValue::Float(100.0));
        data.add_tool_stats(&mut builder);
        assert_eq!(built(&stats, &builder, "mining_speed"), 30.0);
        assert_eq!(built(&stats, &builder, "durability"), 100.0);
        assert_eq!(built(&stats, &builder, "attack_speed"), 2.0);
        assert_eq!(builder.build().len(), 3);
    }

    #[test]
    fn stats_from_a_definition_are_put_where_the_original_puts_them() {
        let stats = stats();
        let with_bonus = data(vec![json!({"type": "tconstruct:base_stats", "stats": {"tconstruct:durability": 100, "tconstruct:attack_speed": 5.5}})]);
        let mut builder = ModifierStatsBuilder::new(&stats);
        with_bonus.add_tool_stats(&mut builder);
        let result = builder.build();
        assert_eq!(result.len(), 2);
        assert_eq!(result[&id("durability")], StatValue::Float(100.0));
        assert_eq!(result[&id("attack_speed")], StatValue::Float(5.5));
        assert!(!result.contains_key(&id("mining_speed")), "a stat the module does not name stays default");
        assert!(builder.build_multipliers().is_empty());
    }

    #[test]
    fn slots_traits_actions_and_parts() {
        let data = data(vec![
            json!({"type": "tconstruct:modifier_slots", "slots": {"upgrades": 5, "abilities": 2}}),
            json!({"type": "tconstruct:traits", "traits": [{"name": "tconstruct:pierce", "level": 1}]}),
            json!({"type": "tconstruct:tool_actions", "tool_actions": ["shovel_flatten"]}),
            json!({"type": "tconstruct:part_stats", "parts": [{"item": "tconstruct:pick_head", "scale": 0.25}, "tconstruct:tool_handle"], "primary_part": 0}),
        ]);
        assert_eq!(data.slots()["upgrades"], 5);
        assert_eq!(data.slots()["abilities"], 2);
        assert_eq!(data.slots().get("defense"), None);
        assert_eq!(data.traits()[0].modifier.to_string(), "tconstruct:pierce");
        assert!(data.can_perform("shovel_flatten") && !data.can_perform("sword_dig"));
        assert_eq!(data.parts().len(), 2);
        assert_eq!((data.parts()[0].scale, data.parts()[1].scale), (0.25, 1.0));
        assert_eq!(data.uncarried_modules(), 0);
    }

    #[test]
    fn broken_modules_are_errors() {
        let registry = ToolModuleRegistry::with_original_types();
        let stats = stats();
        let bad = |module: Value| ToolDefinitionData::from_json(&json!({"modules": [module]}), &registry, &stats).is_err();
        assert!(bad(json!({"type": "nobody:knows"})));
        assert!(bad(json!({"type": "tconstruct:base_stats", "stats": {"tconstruct:nothing": 1}})));
        assert!(bad(json!({"type": "tconstruct:multiply_stats", "multipliers": {"tconstruct:harvest_tier": 2}})));
        assert!(bad(json!({"type": "tconstruct:part_stats", "parts": ["tconstruct:pick_head"], "primary_part": 3})));
    }
}
