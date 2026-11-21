//! ToolDefinitionLoader/ToolDefinitionData lifecycle from SlimeKnights (MIT; see LICENSE).
//! Unsupported modules invalidate a whole definition; a stat-only partial tool is never published.
use crate::{material_stats::Stats, materials::Definitions as Materials, tool_materials::MaterialList, tool_stats::{Builder, StatModule}};
use pluto_lib::{Identifier, Nbt, NbtCompound, ResourceManagerHelper, ResourceType, ServerLifecycleEvents};
use serde_json::Value;
use std::{collections::{BTreeMap, BTreeSet}, sync::{Arc, OnceLock, RwLock}};

pub(crate) const FOLDER: &str = "tinkering/tool_definitions";

/// Modules recorded by type but not implemented, grouped by the consumer they are missing.
/// Their fields are NOT validated and nothing here implements them: `Data::unbound_modules`
/// lists them so a caller can never mistake the definition for fully supported. Every other
/// unknown module still rejects.
///
/// * Melee extras (melee_particle, sweep_melee, circle_melee): a hit context names only the
///   attacker and the victim; PlutoLib has no nearby-entity enumeration, no entity action
///   that addresses a third entity, and no way to spawn a named particle at an entity
///   (ParticleFactoryRegistry only registers item-particle factories).
/// * Interaction gates (attack/dual_option/toggleable_set interaction): nothing in the tool
///   system asks a definition which interaction source a modifier may use; there is no
///   canInteract consumer to bind these to.
/// * Display names (fixed/unique/material/custom material name): a stack's name comes from
///   the host's item name; no code path computes a material-based display name.
/// * melting_fluid_effective: effectiveness here is one static module baked into the stack's
///   `minecraft:tool` component at rebuild; this module's answer depends on the tool tank's
///   current fluid and space and on a melting-recipe lookup at dig time, neither of which
///   the mining paths hold.
const UNBOUND: &[&str] = &[
    "tconstruct:melee_particle", "tconstruct:sweep_melee", "tconstruct:circle_melee",
    "tconstruct:attack_interaction", "tconstruct:dual_option_interaction", "tconstruct:toggleable_set_interaction",
    "tconstruct:melting_fluid_effective",
    "tconstruct:fixed_material_name", "tconstruct:unique_material_name", "tconstruct:material_name", "tconstruct:custom_material_name",
];

#[derive(Clone, Debug, PartialEq, Eq)]
enum VolatileModule { Flag(Identifier), Int(Identifier, i32), Slots(BTreeMap<String, i32>), MaterialLook { slots: usize } }

#[derive(Clone, Debug)]
enum TraitModule {
    Constant(Vec<crate::material_traits::ModifierEntry>),
    Material { stat: Identifier, index: i32 },
    /// MaterialStatsModule.addTraits: positional material traits for each declared stat type.
    Positional(Vec<Identifier>),
}

/// MaterialRepairModule: a fixed amount, or a repairable stat type looked up at query time.
/// StatlessPartRepairModule: the material sitting at one part index repairs for a fixed amount.
#[derive(Clone, Debug)]
enum RepairModule { Constant { material: Identifier, amount: i32 }, Stat { material: Identifier, stat: Identifier }, Parts(Vec<Identifier>), Statless { index: i32, amount: i32 } }
impl RepairModule {
    fn is_for(&self, tool: &MaterialList, material: &Identifier) -> bool {
        match self {
            Self::Constant { material: own, .. } | Self::Stat { material: own, .. } => own == material,
            Self::Parts(stats) => repair_positions(stats).any(|index| tool.get(index as i32).material_id().as_ref() == Some(material)),
            Self::Statless { index, .. } => tool.get(*index).material_id().as_ref() == Some(material),
        }
    }
    fn amount(&self, tool: &MaterialList, material: &Identifier, registry: &Materials) -> f32 {
        if !self.is_for(tool, material) { return 0.0; }
        match self {
            Self::Constant { amount, .. } | Self::Statless { amount, .. } => *amount as f32,
            Self::Stat { stat, .. } => repair_durability(registry, material, stat) as f32,
            // The largest durability among the tool's repairable positions holding this material.
            Self::Parts(stats) => repair_positions(stats).filter(|index| tool.get(*index as i32).material_id().as_ref() == Some(material))
                .map(|index| repair_durability(registry, material, &stats[index])).max().unwrap_or(0) as f32,
        }
    }
}
/// MaterialStatsManager.canRepair: only stat types whose record implements IRepairableMaterialStats.
fn repairable(stat: &Identifier) -> bool {
    stat.namespace() == "tconstruct" && matches!(stat.path(), "head" | "limb" | "skull" | "slime" | "ribcage" | "shell" | "laces"
        | "plating_helmet" | "plating_chestplate" | "plating_leggings" | "plating_boots" | "plating_shield")
}
fn repair_positions(stats: &[Identifier]) -> impl Iterator<Item = usize> + '_ { (0..stats.len()).filter(|index| repairable(&stats[*index])) }
/// Crate-visible MaterialRepairModule.getDurability for recipes that price repair by part.
pub(crate) fn repair_durability_of(registry: &Materials, material: &Identifier, stat: &Identifier) -> i32 { repair_durability(registry, material, stat) }
/// MaterialRepairModule.getDurability: registered stats only, never the type default.
fn repair_durability(registry: &Materials, material: &Identifier, stat: &Identifier) -> i32 {
    match registry.get_stats(material, stat) {
        Some(Stats::Head { durability, .. } | Stats::Limb { durability, .. } | Stats::Plating { durability, .. }
            | Stats::Skull { durability, .. } | Stats::Slime { durability, .. } | Stats::Repair { durability }) => *durability,
        _ => 0,
    }
}

/// RandomMaterial: one entry of a default/remapping materials list. The `random_variant` and
/// `ancient` registry types are not decoded (no shipped definition uses them, and variant
/// choices need the material recipe cache), and a `random` entry's material predicate is not
/// decoded either (no shipped entry carries one); both reject instead of guessing.
#[derive(Clone, Debug, PartialEq)]
enum RandomMaterial {
    Fixed(crate::tool_materials::MaterialVariant),
    First,
    Random { min_tier: i32, max_tier: i32, allow_hidden: bool },
}
impl RandomMaterial {
    fn decode(value: &Value) -> Result<Self, String> {
        let object = value.as_object().ok_or("random material must be an object")?;
        let kind = Identifier::parse(object.get("type").and_then(Value::as_str).ok_or("random material type is required")?)
            .map_err(|error| error.to_string())?;
        match kind.to_string().as_str() {
            "tconstruct:fixed" => {
                let text = object.get("material").and_then(Value::as_str).ok_or("fixed random material requires a material")?;
                crate::tool_materials::MaterialVariant::parse(text).map(Self::Fixed)
                    .ok_or_else(|| format!("invalid material variant {text}"))
            }
            "tconstruct:first" => Ok(Self::First),
            "tconstruct:random" => {
                if object.contains_key("material") { return Err("random material predicates are not supported".into()); }
                // IntRange: a bare number is an exact tier; the default range is every tier.
                let int = |value: &Value| crate::materials::number_int(&value.to_string());
                let (min_tier, max_tier) = match object.get("tier") {
                    None | Some(Value::Null) => (0, i32::MAX),
                    Some(Value::Number(_)) => { let tier = int(object.get("tier").unwrap()); (tier, tier) }
                    Some(Value::Object(range)) => (
                        range.get("min").map(int).unwrap_or(0),
                        range.get("max").map(int).unwrap_or(i32::MAX),
                    ),
                    Some(_) => return Err("tier must be a number or a range object".into()),
                };
                if min_tier < 0 || max_tier < min_tier { return Err("tier range is invalid".into()); }
                let allow_hidden = match object.get("allow_hidden") {
                    None | Some(Value::Null) => false,
                    Some(Value::Bool(value)) => *value,
                    Some(_) => return Err("allow_hidden must be boolean".into()),
                };
                Ok(Self::Random { min_tier, max_tier, allow_hidden })
            }
            _ => Err(format!("random material type {kind} is not supported")),
        }
    }

    /// RandomMaterial.getMaterial for one stat type; `roll` is the port's [0, 1) random source
    /// standing in for RandomSource.nextInt over the choices.
    fn pick(&self, stat: &Identifier, registry: &Materials, roll: &mut dyn FnMut() -> f32) -> crate::tool_materials::MaterialVariant {
        match self {
            Self::Fixed(variant) => variant.clone(),
            Self::First => first_with_stat(registry, stat),
            Self::Random { min_tier, max_tier, allow_hidden } => {
                // Choices in the registry's canonical order (the visible sort extended to
                // hidden materials): tier in range, hidden only when allowed, has the stat.
                let mut choices: Vec<&crate::materials::Material> = registry.materials.values()
                    .filter(|material| material.tier >= *min_tier && material.tier <= *max_tier
                        && (*allow_hidden || !material.hidden) && registry.get_stats(&material.id, stat).is_some())
                    .collect();
                choices.sort_by(|a, b| a.tier.cmp(&b.tier).then(a.sort_order.cmp(&b.sort_order))
                    .then(a.id.path().cmp(b.id.path())).then(a.id.namespace().cmp(b.id.namespace())));
                if choices.is_empty() { return first_with_stat(registry, stat); }
                let index = ((roll() * choices.len() as f32) as usize).min(choices.len() - 1);
                crate::tool_materials::MaterialVariant::parse(&choices[index].id.to_string())
                    .unwrap_or_else(crate::tool_materials::MaterialVariant::unknown)
            }
        }
    }
}

