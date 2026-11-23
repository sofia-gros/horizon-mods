//! Mob equipment replacements (`tinkering/mob_equipment`, SlimeKnights MobEquipmentManager/MobEquipment, MIT; see LICENSE):
//! when a mob spawns, each replacement targeting its type rolls its chance; the ones that pass replace what the mob
//! holds in a slot with a built tool, if what it holds there matches. The manager there cancels the spawn
//! finalization and finalizes the mob itself so its item is not overwritten; here the spawn callback's completion
//! runs after the inherited initialization by construction, so the replacement simply lands on top, and the match
//! is tested there against the item the initialization left, as the original tests it after its own finalize.
//!
//! The rolls and the tool builds are drawn during preparation from the mob's own stream, as every spawn callback
//! here works; the original draws them at apply time, after its manual finalize — the same stream, a different
//! point in it, so the outcomes agree in distribution rather than draw for draw.
//!
//! The tank fill (`fluid`) lands through the tool tank: a passed entry naming a fluid tag fills the built
//! tool's tank with a random amount of a random fluid from the tag, drawn from the mob's stream during
//! preparation like the rest (see `build_replacement`).
//!
//! One part of the original does not land yet:
//! - files carrying `conditions` are skipped: the one shipped file with any asks for another mod
//!   (`forge:mod_loaded` twilightforest, for the minotaur axe), which is not present, so skipping it is the
//!   condition's own answer; any other condition shape is reported as a problem rather than guessed at.
use crate::{common, melee_hook::target_in_tag, tool_materials::{MaterialList, MaterialVariant}};
use pluto_lib::{Identifier, JsonResourceStack, MobSpawnAction, MobSpawnCallback, MobSpawnCompletion, MobSpawnPreparation, RegistryLookup, ResourceManagerHelper, ResourceType, ServerLifecycleEvents, StackData};
use serde_json::Value;
use std::sync::{Arc, RwLock};

pub(crate) const FOLDER: &str = "tinkering/mob_equipment";

/// Which entities a replacement targets: one type, or every type in a tag (`#` in the file).
#[derive(Clone, Debug, PartialEq)]
enum Target {
    Type(Identifier),
    Tag(Identifier),
}

impl Target {
    fn matches(&self, kind: &Identifier, entities: Option<&RegistryLookup>) -> bool {
        match self {
            Target::Type(id) => id == kind,
            Target::Tag(tag) => target_in_tag(entities, Some(kind), tag),
        }
    }
}

/// What the item in the slot must be for the replacement to land (Mantle's item predicate, the shapes the
/// shipped files use): anything, or one of a set of items. An empty hand reads as `minecraft:air`.
#[derive(Clone, Debug, PartialEq)]
enum Match {
    Any,
    Set(Vec<Identifier>),
}

impl Match {
    fn matches(&self, item: &str) -> bool {
        match self {
            Match::Any => true,
            Match::Set(items) => items.iter().any(|id| id.to_string() == item),
        }
    }
}

/// How one material slot of the built tool is chosen (RandomMaterial, the shapes the shipped files use).
#[derive(Clone, Copy, Debug, PartialEq)]
enum RandomMat {
    /// `tconstruct:ancient`: a uniform material with the stat, never a loot-excluded one.
    Ancient,
    /// `tconstruct:first`: the first material with the stat.
    First,
}

/// One slot's replacement (MobEquipment).
#[derive(Clone, Debug, PartialEq)]
struct Equip {
    slot: String,
    matches: Match,
    tool: Identifier,
    count: u32,
    materials: Vec<RandomMat>,
    /// The fluid tag the built tool's tank is filled from (MobEquipment's `fluid`).
    fluid_tag: Option<Identifier>,
    chance: f32,
    priority: i32,
}

/// One file's replacements: the entities it targets and the slot replacements it carries.
#[derive(Clone, Debug, PartialEq)]
struct Replacement {
    targets: Vec<Target>,
    equips: Vec<Equip>,
}

/// The loaded replacements of the last reload.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Replacements {
    list: Vec<Replacement>,
}

impl Replacements {
    /// Parses the folder's files with MobEquipmentManager's rules; a file that fails is reported and skipped,
    /// never half-applied.
    pub fn load(stacks: &[JsonResourceStack]) -> (Self, Vec<String>) {
        let mut problems = Vec::new();
        let mut list = Vec::new();
        for stack in stacks {
            let Some(value) = stack.layers.last() else { continue };
            let value = match value {
                Ok(value) => value,
                Err(error) => {
                    problems.push(format!("mob equipment {}: {error}", stack.id));
                    continue;
                }
            };
            match parse_file(value) {
                Ok(Some(replacement)) => list.push(replacement),
                Ok(None) => problems.push(format!("mob equipment {}: skipped, its conditions do not hold here", stack.id)),
                Err(error) => problems.push(format!("mob equipment {}: {error}", stack.id)),
            }
        }
        (Self { list }, problems)
    }

    /// The replacements for one spawning type, highest priority first (the manager sorts each type's list so;
    /// equal priorities keep file order, as its stable sort does).
    fn for_type(&self, kind: &Identifier, entities: Option<&RegistryLookup>) -> Vec<&Equip> {
        let mut equips: Vec<&Equip> = self.list.iter()
            .filter(|replacement| replacement.targets.iter().any(|target| target.matches(kind, entities)))
            .flat_map(|replacement| &replacement.equips)
            .collect();
        equips.sort_by(|a, b| b.priority.cmp(&a.priority));
        equips
    }
}

fn parse_file(value: &Value) -> Result<Option<Replacement>, String> {
    let object = value.as_object().ok_or("the file is not a JSON object")?;
    // The shipped conditioned file asks for a mod that is not present; no condition is evaluated beyond that
    // answer, so any conditioned file is skipped (the module docs record this).
    if object.get("conditions").and_then(Value::as_array).is_some_and(|conditions| !conditions.is_empty()) {
        return Ok(None);
    }
    let entity = object.get("entity").ok_or("no entity")?;
    let mut targets = Vec::new();
    match entity {
        Value::String(text) => targets.push(parse_target(text)?),
        Value::Array(values) => {
            for value in values {
                // The manager reads an array as plain type names; tags are not supported there.
                let text = value.as_str().ok_or("entity entries must be strings")?;
                if text.starts_with('#') { return Err("tags are not supported in an entity array".into()); }
                targets.push(Target::Type(parse_id(text, "entity")?));
            }
        }
        _ => return Err("entity must be a string or an array of strings".into()),
    }
    let equip = object.get("equip").ok_or("no equip")?;
    let mut equips = Vec::new();
    match equip {
        Value::Object(_) => equips.push(parse_equip(equip)?),
        Value::Array(values) => {
            for value in values { equips.push(parse_equip(value)?); }
        }
        _ => return Err("equip must be an object or an array of objects".into()),
    }
    Ok(Some(Replacement { targets, equips }))
}

fn parse_target(text: &str) -> Result<Target, String> {
    if let Some(tag) = text.strip_prefix('#') {
        Ok(Target::Tag(parse_id(tag, "entity tag")?))
    } else {
        Ok(Target::Type(parse_id(text, "entity")?))
    }
}

fn parse_id(text: &str, what: &str) -> Result<Identifier, String> {
    Identifier::parse(text).map_err(|error| format!("bad {what} {text:?}: {error}"))
}