/// MaterialRegistry.firstWithStatType as a variant; the unknown material when nothing has the
/// stat type. The registry's own first-with-stat scan is over visible (non-hidden) materials.
fn first_with_stat(registry: &Materials, stat: &Identifier) -> crate::tool_materials::MaterialVariant {
    registry.first_with_stat(stat)
        .and_then(|material| crate::tool_materials::MaterialVariant::parse(&material.id.to_string()))
        .unwrap_or_else(crate::tool_materials::MaterialVariant::unknown)
}

/// The definition's MISSING_MATERIALS binding; the hook has no merger, so at most one module.
#[derive(Clone, Debug, PartialEq)]
enum MissingModule {
    Default(Vec<RandomMaterial>),
    Remap { missing: Vec<RandomMaterial>, remap: Vec<Vec<(Identifier, crate::tool_materials::MaterialVariant)>> },
}

fn random_materials(fields: &serde_json::Map<String, Value>, key: &str) -> Result<Vec<RandomMaterial>, String> {
    fields.get(key).and_then(Value::as_array).filter(|entries| !entries.is_empty())
        .ok_or_else(|| format!("{key} must be a nonempty array"))?
        .iter().map(RandomMaterial::decode).collect()
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Data {
    stats: Vec<StatModule>, volatile: Vec<VolatileModule>, actions: BTreeSet<String>,
    traits: Vec<TraitModule>, repairs: Vec<RepairModule>, parts: Vec<Identifier>,
    /// Trait modules bound to the rebalanced/trim ToolTraitHooks; only swappable_tool_traits asks for them.
    hook_traits: BTreeMap<Identifier, Vec<TraitModule>>,
    effective: Option<crate::tool_mining::Effective>,
    /// The AOE_ITERATOR binding (box/circle/vein/conditional/tree), when a module claims the hook.
    aoe: Option<crate::aoe::Iterator>,
    /// MiningSpeedModifierModules in module order (MINING_SPEED composes every module).
    speed: Vec<(crate::mining_speed::BlockCondition, f32)>,
    /// MaxTierModule cap on the harvest tier (MINING_TIER composes by taking the minimum).
    max_tier: Option<Identifier>,
    /// The MISSING_MATERIALS binding, when a module claims the hook.
    missing: Option<MissingModule>,
    unbound: Vec<Identifier>,
}
impl Data {
    pub(crate) fn decode(value: &Value) -> Result<Self, String> {
        let object = value.as_object().ok_or("tool definition must be an object")?;
        let modules = match object.get("modules") {
            None | Some(Value::Null) => return Ok(Self::default()),
            Some(value) => value.as_array().ok_or("modules must be an array")?,
        };
        let mut stats = Vec::new();
        let mut volatile = Vec::new();
        let mut actions = BTreeSet::new();
        let mut traits: Vec<TraitModule> = Vec::new();
        let mut hook_traits: BTreeMap<Identifier, Vec<TraitModule>> = BTreeMap::new();
        let mut alternate_hooks: Vec<Identifier> = Vec::new();
        let mut repairs: Vec<RepairModule> = Vec::new();
        let mut tool_parts: Option<Vec<Identifier>> = None;
        let mut effective: Option<crate::tool_mining::Effective> = None;
        let mut aoe: Option<crate::aoe::Iterator> = None;
        let mut speed: Vec<(crate::mining_speed::BlockCondition, f32)> = Vec::new();
        let mut max_tier: Option<Identifier> = None;
        let mut missing: Option<MissingModule> = None;
        let mut unbound: Vec<Identifier> = Vec::new();
        for module in modules {
            let fields = module.as_object().ok_or("module must be an object")?;
            let kind = Identifier::parse(fields.get("type").and_then(Value::as_str).ok_or("module type is required")?)
                .map_err(|error| error.to_string())?;
            let empty = kind.to_string() == "tconstruct:empty";
            if UNBOUND.contains(&kind.to_string().as_str()) {
                if !unbound.contains(&kind) { unbound.push(kind); }
                continue;
            }
            if kind.to_string() == "tconstruct:tool_parts" {
                // ToolPartsModule: the part items by slot; TOOL_PARTS has no merger.
                if tool_parts.is_some() { return Err("only one module may bind tool_parts".into()); }
                let registry = crate::tool_parts::registry();
                let items = fields.get("parts").and_then(Value::as_array).filter(|items| !items.is_empty()).ok_or("parts must be a nonempty array")?
                    .iter().map(|item| {
                        let item = Identifier::parse(item.as_str().ok_or("part must be an ID string")?).map_err(|error| error.to_string())?;
                        if registry.contains_key(&item) { Ok(item) } else { Err(format!("unknown tool part {item}")) }
                    }).collect::<Result<Vec<_>, String>>()?;
                tool_parts = Some(items);
                continue;
            }
            // Material/part modules also install traits, repair and volatile-data hooks.
            // Their statistical decoder alone is deliberately insufficient here.
            let volatile_kind = matches!(kind.to_string().as_str(), "tconstruct:volatile_flag" | "tconstruct:volatile_int" | "tconstruct:modifier_slots");
            let action_kind = kind.to_string() == "tconstruct:tool_actions";
            let material_trait_kind = kind.to_string() == "tconstruct:material_traits";
            let trait_kind = material_trait_kind || kind.to_string() == "tconstruct:traits";
            let repair_kind = kind.to_string() == "tconstruct:material_repair" || kind.to_string() == "tconstruct:statless_part_repair";
            let effective_kind = kind.to_string() == "tconstruct:is_effective";
            let aoe_kind = matches!(kind.to_string().as_str(), "tconstruct:box_aoe" | "tconstruct:circle_aoe" | "tconstruct:vein_aoe" | "tconstruct:conditional_aoe" | "tconstruct:tree_aoe");
            let speed_kind = kind.to_string() == "tconstruct:mining_speed_modifier";
            let tier_kind = kind.to_string() == "tconstruct:max_tier";
            let missing_kind = matches!(kind.to_string().as_str(), "tconstruct:default_materials" | "tconstruct:remapping_materials");
            // PartStatsModule is MaterialStatsModule plus ToolPartsHook, with parts named by item.
            let parts_kind = kind.to_string() == "tconstruct:part_stats";
            let materials_kind = parts_kind || kind.to_string() == "tconstruct:material_stats";
            if !empty && !volatile_kind && !action_kind && !trait_kind && !repair_kind && !materials_kind && !effective_kind && !aoe_kind && !speed_kind && !tier_kind && !missing_kind && !["tconstruct:base_stats", "tconstruct:multiply_stats"].contains(&kind.to_string().as_str()) {
                return Err(format!("tool module {kind} has unimplemented hooks"));
            }
            let expected_hook = if volatile_kind { "tconstruct:volatile_data" }
                else if action_kind { "tconstruct:tool_actions" }
                else if trait_kind { "tconstruct:tool_traits" }
                else if repair_kind { "tconstruct:material_repair" }
                else if effective_kind { "tconstruct:is_effective" }
                else if aoe_kind { "tconstruct:aoe_iterator" }
                else if speed_kind { "tconstruct:mining_speed_modifier" }
                else if tier_kind { "tconstruct:mining_tier" }
                else if missing_kind { "tconstruct:missing_materials" } else { "tconstruct:tool_stats" };
            let hooks = match fields.get("hooks") {
                None | Some(Value::Null) => &[][..],
                Some(value) => value.as_array().ok_or("hooks must be an array")?.as_slice(),
            };
            for hook in hooks {
                let hook = Identifier::parse(hook.as_str().ok_or("hook must be an ID string")?).map_err(|error| error.to_string())?;
                let allowed = if materials_kind {
                    ["tconstruct:tool_stats", "tconstruct:tool_traits", "tconstruct:tool_materials", "tconstruct:material_repair", "tconstruct:volatile_data"]
                        .contains(&hook.to_string().as_str()) || parts_kind && hook.to_string() == "tconstruct:tool_parts"
                } else if trait_kind {
                    // ToolTraitHook has three bindings; only tool_traits feeds a rebuild.
                    matches!(hook.to_string().as_str(), "tconstruct:tool_traits" | "tconstruct:rebalanced_trait" | "tconstruct:trim_trait")
                } else { hook.to_string() == expected_hook };
                if empty || !allowed {
                    return Err(format!("module {kind} is invalid for hook {hook}"));
                }
            }
            // Missing/empty hooks select defaults. Repeated explicit hooks bind the
            // same module once (ModuleHookMap's LinkedHashMultimap), not once per ID.
            // A trait module bound to rebalanced/trim traits adds nothing to a rebuild itself: those hooks
            // are asked for by name through swappable_tool_traits, so the module is kept per hook.
            let mut feeds_rebuild = true;
            alternate_hooks.clear();
            if trait_kind && !hooks.is_empty() {
                let names: Vec<&str> = hooks.iter().filter_map(Value::as_str).collect();
                feeds_rebuild = names.contains(&"tconstruct:tool_traits");
                for alternate in ["tconstruct:rebalanced_trait", "tconstruct:trim_trait"] {
                    let alternate = Identifier::parse(alternate).expect("valid hook id");
                    if names.contains(&alternate.to_string().as_str()) { alternate_hooks.push(alternate); }
                }
            }
            if effective_kind {
                // IS_EFFECTIVE has no merger, so a second binding would be ambiguous.
                if effective.is_some() { return Err("only one effectiveness module may bind is_effective".into()); }
                effective = Some(crate::tool_mining::Effective::decode(fields)?);
            } else if aoe_kind {
                // AOE_ITERATOR has no merger, so a second binding would be ambiguous.
                if aoe.is_some() { return Err("only one area module may bind aoe_iterator".into()); }
                match crate::aoe::Iterator::decode(fields)? {
                    Some(iterator) => aoe = Some(iterator),
                    // A branch iterator this port does not implement: the module is recorded
                    // as unbound instead of rejecting the whole definition.
                    None => { if !unbound.contains(&kind) { unbound.push(kind); } }
                }
            } else if repair_kind && kind.to_string() == "tconstruct:statless_part_repair" {
                // StatlessPartRepairModule: IntLoadable FROM_ZERO part index, FROM_ONE amount.
                let Some(Value::Number(index)) = fields.get("part_index") else { return Err("part_index must be numeric".into()); };
                let index = crate::materials::number_int(&index.to_string());
                if index < 0 { return Err("part_index must be nonnegative".into()); }
                let Some(Value::Number(amount)) = fields.get("repair_amount") else { return Err("repair_amount must be numeric".into()); };
                let amount = crate::materials::number_int(&amount.to_string());
                if amount < 1 { return Err("repair_amount must be at least 1".into()); }
                repairs.push(RepairModule::Statless { index, amount });
            } else if repair_kind {
                let material = Identifier::parse(fields.get("material").and_then(Value::as_str).ok_or("material must be an ID string")?)
                    .map_err(|error| error.to_string())?;
                // EitherLoadable tries "durability" before "stat_type" and falls back to the constant form.
                let decoded = if fields.contains_key("stat_type") && !fields.contains_key("durability") {
                    let stat = Identifier::parse(fields.get("stat_type").and_then(Value::as_str).ok_or("stat_type must be an ID string")?)
                        .map_err(|error| error.to_string())?;
                    RepairModule::Stat { material, stat }
                } else {
                    let Some(Value::Number(amount)) = fields.get("durability") else { return Err("durability must be numeric".into()); };
                    let amount = crate::materials::number_int(&amount.to_string());
                    if amount < 1 { return Err("durability must be at least 1".into()); }
                    RepairModule::Constant { material, amount }
                };
                repairs.push(decoded);
            } else if speed_kind {
                // MiningSpeedModifierModule: the BlockPredicate is a flat loadable, so its
                // fields sit beside the module's own, keyed by `predicate_type`.
                let Some(Value::Number(modifier)) = fields.get("modifier") else { return Err("modifier must be numeric".into()); };
                let modifier = modifier.as_f64().ok_or("modifier must be numeric")? as f32;
                let mut predicate = fields.clone();
                predicate.remove("type");
                predicate.remove("hooks");
                predicate.remove("modifier");
                let predicate_type = predicate.remove("predicate_type").ok_or("predicate_type is required")?;
                predicate.insert("type".into(), predicate_type);
                let condition = crate::mining_speed::BlockCondition::decode(&Value::Object(predicate))?
                    .ok_or("mining speed predicate type is not supported")?;
                speed.push((condition, modifier));
            } else if tier_kind {
                // MaxTierModule: the tier must be a known vanilla tier (as for base_stats),
                // and composed modules keep the minimum.
                let tier = Identifier::parse(fields.get("tier").and_then(Value::as_str).ok_or("tier must be an ID string")?)
                    .map_err(|error| error.to_string())?;
                if tier.namespace() != "minecraft" || !["wood", "gold", "stone", "iron", "diamond", "netherite"].contains(&tier.path()) {
                    return Err(format!("unknown tool tier {tier}"));
                }
                max_tier = Some(match max_tier { Some(current) => crate::tool_stats::tier_min(&current, &tier), None => tier });
            } else if missing_kind {
                // MISSING_MATERIALS has no merger; a second binding would be ambiguous.
                if missing.is_some() { return Err("only one module may bind missing_materials".into()); }
                missing = Some(if kind.path() == "default_materials" {
                    MissingModule::Default(random_materials(fields, "materials")?)
                } else {
                    let remap = fields.get("remap").and_then(Value::as_array).filter(|maps| !maps.is_empty())
                        .ok_or("remap must be a nonempty array")?
                        .iter().map(|map| {
                            map.as_object().ok_or("remap entries must be objects")?.iter().map(|(from, to)| {
                                let from = Identifier::parse(from).map_err(|error| error.to_string())?;
                                let to = to.as_str().ok_or("remap target must be a material variant string")?;
                                let to = crate::tool_materials::MaterialVariant::parse(to)
                                    .ok_or_else(|| format!("invalid material variant {to}"))?;
                                Ok((from, to))
                            }).collect::<Result<Vec<_>, String>>()
                        }).collect::<Result<Vec<_>, String>>()?;
                    MissingModule::Remap { missing: random_materials(fields, "missing")?, remap }
                });
            } else if material_trait_kind {
                let Some(Value::Number(index)) = fields.get("material_index") else { return Err("material_index must be numeric".into()); };
                let index = crate::materials::number_int(&index.to_string());
                if index < 0 { return Err("material_index must be nonnegative".into()); }
                let stat = Identifier::parse(fields.get("stat_type").and_then(Value::as_str)
                    .ok_or("inferred material stat types require the unfinished tool_materials hook")?).map_err(|e| e.to_string())?;
                // MaterialTraitsModule is a class without value equality. Separate
                // JSON instances must both contribute even when their fields match.
                if feeds_rebuild { traits.push(TraitModule::Material { stat: stat.clone(), index }); }
                for hook in &alternate_hooks { hook_traits.entry(hook.clone()).or_default().push(TraitModule::Material { stat: stat.clone(), index }); }
            } else if trait_kind {
                let entries = fields.get("traits").and_then(Value::as_array).filter(|entries| !entries.is_empty())
                    .ok_or("traits must be a nonempty array")?;
                let decoded = entries.iter().map(crate::material_traits::decode_entry).collect::<Result<Vec<_>, _>>()?;
                // ToolTraitsModule is a record; ModifierEntry.equals compares ID/level,
                // not LazyModifier's optional flag. First equal module keeps its bindings.
                let same = |list: &[TraitModule]| list.iter().any(|previous| match previous {
                    TraitModule::Constant(previous) => previous.len() == decoded.len() && previous.iter().zip(&decoded)
                        .all(|(a,b)| a.id == b.id && a.level == b.level),
                    _ => false,
                });
                for hook in &alternate_hooks {
                    let list = hook_traits.entry(hook.clone()).or_default();
                    if !same(list) { list.push(TraitModule::Constant(decoded.clone())); }
                }
                if !same(&traits) && feeds_rebuild { traits.push(TraitModule::Constant(decoded)); }
            } else if action_kind {
                let entries = fields.get("tool_actions").and_then(Value::as_array).filter(|values| !values.is_empty())
                    .ok_or("tool_actions must be a nonempty array")?;
                for entry in entries {
                    // Forge action names are exact strings, not resource IDs or
                    // a closed enum. AnyMerger is equivalent to set union here.
                    let name = match entry {
                        Value::String(name) => name.clone(),
                        Value::Number(_) | Value::Bool(_) => entry.to_string(),
                        _ => return Err("tool action must be a JSON primitive".into()),
                    };
                    if name.encode_utf16().count() > i16::MAX as usize { return Err("tool action exceeds 32767 UTF-16 units".into()); }
                    actions.insert(name);
                }
            } else if kind.to_string() == "tconstruct:modifier_slots" {
                let slots = fields.get("slots").and_then(Value::as_object).filter(|slots| !slots.is_empty())
                    .ok_or("slots must be a nonempty object")?;
                let mut decoded = BTreeMap::new();
                for (name, value) in slots {
                    // SlotType's original regex allows the empty name, but not ':'
                    // or uppercase. Slot names are not resource identifiers.
                    if !name.bytes().all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'_')) {
                        return Err(format!("invalid modifier slot name {name}"));
                    }
                    let Value::Number(value) = value else { return Err("slot count must be numeric".into()); };
                    let count = crate::materials::number_int(&value.to_string());
                    if count < 1 { return Err("slot count must be positive".into()); }
                    decoded.insert(name.clone(), count);
                }
                let decoded = VolatileModule::Slots(decoded);
                if !volatile.contains(&decoded) { volatile.push(decoded); }
            } else if volatile_kind {
                let flag = Identifier::parse(fields.get("flag").and_then(Value::as_str).ok_or("flag must be an ID string")?)
                    .map_err(|error| error.to_string())?;
                let decoded = if kind.path() == "volatile_flag" { VolatileModule::Flag(flag) } else {
                    let Some(Value::Number(value)) = fields.get("value") else { return Err("volatile value must be numeric".into()); };
                    VolatileModule::Int(flag, crate::materials::number_int(&value.to_string()))
                };
                if !volatile.contains(&decoded) { volatile.push(decoded); }
            } else if materials_kind {
                let decoded = StatModule::decode(module, &crate::tool_parts::registry())?;
                let StatModule::Materials { types, parts: part_items, .. } = &decoded else { return Err("material_stats decoded to another module".into()); };
                // TOOL_MATERIALS has no merger: a second material module would be an ambiguous binding.
                if stats.iter().any(|previous| matches!(previous, StatModule::Materials { .. })) {
                    return Err("only one material stats module may bind tool_materials".into());
                }
                if let Some(items) = part_items {
                    // TOOL_PARTS also has no merger; a second parts binding is ambiguous.
                    if tool_parts.is_some() { return Err("only one part stats module may bind tool_parts".into()); }
                    tool_parts = Some(items.clone());
                }
                let stat_ids: Vec<Identifier> = types.iter().map(|(stat, _)| stat.clone()).collect();
                // Hooks bind in module order: stats, traits, volatile data, repair.
                traits.push(TraitModule::Positional(stat_ids.clone()));
                volatile.push(VolatileModule::MaterialLook { slots: stat_ids.len() });
                repairs.push(RepairModule::Parts(stat_ids));
                stats.push(decoded);
            } else if !empty {
                let decoded = StatModule::decode(module, &BTreeMap::new())?;
                // SetStatsModule/MultiplyStatsModule are Java records whose NBT
                // maps have value equality, so equal modules also deduplicate.
                if !stats.iter().any(|previous| same_stat_module(previous, &decoded)) { stats.push(decoded); }
            }
        }
        Ok(Self { stats, volatile, actions, traits, hook_traits, repairs, parts: tool_parts.unwrap_or_default(), effective, aoe, speed, max_tier, missing, unbound })
    }

    /// Definition traits in hook order. Resolution happens against the current
    /// modifier generation, never at JSON decode time.
    pub fn traits(&self, materials: &MaterialList, registry: &Materials) -> Result<Vec<crate::material_traits::ModifierEntry>, String> {
        Self::module_traits(&self.traits, materials, registry)
    }

    /// ToolTraitHook.addTraits for an alternate hook (rebalanced_trait, trim_trait): the same module
    /// kinds as the main hook, but only those bound to it.
    /// ToolDefinitionData.getHooks().hasHook: the definition binds an alternate trait hook (the main trait hook
    /// is bound by any definition with trait modules).
    pub fn has_hook(&self, hook: &Identifier) -> bool {
        self.hook_traits.contains_key(hook) || (hook.to_string() == "tconstruct:tool_traits" && !self.traits.is_empty())
    }
    pub fn hook_traits(&self, hook: &Identifier, materials: &MaterialList, registry: &Materials) -> Result<Vec<crate::material_traits::ModifierEntry>, String> {
        Self::module_traits(self.hook_traits.get(hook).map_or(&[][..], Vec::as_slice), materials, registry)
    }

    fn module_traits(modules: &[TraitModule], materials: &MaterialList, registry: &Materials) -> Result<Vec<crate::material_traits::ModifierEntry>, String> {
        let mut result = Vec::new();
        for module in modules {
            match module {
                TraitModule::Constant(entries) => result.extend(entries.iter().cloned()),
                TraitModule::Positional(stats) => {
                    for (index, stat) in stats.iter().enumerate().take(materials.as_slice().len()) {
                        let Some(id) = materials.get(index as i32).material_id() else { continue; };
                        for entry in registry.get_traits(&id, stat) {
                            result.push(entry.clone().ok_or_else(|| format!("null material trait in {id}/{stat}"))?);
                        }
                    }
                }
                TraitModule::Material { stat, index } => {
                    let material = materials.get(*index);
                    let Some(id) = material.material_id() else { continue; };
                    if id.to_string() == "tconstruct:unknown" || stat.to_string() == "tconstruct:missingno" { continue; }
                    for entry in registry.get_traits(&id, stat) {
                        result.push(entry.clone().ok_or_else(|| format!("null material trait in {id}/{stat}"))?);
                    }
                }
            }
        }
        Ok(result)
    }

    /// MaterialRepairToolHook (MaxMerger over every module); modifier repair hooks do not exist yet.
    pub fn is_repair_material(&self, tool: &MaterialList, material: &Identifier) -> bool { self.repairs.iter().any(|module| module.is_for(tool, material)) }
    /// The largest amount any module grants for this material, 0 when none applies.
    pub fn repair_amount(&self, tool: &MaterialList, material: &Identifier, registry: &Materials) -> f32 {
        self.repairs.iter().fold(0.0_f32, |best, module| { let amount = module.amount(tool, material, registry); if amount > best { amount } else { best } })
    }
    /// addRepairMaterials: fixed modules list their material; stat modules list the tool's known,
    /// non-render materials at repairable positions.
    pub fn repair_materials(&self, tool: &MaterialList) -> BTreeSet<Identifier> {
        let mut result = BTreeSet::new();
        for module in &self.repairs {
            match module {
                RepairModule::Constant { material, .. } | RepairModule::Stat { material, .. } => { result.insert(material.clone()); }
                RepairModule::Parts(stats) => for index in repair_positions(stats) {
                    if let Some(id) = tool.get(index as i32).material_id() {
                        if id.to_string() != "tconstruct:unknown" && id.to_string() != "tconstruct:ui_render" { result.insert(id); }
                    }
                },
                // StatlessPartRepairModule.addRepairMaterials: the material at the part index,
                // unless it is unknown or the render-only material.
                RepairModule::Statless { index, .. } => {
                    if let Some(id) = tool.get(*index).material_id() {
                        if id.to_string() != "tconstruct:unknown" && id.to_string() != "tconstruct:ui_render" { result.insert(id); }
                    }
                }
            }
        }
        result
    }
    /// The per-stack `minecraft:tool` component for this definition's mining behavior, None for
    /// tools with no effectiveness module (they mine as the item does).
    pub fn mining_component(&self, broken: bool, mining_speed: f32, tier: &Identifier) -> Result<Option<NbtCompound>, String> {
        self.effective.as_ref().map(|effective| crate::tool_mining::tool_component(effective, broken, mining_speed, tier)).transpose()
    }
    /// The definition's effectiveness module, if it has one.
    pub fn effective(&self) -> Option<&crate::tool_mining::Effective> { self.effective.as_ref() }
    /// MaxTierModule: the cap this definition puts on the harvest tier, if it declares one.
    pub fn max_tier(&self) -> Option<&Identifier> { self.max_tier.as_ref() }
    /// MiningSpeedModifierModule (MiningSpeedToolHook.modifyDestroySpeed): every module whose
    /// block predicate matches multiplies the speed, in module order. Definition modules run
    /// before modifier break-speed modules, as in the original hook order.
    pub fn modify_mining_speed(&self, context: &crate::mining_speed::Context, speed: f32) -> f32 {
        self.speed.iter().fold(speed, |speed, (condition, modifier)| {
            if condition.matches(context.block, context.block_in_tag) { speed * modifier } else { speed }
        })
    }
    /// MissingMaterialsToolHook: the filled material list when the tool holds fewer materials
    /// than it has stat types (needsMaterials), else None. The caller writes the result back
    /// before rebuilding, as ToolStack.ensureHasData does.
    pub fn fill_materials(&self, existing: &MaterialList, registry: &Materials, roll: &mut dyn FnMut() -> f32) -> Option<MaterialList> {
        let types = self.material_stat_types()?;
        let stats: Vec<&Identifier> = types.iter().map(|(stat, _)| stat).collect();
        let old = existing.as_slice();
        if old.len() >= stats.len() { return None; }
        // RandomMaterial.build: module entries by position, first-with-stat beyond them.
        let build = |entries: &[RandomMaterial], roll: &mut dyn FnMut() -> f32| -> Vec<crate::tool_materials::MaterialVariant> {
            (0..stats.len()).map(|index| match entries.get(index) {
                Some(entry) => entry.pick(stats[index], registry, roll),
                None => first_with_stat(registry, stats[index]),
            }).collect()
        };
        let filled = match &self.missing {
            // The hook's default instance builds from firsts; the interface's default
            // fillMaterials(definition, existing, random) then keeps the existing prefix.
            None => old.iter().cloned().chain(build(&[], roll).into_iter().skip(old.len())).collect(),
            Some(MissingModule::Default(entries)) => {
                let mut result: Vec<crate::tool_materials::MaterialVariant> = old.to_vec();
                let default_size = entries.len().min(stats.len());
                for index in old.len()..default_size { result.push(entries[index].pick(stats[index], registry, roll)); }
                // The original appends firsts for default_size..stats even when existing
                // materials already pass default_size; the quirk is kept verbatim.
                for index in default_size..stats.len() { result.push(first_with_stat(registry, stats[index])); }
                result
            }
            Some(MissingModule::Remap { missing, remap }) => {
                let built = build(missing, roll);
                if old.is_empty() { built } else {
                    let mut result: Vec<crate::tool_materials::MaterialVariant> = old.iter().enumerate().map(|(index, material)| {
                        if let Some(map) = remap.get(index) {
                            if let Some(id) = material.material_id() {
                                if let Some((_, replacement)) = map.iter().find(|(from, _)| *from == id) { return replacement.clone(); }
                            }
                        }
                        material.clone()
                    }).collect();
                    result.extend(built.into_iter().skip(old.len()));
                    result
                }
            }
        };
        Some(MaterialList::from_variants(filled))
    }
    /// Modules this definition declares that are recorded but not implemented (see `UNBOUND`).
    pub fn unbound_modules(&self) -> &[Identifier] { &self.unbound }
    /// ToolHooks.AOE_ITERATOR: the definition's area iterator, when it binds one.
    pub fn aoe(&self) -> Option<&crate::aoe::Iterator> { self.aoe.as_ref() }
    /// ToolPartsHook.parts: the part items in slot order (empty for tools without a parts module).
    pub fn tool_parts(&self) -> &[Identifier] { &self.parts }
    /// ToolMaterialHook.getStatTypes with the scale of each position.
    pub fn material_stat_types(&self) -> Option<&[(Identifier, f32)]> {
        self.stats.iter().find_map(|module| match module { StatModule::Materials { types, .. } => Some(types.as_slice()), _ => None })
    }

    /// Definition hook only. ModifierUtil's broken-tool guard and modifier
    /// contributions belong to its caller, not this unconditional module.
    pub fn allows_action(&self, action: &str) -> bool { self.actions.contains(action) }

    /// A fresh definition contribution on every rebuild, never yesterday's cache.
    /// Modifier/material hooks will extend this stage when those modules are ported.
    pub fn build_volatile(&self) -> NbtCompound { self.build_volatile_for(&MaterialList::default(), &Materials::default()) }

    pub fn build_volatile_for(&self, materials: &MaterialList, registry: &Materials) -> NbtCompound {
        let mut data = NbtCompound::new();
        for module in &self.volatile {
            match module {
                VolatileModule::Flag(flag) => { data.insert(flag.to_string(), Nbt::Byte(1)); }
                VolatileModule::Int(flag, value) => {
                    // This stage starts empty and only writes byte/int tags.
                    let previous = data.get(&flag.to_string()).and_then(Nbt::as_i64).unwrap_or(0) as i32;
                    data.insert(flag.to_string(), Nbt::Int(previous.wrapping_add(*value)));
                }
                VolatileModule::MaterialLook { slots } => {
                    // Shiny if any material is tagged; keep only the rarest rarity (RarityModule.setRarity).
                    let shiny = Identifier::new("tconstruct", "shiny").unwrap();
                    let mut is_shiny = false;
                    let mut rarest: Option<crate::materials::Rarity> = None;
                    for index in 0..*slots {
                        let Some(id) = materials.get(index as i32).material_id() else { continue; };
                        if registry.reverse_tags.get(&id).is_some_and(|tags| tags.contains(&shiny)) { is_shiny = true; }
                        let rarity = registry.materials.get(registry.resolve_redirect(&id)).map(|m| m.rarity).unwrap_or(crate::materials::Rarity::Common);
                        rarest = Some(rarest.map_or(rarity, |previous| if rarity.ordinal() > previous.ordinal() { rarity } else { previous }));
                    }
                    if is_shiny { data.insert("tconstruct:shiny".into(), Nbt::Byte(1)); }
                    if let Some(rarity) = rarest {
                        let current = crate::modifier_entries::nbt_int(data.get("tconstruct:rarity"));
                        if rarity.ordinal() > current { data.insert("tconstruct:rarity".into(), Nbt::Int(rarity.ordinal())); }
                    }
                }
                VolatileModule::Slots(slots) => for (name, count) in slots {
                    let previous = data.get(name).and_then(Nbt::as_i64).unwrap_or(0) as i32;
                    let total = previous.wrapping_add(*count);
                    // ToolDataNBT.setSlots differs from ModDataNBT.putInt:
                    // a zero count removes the key rather than storing zero.
                    if total == 0 { data.remove(name); }
                    else { data.insert(name.clone(), Nbt::Int(total)); }
                },
            }
        }
        data
    }

    pub fn build_stats(&self, materials: &MaterialList, registry: &Materials) -> Builder {
        let mut builder = Builder::default();
        for module in &self.stats { module.apply(&mut builder, materials, registry); }
        builder
    }
}