fn parse_equip(value: &Value) -> Result<Equip, String> {
    let object = value.as_object().ok_or("an equip entry is not a JSON object")?;
    let slot = object.get("slot").and_then(Value::as_str).ok_or("an equip entry has no slot")?.to_owned();
    let matches = match object.get("match") {
        None => Match::Any,
        Some(predicate) => {
            let predicate = predicate.as_object().ok_or("match is not an object")?;
            match predicate.get("type").and_then(Value::as_str) {
                Some("mantle:set") => {
                    let items = predicate.get("items").and_then(Value::as_array).ok_or("a set match has no items")?;
                    let mut set = Vec::new();
                    for item in items {
                        set.push(parse_id(item.as_str().ok_or("a set item is not a string")?, "match item")?);
                    }
                    Match::Set(set)
                }
                other => return Err(format!("unsupported match type {other:?}")),
            }
        }
    };
    let (tool, count) = match object.get("tool").ok_or("an equip entry has no tool")? {
        Value::String(text) => (parse_id(text, "tool")?, 1),
        Value::Object(output) => {
            let item = output.get("item").and_then(Value::as_str).ok_or("a tool output has no item")?;
            let count = output.get("count").and_then(Value::as_i64).unwrap_or(1).clamp(0, i64::from(u32::MAX)) as u32;
            (parse_id(item, "tool")?, count)
        }
        _ => return Err("tool must be an item id or an output object".into()),
    };
    let mut materials = Vec::new();
    if let Some(values) = object.get("materials") {
        for value in values.as_array().ok_or("materials is not an array")? {
            materials.push(match value.get("type").and_then(Value::as_str) {
                Some("tconstruct:ancient") => RandomMat::Ancient,
                Some("tconstruct:first") => RandomMat::First,
                other => return Err(format!("unsupported random material type {other:?}")),
            });
        }
    }
    let fluid_tag = match object.get("fluid") {
        None => None,
        Some(Value::String(text)) => Some(parse_id(text, "fluid tag")?),
        Some(_) => return Err("fluid must be a tag id".into()),
    };
    let chance = object.get("chance").and_then(Value::as_f64).unwrap_or(0.05) as f32;
    let priority = object.get("priority").and_then(Value::as_i64).unwrap_or(100) as i32;
    Ok(Equip { slot, matches, tool, count, materials, fluid_tag, chance, priority })
}

static LOADED: RwLock<Option<Arc<Replacements>>> = RwLock::new(None);

/// The replacements of the last reload, empty before the first.
pub(crate) fn replacements() -> Arc<Replacements> {
    LOADED.read().ok().and_then(|loaded| loaded.clone()).unwrap_or_default()
}

/// Declares the mob equipment folder and publishes its replacements on each reload.
pub(crate) fn register_data() -> Result<(), String> {
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "mob_equipment").map_err(|e| e.to_string())?, FOLDER,
        |_| {
            let stacks = ResourceManagerHelper::reload_json_stacks(FOLDER).ok_or("mob equipment folder was not captured")?;
            let (loaded, problems) = Replacements::load(&stacks);
            for problem in &problems { eprintln!("Tinkers: {problem}"); }
            *LOADED.write().map_err(|_| "mob equipment poisoned")? = Some(Arc::new(loaded));
            Ok(())
        },
    )?;
    ServerLifecycleEvents::SERVER_STOPPED.register(|_| { if let Ok(mut loaded) = LOADED.write() { *loaded = None; } });
    Ok(())
}

/// Registers the spawn callback. Called once from the product's hooks.
pub(crate) fn register() {
    MobSpawnCallback::register(Identifier::parse("tconstruct:mob_equipment").expect("callback id"), |view| prepare(view, &replacements()))
        .unwrap_or_else(|error| panic!("registering the mob equipment spawn callback: {error}"));
}

/// MobEquipmentManager.finalizeSpawn: roll each replacement for the spawning type in priority order; the ones
/// that pass are built here, and land at completion where what the mob holds can be matched.
fn prepare(view: &mut dyn MobSpawnPreparation, loaded: &Replacements) -> Result<Option<MobSpawnAction>, String> {
    if loaded.list.is_empty() { return Ok(None); }
    let kind = view.info().entity_type.clone();
    let entities = common::entity_type_snapshot();
    let equips = loaded.for_type(&kind, entities.as_deref());
    if equips.is_empty() { return Ok(None); }
    let mut passed: Vec<(Equip, StackData)> = Vec::new();
    for equip in equips {
        if view.next_float() < equip.chance {
            if let Some(stack) = build_replacement(view, equip) { passed.push((equip.clone(), stack)); }
        }
    }
    if passed.is_empty() { return Ok(None); }
    Ok(Some(Box::new(move |completion: &mut dyn MobSpawnCompletion| {
        for (equip, stack) in &passed {
            // MobEquipment.apply: the replacement lands only on what the slot's item matches; an empty
            // slot reads as air, as an empty ItemStack's item does there.
            let current = completion.equipment(&equip.slot)?.map(|stack| stack.item).unwrap_or_else(|| "minecraft:air".to_owned());
            if equip.matches.matches(&current) { completion.set_equipment(&equip.slot, stack.clone())?; }
        }
        Ok(())
    })))
}

/// The replacement stack for one passed entry: a tool built with the entry's random materials when the item is
/// a tool whose definition is loaded (MobEquipment.apply's ToolStack path), the plain stack otherwise. A tool
/// whose registries are not loaded is not fabricated; the entry is skipped for this spawn.
fn build_replacement(view: &mut dyn MobSpawnPreparation, equip: &Equip) -> Option<StackData> {
    let definitions = crate::tool_definitions::register().read().ok()?;
    let Some(definition) = definitions.data.get(&equip.tool) else {
        return Some(StackData { item: equip.tool.to_string(), count: equip.count, ..Default::default() });
    };
    let materials = common::register().read().ok()?;
    if !materials.server_loaded { return None; }
    let modifiers = common::server_modifiers()?.read().ok()?;
    if !modifiers.definitions_loaded { return None; }
    let stat_types = definition.material_stat_types()?;
    let tags = common::item_snapshot();
    let env = crate::tool_stack::Environment {
        definition: Some(definition), has_materials: true, materials: &materials, modifiers: &modifiers,
        item_tags_loaded: tags.is_some(), item: Some(&equip.tool), item_tags: tags.as_deref(),
    };
    // RandomMaterial.build over the definition's stat types: the entry's materials in order, and any stat
    // past them takes the first material with the stat, as there.
    let mut variants = Vec::new();
    for (index, (stat, _scale)) in stat_types.iter().enumerate() {
        let chosen = match equip.materials.get(index) {
            Some(RandomMat::Ancient) => crate::slime_entities::ancient_material(&materials, stat, view)?,
            Some(RandomMat::First) | None => materials.first_with_stat(stat).map(|material| material.id.clone())?,
        };
        variants.push(MaterialVariant::parse(&chosen.to_string())?);
    }
    let mut tool = crate::tool_build::create_tool(&env, &MaterialList::from_variants(variants), false).ok()?;
    // MobEquipment's fluid fill: a random amount up to the tank's capacity (`nextInt(capacity + 1)`) of a
    // random fluid from the entry's tag, both drawn from the mob's stream. A tool without a tank reads
    // capacity 0 and is given empty, as there; a tag with no captured members fills nothing.
    if let Some(tag) = &equip.fluid_tag {
        let amount = view.next_int(crate::tool_tank::capacity(&tool) + 1).unwrap_or(0);
        if amount > 0 {
            if let Some(members) = common::fluid_snapshot().and_then(|fluids| fluids.get_tag(tag).cloned()) {
                if !members.is_empty() {
                    let pick = view.next_int(members.len() as i32).unwrap_or(0);
                    if let Some(fluid) = members.iter().nth(pick as usize) {
                        crate::tool_tank::set_fluid(&mut tool, Some(&pluto_lib::FluidStack::new(fluid.clone(), amount as u32)));
                    }
                }
            }
        }
    }
    Some(StackData { item: equip.tool.to_string(), count: equip.count, custom_data: tool, ..Default::default() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn stack(id: &str, value: Value) -> JsonResourceStack {
        JsonResourceStack { id: Identifier::parse(id).unwrap(), layers: vec![Ok(value)] }
    }

    const PIGLINS: &str = r##"{"entity":"#tconstruct:spawns_with/battle_sign","equip":{"chance":0.05,"match":{"type":"mantle:set","items":["minecraft:golden_sword","minecraft:golden_axe"]},"materials":[{"type":"tconstruct:ancient"},{"type":"tconstruct:ancient"}],"priority":100,"slot":"mainhand","tool":"tconstruct:battlesign"}}"##;
    const DROWNED: &str = r##"{"entity":"#tconstruct:spawns_with/swasher/drowned","equip":{"chance":0.05,"fluid":"tconstruct:swasher/drowned","match":{"type":"mantle:set","items":["minecraft:air"]},"materials":[{"type":"tconstruct:ancient"},{"type":"tconstruct:ancient"},{"type":"tconstruct:ancient"}],"priority":100,"slot":"mainhand","tool":"tconstruct:swasher"}}"##;
    const MINOTAUR: &str = r##"{"conditions":[{"type":"forge:mod_loaded","modid":"twilightforest"}],"entity":"twilightforest:minotaur","equip":{"chance":0.05,"materials":[{"type":"tconstruct:ancient"}],"priority":100,"slot":"mainhand","tool":"tconstruct:minotaur_axe"}}"##;

    #[test]
    fn the_shipped_files_parse_with_the_managers_defaults() {
        let (loaded, problems) = Replacements::load(&[
            stack("tconstruct:piglins", serde_json::from_str(PIGLINS).unwrap()),
            stack("tconstruct:drowned", serde_json::from_str(DROWNED).unwrap()),
            stack("tconstruct:minotaur", serde_json::from_str(MINOTAUR).unwrap()),
        ]);
        assert_eq!(loaded.list.len(), 2, "the conditioned minotaur file is skipped: {problems:?}");
        assert_eq!(problems.len(), 1);
        let piglins = &loaded.list[0];
        assert_eq!(piglins.targets, vec![Target::Tag(Identifier::parse("tconstruct:spawns_with/battle_sign").unwrap())]);
        let equip = &piglins.equips[0];
        assert_eq!(equip.slot, "mainhand");
        assert_eq!(equip.tool, Identifier::parse("tconstruct:battlesign").unwrap());
        assert_eq!(equip.matches, Match::Set(vec![Identifier::parse("minecraft:golden_sword").unwrap(), Identifier::parse("minecraft:golden_axe").unwrap()]));
        assert_eq!(equip.materials, vec![RandomMat::Ancient, RandomMat::Ancient]);
        assert_eq!(equip.chance, 0.05);
        assert_eq!(equip.priority, 100);
        let drowned = &loaded.list[1].equips[0];
        assert_eq!(drowned.fluid_tag, Some(Identifier::parse("tconstruct:swasher/drowned").unwrap()), "the fluid tag is parsed and kept");
        assert_eq!(drowned.materials.len(), 3);
    }

    #[test]
    fn defaults_and_the_array_forms_parse() {
        let (loaded, problems) = Replacements::load(&[stack("tconstruct:custom", json!({
            "entity": ["minecraft:zombie", "minecraft:husk"],
            "equip": [{ "slot": "offhand", "tool": "tconstruct:melting_pan" }],
        }))]);
        assert!(problems.is_empty(), "{problems:?}");
        let replacement = &loaded.list[0];
        assert_eq!(replacement.targets.len(), 2, "an entity array names plain types");
        let equip = &replacement.equips[0];
        assert_eq!(equip.matches, Match::Any, "no match predicate matches anything");
        assert_eq!(equip.chance, 0.05, "the default chance");
        assert_eq!(equip.priority, 100, "the default priority");
        assert!(equip.materials.is_empty());
    }

    #[test]
    fn replacements_for_a_type_come_highest_priority_first() {
        let (loaded, _) = Replacements::load(&[
            stack("tconstruct:low", json!({ "entity": "minecraft:zombie", "equip": { "slot": "mainhand", "tool": "tconstruct:war_pick", "priority": 50 } })),
            stack("tconstruct:high", json!({ "entity": "minecraft:zombie", "equip": { "slot": "offhand", "tool": "tconstruct:melting_pan", "priority": 150 } })),
            stack("tconstruct:other", json!({ "entity": "minecraft:husk", "equip": { "slot": "mainhand", "tool": "tconstruct:war_pick" } })),
        ]);
        let zombie = Identifier::parse("minecraft:zombie").unwrap();
        let equips = loaded.for_type(&zombie, None);
        assert_eq!(equips.len(), 2);
        assert_eq!(equips[0].slot, "offhand", "priority 150 runs before 50");
        assert_eq!(equips[1].slot, "mainhand");
        // A tag target matches nothing without the entity snapshot, as before its first capture at runtime.
        let (tagged, _) = Replacements::load(&[stack("tconstruct:piglins", serde_json::from_str(PIGLINS).unwrap())]);
        let piglin = Identifier::parse("minecraft:piglin").unwrap();
        assert!(tagged.for_type(&piglin, None).is_empty());
    }

    struct FakeSpawn {
        info: pluto_lib::MobSpawnInfo,
        floats: std::collections::VecDeque<f32>,
        held: std::collections::BTreeMap<String, StackData>,
        set: Vec<(String, StackData)>,
    }

    impl FakeSpawn {
        fn for_type(kind: &str, floats: &[f32]) -> Self {
            Self {
                info: pluto_lib::MobSpawnInfo {
                    entity_type: Identifier::parse(kind).unwrap(), dimension: Identifier::parse("minecraft:overworld").unwrap(),
                    position: [0.0; 3], reason: "natural".into(), special_multiplier: 0.0,
                },
                floats: floats.iter().copied().collect(), held: Default::default(), set: Vec::new(),
            }
        }
    }

    impl MobSpawnPreparation for FakeSpawn {
        fn info(&self) -> &pluto_lib::MobSpawnInfo { &self.info }
        fn next_float(&mut self) -> f32 { self.floats.pop_front().unwrap_or(1.0) }
        fn next_double(&mut self) -> f64 { self.next_float() as f64 }
        fn next_int(&mut self, bound: i32) -> Result<i32, String> { Ok(0.min(bound - 1)) }
        fn equipment(&self, slot: &str) -> Result<Option<StackData>, String> { Ok(self.held.get(slot).cloned()) }
    }

    impl MobSpawnCompletion for FakeSpawn {
        fn set_equipment(&mut self, slot: &str, stack: StackData) -> Result<(), String> {
            self.set.push((slot.to_owned(), stack));
            Ok(())
        }
    }

    #[test]
    fn a_passed_roll_replaces_only_what_the_slot_matches() {
        let (loaded, _) = Replacements::load(&[stack("tconstruct:piglins2", json!({
            "entity": "minecraft:piglin",
            "equip": { "slot": "mainhand", "tool": "minecraft:stone", "chance": 0.05,
                       "match": { "type": "mantle:set", "items": ["minecraft:golden_sword", "minecraft:golden_axe"] } },
        }))]);
        // The roll passes (0.01 < 0.05) and the piglin holds a golden sword: the replacement lands.
        let mut view = FakeSpawn::for_type("minecraft:piglin", &[0.01]);
        view.held.insert("mainhand".into(), StackData { item: "minecraft:golden_sword".into(), count: 1, ..Default::default() });
        let action = prepare(&mut view, &loaded).unwrap().expect("a passed roll prepares work");
        action(&mut view).unwrap();
        assert_eq!(view.set.len(), 1);
        assert_eq!(view.set[0].1.item, "minecraft:stone");
        // The same roll against a stone sword: the match fails and nothing is set.
        let mut view = FakeSpawn::for_type("minecraft:piglin", &[0.01]);
        view.held.insert("mainhand".into(), StackData { item: "minecraft:stone_sword".into(), count: 1, ..Default::default() });
        let action = prepare(&mut view, &loaded).unwrap().expect("the roll still passed");
        action(&mut view).unwrap();
        assert!(view.set.is_empty(), "a non-matching item is left alone");
        // A failed roll prepares nothing at all.
        let mut view = FakeSpawn::for_type("minecraft:piglin", &[0.5]);
        assert!(prepare(&mut view, &loaded).unwrap().is_none());
        // A type with no replacements draws nothing.
        let mut view = FakeSpawn::for_type("minecraft:creeper", &[]);
        assert!(prepare(&mut view, &loaded).unwrap().is_none());
    }

    #[test]
    fn an_empty_hand_matches_air() {
        let (loaded, _) = Replacements::load(&[stack("tconstruct:drowned2", json!({
            "entity": "minecraft:drowned",
            "equip": { "slot": "mainhand", "tool": "minecraft:stone", "chance": 1.0,
                       "match": { "type": "mantle:set", "items": ["minecraft:air"] } },
        }))]);
        let mut view = FakeSpawn::for_type("minecraft:drowned", &[0.0]);
        let action = prepare(&mut view, &loaded).unwrap().expect("chance 1.0 always passes");
        action(&mut view).unwrap();
        assert_eq!(view.set.len(), 1, "an empty slot reads as air and matches");
    }
}