fn same_stat_module(left: &StatModule, right: &StatModule) -> bool {
    let numbers_equal = |a: &[(crate::tool_stats::Stat, f32)], b: &[(crate::tool_stats::Stat, f32)]| {
        a.len() == b.len() && a.iter().zip(b).all(|((sa, a), (sb, b))|
            sa == sb && (a.to_bits() == b.to_bits() || a.is_nan() && b.is_nan()))
    };
    match (left, right) {
        (StatModule::Base { numbers: a, tier: ta }, StatModule::Base { numbers: b, tier: tb }) => ta == tb && numbers_equal(a, b),
        (StatModule::Multiply(a), StatModule::Multiply(b)) => numbers_equal(a, b),
        _ => false,
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Definitions {
    pub generation: u64,
    /// Presence distinguishes a loaded empty JSON definition from missing/failed data.
    pub data: BTreeMap<Identifier, Data>,
    pub problems: Vec<String>,
}
impl Definitions {
    fn load(registered: &BTreeSet<Identifier>, files: &[(Identifier, Value)]) -> Self {
        let files: BTreeMap<_, _> = files.iter().map(|(id, value)| (id, value)).collect();
        let mut result = Self::default();
        for id in registered {
            match files.get(id).ok_or_else(|| format!("missing tool definition {id}"))
                .and_then(|value| Data::decode(value).map_err(|error| format!("{id}: {error}"))) {
                Ok(data) => { result.data.insert(id.clone(), data); }
                Err(error) => result.problems.push(error),
            }
        }
        result
    }
}

/// ToolDefinitions and ArmorDefinitions, excluding other-mod integration.
/// These are definition identities, not item registrations (slime_chestplate is
/// intentionally different from the original item's slimy_chestplate identity).
pub(crate) fn registered() -> BTreeSet<Identifier> {
    let mut ids: BTreeSet<_> = [
        "pickaxe", "sledge_hammer", "vein_hammer", "mattock", "pickadze", "excavator",
        "hand_axe", "broad_axe", "kama", "scythe", "dagger", "sword", "cleaver",
        "crossbow", "longbow", "fishing_rod", "javelin", "arrow", "shuriken", "throwing_axe",
        "flint_and_brick", "sky_staff", "earth_staff", "ichor_staff", "ender_staff",
        "melting_pan", "war_pick", "battlesign", "swasher",
        "travelers_shield", "plate_shield", "slime_wings",
    ].into_iter().map(|path| Identifier::new("tconstruct", path).unwrap()).collect();
    for family in ["travelers", "plate", "slime"] {
        for slot in ["helmet", "chestplate", "leggings", "boots"] {
            ids.insert(Identifier::new("tconstruct", &format!("{family}_{slot}")).unwrap());
        }
    }
    ids
}

static SERVER: OnceLock<Arc<RwLock<Definitions>>> = OnceLock::new();
pub(crate) fn register() -> &'static Arc<RwLock<Definitions>> {
    SERVER.get_or_init(|| {
        let state = Arc::new(RwLock::new(Definitions::default()));
        let output = state.clone();
        let ids = registered();
        ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
            Identifier::new("tconstruct", "tool_definitions").unwrap(), FOLDER, move |files| {
                let mut next = Definitions::load(&ids, files);
                for error in &next.problems { eprintln!("Tinkers: {error}"); }
                let mut previous = output.write().map_err(|_| "tool definition registry poisoned")?;
                next.generation = previous.generation.wrapping_add(1);
                *previous = next;
                Ok(())
            },
        ).expect("Tinkers tool definition listener registration failed");
        let stopped = state.clone();
        ServerLifecycleEvents::SERVER_STOPPED.register(move |_| {
            let mut state = stopped.write().expect("tool definition registry poisoned");
            let generation = state.generation.wrapping_add(1);
            *state = Definitions { generation, ..Definitions::default() };
        });
        state
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use crate::tool_stats::Stat;

    #[test]
    fn material_trait_hooks_use_positions_and_preserve_distinct_class_instances() {
        let data = Data::decode(&json!({"modules":[
            {"type":"tconstruct:material_traits","material_index":0,"stat_type":"tconstruct:head"},
            {"type":"tconstruct:material_traits","material_index":0,"stat_type":"tconstruct:head","hooks":["tconstruct:tool_traits","tconstruct:tool_traits"]},
            {"type":"tconstruct:material_traits","material_index":5,"stat_type":"tconstruct:head"}
        ]})).unwrap();
        let id = Identifier::parse("test:wood").unwrap();
        let mut registry = Materials::default();
        registry.traits = crate::material_traits::load(&[pluto_lib::JsonResourceStack { id: id.clone(), layers: vec![
            Ok(json!({"default":[{"name":"test:trait","level":2}]})),
        ] }], &mut registry.problems);
        let materials = MaterialList::from_nbt(Some(&Nbt::List(vec![Nbt::String("test:wood#grain".into())])));
        assert_eq!(data.traits(&materials, &registry).unwrap().iter().map(|entry| entry.level).collect::<Vec<_>>(), vec![2,2]);
        assert!(data.traits(&MaterialList::default(), &registry).unwrap().is_empty());
        registry.traits.get_mut(&id).unwrap().default = vec![None];
        assert!(data.traits(&materials, &registry).is_err(), "null traits cannot silently become no effect");
        for module in [
            json!({"type":"tconstruct:material_traits","material_index":-1,"stat_type":"tconstruct:head"}),
            json!({"type":"tconstruct:material_traits","material_index":0}),
        ] { assert!(Data::decode(&json!({"modules":[module]})).is_err()); }
    }

    #[test]
    fn material_repair_uses_the_largest_module_and_repairable_stats_only() {
        let data = Data::decode(&json!({"modules":[
            {"type":"tconstruct:material_repair","material":"test:wood","durability":10},
            {"type":"tconstruct:material_repair","material":"test:wood","stat_type":"tconstruct:head"},
            {"type":"tconstruct:material_repair","material":"test:stone","stat_type":"tconstruct:handle"},
            {"type":"tconstruct:material_repair","material":"test:ghost","stat_type":"tconstruct:head"}
        ]})).unwrap();
        let wood = Identifier::parse("test:wood").unwrap();
        let mut registry = Materials::default();
        registry.stats = crate::material_stats::load(&[pluto_lib::JsonResourceStack { id: wood.clone(), layers: vec![
            Ok(json!({"stats":{"tconstruct:head":{"durability":35},"tconstruct:handle":{"durability":0.5}}})),
        ] }, pluto_lib::JsonResourceStack { id: Identifier::parse("test:stone").unwrap(), layers: vec![
            Ok(json!({"stats":{"tconstruct:handle":{"durability":0.5}}})),
        ] }], &mut registry.problems);
        assert!(data.is_repair_material(&MaterialList::default(), &wood));
        assert!(!data.is_repair_material(&MaterialList::default(), &Identifier::parse("test:other").unwrap()));
        assert_eq!(data.repair_amount(&MaterialList::default(), &wood, &registry), 35.0, "stat-derived 35 beats the constant 10");
        // Handle durability is a multiplier, not IRepairableMaterialStats: it grants nothing.
        assert_eq!(data.repair_amount(&MaterialList::default(), &Identifier::parse("test:stone").unwrap(), &registry), 0.0);
        // Registered stats only: a missing stat record never falls back to the type default.
        assert_eq!(data.repair_amount(&MaterialList::default(), &Identifier::parse("test:ghost").unwrap(), &registry), 0.0);
        assert_eq!(data.repair_amount(&MaterialList::default(), &Identifier::parse("test:other").unwrap(), &registry), 0.0);
        assert_eq!(data.repair_materials(&MaterialList::default()).len(), 3);
        for invalid in [
            json!({"type":"tconstruct:material_repair","durability":3}),
            json!({"type":"tconstruct:material_repair","material":"test:wood"}),
            json!({"type":"tconstruct:material_repair","material":"test:wood","durability":0}),
            json!({"type":"tconstruct:material_repair","material":"test:wood","durability":3,"hooks":["tconstruct:tool_stats"]}),
        ] { assert!(Data::decode(&json!({"modules":[invalid]})).is_err()); }
    }

    #[test]
    fn material_stats_module_binds_traits_volatile_data_and_repair_by_position() {
        let data = Data::decode(&json!({"modules":[
            {"type":"tconstruct:material_stats","stat_types":["tconstruct:head","tconstruct:handle","tconstruct:binding"]}
        ]})).unwrap();
        assert_eq!(data.material_stat_types().unwrap().len(), 3);
        let (wood, stone) = (Identifier::parse("test:wood").unwrap(), Identifier::parse("test:stone").unwrap());
        let mut registry = Materials::default();
        registry.traits = crate::material_traits::load(&[pluto_lib::JsonResourceStack { id: wood.clone(), layers: vec![
            Ok(json!({"default":[{"name":"test:grain"}],"stats":{"tconstruct:head":[{"name":"test:hard"}]}})),
        ] }], &mut registry.problems);
        registry.stats = crate::material_stats::load(&[pluto_lib::JsonResourceStack { id: wood.clone(), layers: vec![
            Ok(json!({"stats":{"tconstruct:head":{"durability":30}}})),
        ] }, pluto_lib::JsonResourceStack { id: stone.clone(), layers: vec![
            Ok(json!({"stats":{"tconstruct:head":{"durability":90}}})),
        ] }], &mut registry.problems);
        for (id, rarity) in [(&wood, crate::materials::Rarity::Common), (&stone, crate::materials::Rarity::Epic)] {
            registry.materials.insert(id.clone(), crate::materials::Material { id: id.clone(), tier: 1, sort_order: 1, rarity, craftable: true, hidden: false });
        }
        registry.reverse_tags.insert(wood.clone(), [Identifier::parse("tconstruct:shiny").unwrap()].into_iter().collect());
        // Head holds stone (repairable 90); handle holds wood (not repairable); binding is unlisted data.
        let tool = MaterialList::from_nbt(Some(&Nbt::List(vec![Nbt::String("test:stone".into()), Nbt::String("test:wood".into())])));
        // Traits: only min(materials, stat types) positions, per (material, stat) with default fallback.
        assert_eq!(data.traits(&tool, &registry).unwrap().len(), 1, "wood's default at the handle position");
        assert!(data.traits(&MaterialList::default(), &registry).unwrap().is_empty());
        // Volatile: shiny from any tagged material, the rarest rarity of the declared positions.
        let volatile = data.build_volatile_for(&tool, &registry);
        assert_eq!(volatile["tconstruct:shiny"], Nbt::Byte(1));
        assert_eq!(volatile["tconstruct:rarity"], Nbt::Int(3));
        // Repair: only repairable stat positions count, and the amount is the registered durability.
        assert!(data.is_repair_material(&tool, &stone));
        assert!(!data.is_repair_material(&tool, &wood), "the handle is not a repairable stat type");
        assert_eq!(data.repair_amount(&tool, &stone, &registry), 90.0);
        assert_eq!(data.repair_amount(&tool, &wood, &registry), 0.0);
        assert_eq!(data.repair_materials(&tool), [stone.clone()].into_iter().collect());
        // Decode contract.
        for invalid in [
            json!({"modules":[{"type":"tconstruct:material_stats","stat_types":["tconstruct:head"]},{"type":"tconstruct:material_stats","stat_types":["tconstruct:limb"]}]}),
            json!({"modules":[{"type":"tconstruct:material_stats","stat_types":["tconstruct:head"],"hooks":["tconstruct:tool_parts"]}]}),
            json!({"modules":[{"type":"tconstruct:material_stats","stat_types":[]}]}),
        ] { assert!(Data::decode(&invalid).is_err(), "{invalid}"); }
        assert!(Data::decode(&json!({"modules":[{"type":"tconstruct:material_stats","stat_types":["tconstruct:head"],
            "hooks":["tconstruct:tool_stats","tconstruct:material_repair"]}]})).is_ok());
    }

    #[test]
    fn part_stats_name_registered_part_items_and_bind_tool_parts_once() {
        let data = Data::decode(&json!({"modules":[
            {"type":"tconstruct:part_stats","parts":["tconstruct:pick_head",{"item":"tconstruct:tool_binding","scale":0.5},"tconstruct:tough_handle"],"primary_part":2}
        ]})).unwrap();
        assert_eq!(data.tool_parts().iter().map(Identifier::to_string).collect::<Vec<_>>(),
            vec!["tconstruct:pick_head", "tconstruct:tool_binding", "tconstruct:tough_handle"]);
        let types = data.material_stat_types().unwrap();
        assert_eq!((types[0].0.to_string(), types[1].0.to_string(), types[2].0.to_string(), types[1].1),
            ("tconstruct:head".into(), "tconstruct:binding".into(), "tconstruct:handle".into(), 0.5));
        assert!(Data::decode(&json!({"modules":[{"type":"tconstruct:material_stats","stat_types":["tconstruct:head"]}]})).unwrap().tool_parts().is_empty());
        assert!(Data::decode(&json!({"modules":[
            {"type":"tconstruct:part_stats","parts":["tconstruct:pick_head"]},
            {"type":"tconstruct:part_stats","parts":["tconstruct:hammer_head"]}]})).is_err());
        assert!(Data::decode(&json!({"modules":[{"type":"tconstruct:material_stats","stat_types":["tconstruct:head"],"hooks":["tconstruct:tool_parts"]}]})).is_err(),
            "only part_stats binds tool_parts");
    }

    #[test]
    fn unbound_modules_are_listed_not_validated_and_tool_parts_is_real() {
        let data = Data::decode(&json!({"modules":[
            {"type":"tconstruct:part_stats","parts":["tconstruct:pick_head"]},
            {"type":"tconstruct:melee_particle","anything":"not checked"},
            {"type":"tconstruct:fixed_material_name","index":0},
            {"type":"tconstruct:melee_particle"}
        ]})).unwrap();
        assert_eq!(data.unbound_modules().iter().map(Identifier::to_string).collect::<Vec<_>>(),
            vec!["tconstruct:melee_particle", "tconstruct:fixed_material_name"], "deduplicated, in order");
        assert!(Data::decode(&json!({"modules":[{"type":"tconstruct:unknown_module"}]})).is_err(), "unlisted modules still reject");
        let shield = Data::decode(&json!({"modules":[{"type":"tconstruct:tool_parts","parts":["tconstruct:shield_core"]}]})).unwrap();
        assert_eq!(shield.tool_parts().len(), 1);
        for bad in [
            json!({"modules":[{"type":"tconstruct:tool_parts","parts":[]}]}),
            json!({"modules":[{"type":"tconstruct:tool_parts","parts":["tconstruct:no_such_part"]}]}),
            json!({"modules":[{"type":"tconstruct:tool_parts","parts":["tconstruct:maille"]},{"type":"tconstruct:part_stats","parts":["tconstruct:pick_head"]}]}),
        ] { assert!(Data::decode(&bad).is_err(), "{bad}"); }
    }

    #[test]
    fn trait_modules_keep_order_and_java_entry_equality() {
        let data = Data::decode(&json!({"modules":[
            {"type":"tconstruct:traits","traits":[{"name":"test:first","optional":true}]},
            {"type":"tconstruct:traits","traits":[{"name":"test:first"}],"hooks":["tconstruct:tool_traits"]},
            {"type":"tconstruct:traits","traits":[{"name":"test:first","level":2},{"name":"test:second"}]}
        ]})).unwrap();
        let traits = data.traits(&MaterialList::default(), &Materials::default()).unwrap();
        assert_eq!(traits.iter().map(|entry| entry.level).collect::<Vec<_>>(), vec![1,2,1]);
        assert!(traits[0].optional, "first equal record wins despite different optional flags");
        for invalid in [json!([]), json!([null]), json!([{"name":"test:x","level":0}])] {
            assert!(Data::decode(&json!({"modules":[{"type":"tconstruct:traits","traits":invalid}]})).is_err());
        }
    }

    #[test]
    fn alternate_trait_hooks_are_kept_per_hook_and_do_not_feed_the_rebuild() {
        let data = Data::decode(&json!({"modules":[
            {"type":"tconstruct:traits","traits":[{"name":"test:main"}]},
            {"type":"tconstruct:traits","traits":[{"name":"test:rebalanced","level":2}],"hooks":["tconstruct:rebalanced_trait"]},
            {"type":"tconstruct:traits","traits":[{"name":"test:both"}],"hooks":["tconstruct:tool_traits","tconstruct:trim_trait"]}
        ]})).unwrap();
        let ids = |entries: Vec<crate::material_traits::ModifierEntry>| entries.iter().map(|entry| (entry.id.to_string(), entry.level)).collect::<Vec<_>>();
        let hook = |name: &str| Identifier::parse(name).unwrap();
        let (materials, registry) = (MaterialList::default(), Materials::default());
        assert_eq!(ids(data.traits(&materials, &registry).unwrap()), vec![("test:main".into(), 1), ("test:both".into(), 1)]);
        assert_eq!(ids(data.hook_traits(&hook("tconstruct:rebalanced_trait"), &materials, &registry).unwrap()), vec![("test:rebalanced".into(), 2)]);
        assert_eq!(ids(data.hook_traits(&hook("tconstruct:trim_trait"), &materials, &registry).unwrap()), vec![("test:both".into(), 1)]);
        assert!(data.unbound_modules().is_empty());
    }

    #[test]
    fn action_hooks_union_exact_names_and_validate_whole_definitions() {
        let data = Data::decode(&json!({"modules":[
            {"type":"tconstruct:tool_actions","tool_actions":["axe_strip","axe_strip","Custom Action","",true]},
            {"type":"tconstruct:tool_actions","tool_actions":["shears_dig","axe_strip"],"hooks":["tconstruct:tool_actions"]}
        ]})).unwrap();
        for name in ["axe_strip", "shears_dig", "Custom Action", "", "true"] { assert!(data.allows_action(name)); }
        assert!(!data.allows_action("custom action"));
        assert!(!data.allows_action("shield_block"));
        assert!(!Data::default().allows_action("axe_strip"));
        for invalid in [
            json!({"type":"tconstruct:tool_actions","tool_actions":[]}),
            json!({"type":"tconstruct:tool_actions","tool_actions":[null]}),
            json!({"type":"tconstruct:tool_actions","tool_actions":[{}]}),
            json!({"type":"tconstruct:tool_actions","tool_actions":["axe_strip"],"hooks":["tconstruct:tool_stats"]}),
            json!({"type":"tconstruct:tool_actions","tool_actions":["😀".repeat(16384)]}),
        ] { assert!(Data::decode(&json!({"modules":[invalid]})).is_err()); }
    }

    #[test]
    fn modifier_slots_validate_names_deduplicate_maps_and_remove_zero_totals() {
        let data = Data::decode(&json!({"modules":[
            {"type":"tconstruct:modifier_slots","slots":{"upgrades":2147483647}},
            {"type":"tconstruct:modifier_slots","slots":{"upgrades":2147483647,"abilities":1}},
            {"type":"tconstruct:modifier_slots","slots":{"abilities":1,"upgrades":2147483647}},
            {"type":"tconstruct:modifier_slots","slots":{"upgrades":2,"custom_2":3,"":1}}
        ]})).unwrap();
        let values = data.build_volatile();
        assert!(!values.contains_key("upgrades"), "wrapping total zero removes the slot key");
        assert_eq!(values["abilities"], Nbt::Int(1), "map order does not change record equality");
        assert_eq!(values["custom_2"], Nbt::Int(3));
        assert_eq!(values[""], Nbt::Int(1), "original SlotType regex accepts empty names");
        for slots in [json!({}), json!({"upgrades":0}), json!({"upgrades":-1}), json!({"bad:name":1}), json!({"Upper":1})] {
            assert!(Data::decode(&json!({"modules":[{"type":"tconstruct:modifier_slots","slots":slots}]})).is_err());
        }
    }

    #[test]
    fn volatile_hooks_preserve_order_record_equality_overflow_and_fresh_rebuilds() {
        let data = Data::decode(&json!({"modules":[
            {"type":"tconstruct:volatile_int","flag":"test:count","value":5},
            {"type":"tconstruct:volatile_flag","flag":"test:count"},
            {"type":"tconstruct:volatile_int","flag":"test:count","value":2,
             "hooks":["tconstruct:volatile_data","tconstruct:volatile_data"]},
            {"type":"tconstruct:volatile_int","flag":"test:count","value":2},
            {"type":"tconstruct:volatile_int","flag":"test:overflow","value":2147483647},
            {"type":"tconstruct:volatile_int","flag":"test:overflow","value":1},
            {"type":"tconstruct:volatile_int","flag":"test:zero","value":0}
        ]})).unwrap();
        let mut first = data.build_volatile();
        assert_eq!(first["test:count"], Nbt::Int(3), "flag overwrites then the distinct integer module adds once");
        assert_eq!(first["test:overflow"], Nbt::Int(i32::MIN));
        assert_eq!(first["test:zero"], Nbt::Int(0), "putInt retains an explicit zero");
        first.insert("test:count".into(), Nbt::Int(99));
        assert_eq!(data.build_volatile()["test:count"], Nbt::Int(3));
        assert!(Data::decode(&json!({"modules":[{"type":"tconstruct:volatile_int","flag":"test:x"}]})).is_err());
        assert!(Data::decode(&json!({"modules":[{"type":"tconstruct:volatile_flag","flag":"test:x","hooks":["tconstruct:tool_stats"]}]})).is_err());
    }

    #[test]
    fn hooks_use_defaults_deduplicate_and_reject_partial_definitions() {
        let data = Data::decode(&json!({"modules":[
            {"type":"tconstruct:base_stats", "stats":{"tconstruct:durability":10}, "hooks":[]},
            {"type":"tconstruct:multiply_stats", "multipliers":{"tconstruct:durability":2},
             "hooks":["tconstruct:tool_stats","tconstruct:tool_stats"]},
            {"type":"tconstruct:base_stats", "stats":{"tconstruct:durability":10.0}},
            {"type":"tconstruct:multiply_stats", "multipliers":{"tconstruct:durability":2}},
            {"type":"tconstruct:empty"}
        ]})).unwrap();
        assert_eq!(data.build_stats(&MaterialList::default(), &Materials::default()).get(Stat::Durability), 20.0);
        for invalid in [
            json!({"modules":[{"type":"tconstruct:empty","hooks":["tconstruct:tool_stats"]}]}),
            json!({"modules":[{"type":"tconstruct:base_stats","stats":{},"hooks":["tconstruct:tool_traits"]}]}),
            json!({"modules":[{"type":"tconstruct:part_stats","parts":["tconstruct:no_such_part"]}]}),
            json!({"modules":{}}),
        ] { assert!(Data::decode(&invalid).is_err(), "{invalid}"); }
    }

    #[test]
    fn mining_speed_modifier_multiplies_only_matching_blocks() {
        let data = Data::decode(&json!({"modules":[
            {"type":"tconstruct:mining_speed_modifier","predicate_type":"mantle:set","blocks":["minecraft:cobweb"],"modifier":7.5},
            {"type":"tconstruct:mining_speed_modifier","predicate_type":"mantle:and","predicates":[
                {"type":"mantle:tag","tag":"minecraft:mineable/axe"},
                {"type":"mantle:inverted","inverted_type":"mantle:tag","tag":"tconstruct:mineable/mattock"}
            ],"modifier":2.0}
        ]})).unwrap();
        assert!(data.unbound_modules().is_empty());
        let cobweb = Identifier::parse("minecraft:cobweb").unwrap();
        let log = Identifier::parse("minecraft:oak_log").unwrap();
        let no_tags = |_: &Identifier| false;
        let axe_only = |tag: &Identifier| tag.to_string() == "minecraft:mineable/axe";
        let axe_and_mattock = |tag: &Identifier| matches!(tag.to_string().as_str(), "minecraft:mineable/axe" | "tconstruct:mineable/mattock");
        let cobweb_context = crate::mining_speed::Context { block: &cobweb, block_in_tag: &no_tags, original: 1.0, multiplier: 1.0, effective: true, on_ground: None, eyes_in_water: None, facts: crate::mining_speed::BlockFacts::default() };
        assert_eq!(data.modify_mining_speed(&cobweb_context, 4.0), 30.0, "cobweb set matches");
        let axe_context = crate::mining_speed::Context { block: &log, block_in_tag: &axe_only, original: 1.0, multiplier: 1.0, effective: true, on_ground: None, eyes_in_water: None, facts: crate::mining_speed::BlockFacts::default() };
        assert_eq!(data.modify_mining_speed(&axe_context, 4.0), 8.0, "axe tag and not mattock tag");
        let both_context = crate::mining_speed::Context { block: &log, block_in_tag: &axe_and_mattock, original: 1.0, multiplier: 1.0, effective: true, on_ground: None, eyes_in_water: None, facts: crate::mining_speed::BlockFacts::default() };
        assert_eq!(data.modify_mining_speed(&both_context, 4.0), 4.0, "the inverted tag excludes it");
        let plain_context = crate::mining_speed::Context { block: &log, block_in_tag: &no_tags, original: 1.0, multiplier: 1.0, effective: true, on_ground: None, eyes_in_water: None, facts: crate::mining_speed::BlockFacts::default() };
        assert_eq!(data.modify_mining_speed(&plain_context, 4.0), 4.0);
        for invalid in [
            json!({"type":"tconstruct:mining_speed_modifier","predicate_type":"mantle:set","blocks":["minecraft:cobweb"]}),
            json!({"type":"tconstruct:mining_speed_modifier","blocks":["minecraft:cobweb"],"modifier":2.0}),
            json!({"type":"tconstruct:mining_speed_modifier","predicate_type":"mantle:block_state","modifier":2.0}),
            json!({"type":"tconstruct:mining_speed_modifier","predicate_type":"mantle:set","blocks":["minecraft:cobweb"],"modifier":2.0,"hooks":["tconstruct:tool_stats"]}),
        ] { assert!(Data::decode(&json!({"modules":[invalid]})).is_err(), "{invalid}"); }
    }

    #[test]
    fn max_tier_keeps_the_minimum_and_validates_the_tier() {
        let data = Data::decode(&json!({"modules":[{"type":"tconstruct:max_tier","tier":"minecraft:gold"}]})).unwrap();
        assert_eq!(data.max_tier().map(Identifier::to_string), Some("minecraft:gold".into()));
        assert!(data.unbound_modules().is_empty());
        let composed = Data::decode(&json!({"modules":[
            {"type":"tconstruct:max_tier","tier":"minecraft:diamond"},
            {"type":"tconstruct:max_tier","tier":"minecraft:stone"}
        ]})).unwrap();
        assert_eq!(composed.max_tier().map(Identifier::to_string), Some("minecraft:stone".into()));
        for invalid in [
            json!({"type":"tconstruct:max_tier","tier":"minecraft:emerald"}),
            json!({"type":"tconstruct:max_tier","tier":"tconstruct:gold"}),
            json!({"type":"tconstruct:max_tier"}),
            json!({"type":"tconstruct:max_tier","tier":"minecraft:gold","hooks":["tconstruct:tool_stats"]}),
        ] { assert!(Data::decode(&json!({"modules":[invalid]})).is_err(), "{invalid}"); }
    }

    #[test]
    fn statless_part_repair_uses_the_material_at_the_part_index() {
        let data = Data::decode(&json!({"modules":[
            {"type":"tconstruct:statless_part_repair","part_index":1,"repair_amount":120}
        ]})).unwrap();
        assert!(data.unbound_modules().is_empty());
        let (wood, stone) = (Identifier::parse("test:wood").unwrap(), Identifier::parse("test:stone").unwrap());
        let tool = MaterialList::from_nbt(Some(&Nbt::List(vec![Nbt::String("test:stone".into()), Nbt::String("test:wood#grain".into())])));
        assert!(data.is_repair_material(&tool, &wood), "the base material of the variant at index 1");
        assert!(!data.is_repair_material(&tool, &stone), "index 0 is not the repair part");
        assert_eq!(data.repair_amount(&tool, &wood, &Materials::default()), 120.0);
        assert_eq!(data.repair_amount(&tool, &stone, &Materials::default()), 0.0);
        assert_eq!(data.repair_materials(&tool), [wood.clone()].into_iter().collect());
        assert!(data.repair_materials(&MaterialList::default()).is_empty(), "unknown at the index offers nothing");
        for invalid in [
            json!({"type":"tconstruct:statless_part_repair","part_index":-1,"repair_amount":10}),
            json!({"type":"tconstruct:statless_part_repair","part_index":0,"repair_amount":0}),
            json!({"type":"tconstruct:statless_part_repair","repair_amount":10}),
            json!({"type":"tconstruct:statless_part_repair","part_index":0,"repair_amount":10,"hooks":["tconstruct:tool_stats"]}),
        ] { assert!(Data::decode(&json!({"modules":[invalid]})).is_err(), "{invalid}"); }
    }

    fn fill_registry() -> Materials {
        let (wood, stone) = (Identifier::parse("test:wood").unwrap(), Identifier::parse("test:stone").unwrap());
        let mut registry = Materials::default();
        registry.stats = crate::material_stats::load(&[
            pluto_lib::JsonResourceStack { id: wood.clone(), layers: vec![
                Ok(json!({"stats":{"tconstruct:head":{"durability":30},"tconstruct:handle":{"durability":0.5}}})),
            ] },
            pluto_lib::JsonResourceStack { id: stone.clone(), layers: vec![
                Ok(json!({"stats":{"tconstruct:head":{"durability":90},"tconstruct:handle":{"durability":0.5}}})),
            ] },
        ], &mut registry.problems);
        for (id, tier, sort_order) in [(&wood, 1, 1), (&stone, 2, 2)] {
            registry.materials.insert(id.clone(), crate::materials::Material {
                id: id.clone(), tier, sort_order, rarity: crate::materials::Rarity::Common, craftable: true, hidden: false,
            });
        }
        registry.visible = vec![wood, stone];
        registry
    }

    fn material_names(list: &MaterialList) -> Vec<String> { list.as_slice().iter().map(ToString::to_string).collect() }

    #[test]
    fn default_materials_fill_missing_positions_and_keep_existing() {
        let registry = fill_registry();
        let data = Data::decode(&json!({"modules":[
            {"type":"tconstruct:material_stats","stat_types":["tconstruct:head","tconstruct:handle"]},
            {"type":"tconstruct:default_materials","materials":[
                {"type":"tconstruct:fixed","material":"test:stone"},
                {"type":"tconstruct:random","tier":1}
            ]}
        ]})).unwrap();
        assert!(data.unbound_modules().is_empty());
        // From scratch: fixed head, then a tier-1 material with handle stats (wood).
        let filled = data.fill_materials(&MaterialList::default(), &registry, &mut || 0.0).unwrap();
        assert_eq!(material_names(&filled), vec!["test:stone", "test:wood"]);
        // Existing materials are kept; only the missing position is filled.
        let existing = MaterialList::from_variants(vec![crate::tool_materials::MaterialVariant::parse("test:wood").unwrap()]);
        let filled = data.fill_materials(&existing, &registry, &mut || 0.0).unwrap();
        assert_eq!(material_names(&filled), vec!["test:wood", "test:wood"]);
        // A full list needs no fill.
        let full = MaterialList::from_variants(vec![
            crate::tool_materials::MaterialVariant::parse("test:wood").unwrap(),
            crate::tool_materials::MaterialVariant::parse("test:stone").unwrap(),
        ]);
        assert!(data.fill_materials(&full, &registry, &mut || 0.0).is_none());
        // Without a module, the hook's default instance fills with first-with-stat.
        let plain = Data::decode(&json!({"modules":[
            {"type":"tconstruct:material_stats","stat_types":["tconstruct:head","tconstruct:handle"]}
        ]})).unwrap();
        let filled = plain.fill_materials(&existing, &registry, &mut || 0.0).unwrap();
        assert_eq!(material_names(&filled), vec!["test:wood", "test:wood"], "wood is first with both stats");
        for invalid in [
            json!({"type":"tconstruct:default_materials","materials":[]}),
            json!({"type":"tconstruct:default_materials","materials":[{"type":"tconstruct:random","material":"test:wood"}]}),
            json!({"type":"tconstruct:default_materials","materials":[{"type":"tconstruct:random_variant"}]}),
            json!({"type":"tconstruct:default_materials","materials":[{"type":"tconstruct:first"}],
                  "hooks":["tconstruct:tool_stats"]}),
        ] { assert!(Data::decode(&json!({"modules":[
            {"type":"tconstruct:material_stats","stat_types":["tconstruct:head"]}, invalid,
        ]})).is_err(), "{invalid}"); }
        // Two missing-materials bindings are ambiguous (the hook has no merger).
        assert!(Data::decode(&json!({"modules":[
            {"type":"tconstruct:material_stats","stat_types":["tconstruct:head"]},
            {"type":"tconstruct:default_materials","materials":[{"type":"tconstruct:first"}]},
            {"type":"tconstruct:default_materials","materials":[{"type":"tconstruct:first"}]},
        ]})).is_err());
    }

    #[test]
    fn remapping_materials_rewrite_existing_positions_then_fill() {
        let registry = fill_registry();
        let data = Data::decode(&json!({"modules":[
            {"type":"tconstruct:material_stats","stat_types":["tconstruct:head","tconstruct:handle"]},
            {"type":"tconstruct:remapping_materials",
             "missing":[{"type":"tconstruct:fixed","material":"test:wood"},{"type":"tconstruct:fixed","material":"test:stone"}],
             "remap":[{"test:wood":"test:stone"}]}
        ]})).unwrap();
        assert!(data.unbound_modules().is_empty());
        // From scratch the remap does not apply; the missing list builds the tool.
        let filled = data.fill_materials(&MaterialList::default(), &registry, &mut || 0.0).unwrap();
        assert_eq!(material_names(&filled), vec!["test:wood", "test:stone"]);
        // An existing wood head is remapped to stone; the handle comes from the built list.
        let existing = MaterialList::from_variants(vec![crate::tool_materials::MaterialVariant::parse("test:wood#grain").unwrap()]);
        let filled = data.fill_materials(&existing, &registry, &mut || 0.0).unwrap();
        assert_eq!(material_names(&filled), vec!["test:stone", "test:stone"]);
        // An existing material the map does not name stays as it is.
        let existing = MaterialList::from_variants(vec![crate::tool_materials::MaterialVariant::parse("test:stone").unwrap()]);
        let filled = data.fill_materials(&existing, &registry, &mut || 0.0).unwrap();
        assert_eq!(material_names(&filled), vec!["test:stone", "test:stone"]);
        for invalid in [
            json!({"type":"tconstruct:remapping_materials","missing":[{"type":"tconstruct:first"}],"remap":[]}),
            json!({"type":"tconstruct:remapping_materials","missing":[],"remap":[{"test:wood":"test:stone"}]}),
            json!({"type":"tconstruct:remapping_materials","remap":[{"test:wood":"test:stone"}]}),
        ] { assert!(Data::decode(&json!({"modules":[
            {"type":"tconstruct:material_stats","stat_types":["tconstruct:head"]}, invalid,
        ]})).is_err(), "{invalid}"); }
    }
}
