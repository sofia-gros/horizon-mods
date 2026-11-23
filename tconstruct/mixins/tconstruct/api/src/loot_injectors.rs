//! Mantle's loot-table injections, from LootTableInjectionProvider (SlimeKnights, MIT; see LICENSE):
//! a `mantle/loot_injectors/<name>.json` names a vanilla table (`name`) and the pools to add entries
//! to (`pools`, each named for a pool of the target). At table load the entries merge into the
//! target's pools; the whole injection is skipped when its top-level conditions do not hold
//! (a config prop, a mod being loaded), evaluated against the live config at that load.
//!
//! The merge translates the entries from the vanilla loot dialect into the host's, because the host
//! reads neither `functions` nor the `tconstruct:tool_part` entry type:
//! * an entry's `functions` become its `modifier` — one function as-is, several under a
//!   `minecraft:sequence` — and each function's `function` key becomes `type`, the host's key;
//! * a `tconstruct:tool_part` entry becomes a `minecraft:tag` entry over the part tag, expanded so
//!   the roll picks one part uniformly (the original picks one tag member uniformly too), carrying
//!   a `tconstruct:tool_part_material` function with the entry's material, which sets the part's
//!   material when the roll lands;
//! * pool names resolve by the vanilla rule — the first pool is `main`, the rest are `pool1`,
//!   `pool2`, ... — because the host's tables carry no pool names. A pool the target does not
//!   have is appended whole, rolling once (the vanilla LootPool builder's default), so the
//!   injection still lands.
//!
//! The functions run through PlutoLib's loot-function event, with draws from the roll's own seed
//! (see `LootFunctionContext.seed`): `tconstruct:add_tool_data` (AddToolDataFunction), `mantle:set_fluid`
//! (SetFluidLootFunction) and `tconstruct:tool_part_material` (the tool-part half above). The game's
//! own functions (`minecraft:set_count`, `minecraft:set_damage`) run natively in the host.
use crate::{
    common,
    conditions::Context as ConditionContext,
    materials::Conditions as _,
    tool_definitions, tool_damage, tool_stack, tool_tank,
};
use pluto_lib::{
    FluidItems, FluidStack, Identifier, ItemSlot, LootFunctionContext, LootTableEvents,
    ResourceManagerHelper, ResourceType, ServerLifecycleEvents,
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

/// The folder under the mod's data the injector files live in.
pub(crate) const FOLDER: &str = "mantle/loot_injectors";

/// The synthesized function kind the tool-part translation carries (see `translate_entry`).
const TOOL_PART_MATERIAL: &str = "tconstruct:tool_part_material";

/// One injector file, decoded: its target table, the conditions gating the whole injection, and
/// its pools with the entries as written (translated at merge time).
struct Injector {
    target: String,
    conditions: Vec<crate::conditions::Condition>,
    pools: Vec<InjectorPool>,
}

struct InjectorPool {
    name: String,
    entries: Vec<Value>,
}

impl Injector {
    fn decode(value: &Value) -> Result<Self, String> {
        let object = value.as_object().ok_or("loot injector must be an object")?;
        let target = object.get("name").and_then(Value::as_str).ok_or("loot injector needs a name")?.to_owned();
        // Top-level conditions gate the whole injection; decoded here, evaluated against the live
        // config at each table load (the same `tconstruct:config` conditions the recipes use).
        let conditions = match object.get("conditions") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(conditions)) => {
                let context = ConditionContext { items: None, config: BTreeMap::new() };
                conditions.iter().map(|condition| context.decode(condition)).collect::<Result<_, _>>()?
            }
            Some(_) => return Err("loot injector conditions must be a list".into()),
        };
        let pools = object.get("pools").and_then(Value::as_array).ok_or("loot injector needs pools")?;
        let mut decoded_pools = Vec::new();
        for pool in pools {
            let pool = pool.as_object().ok_or("loot injector pool must be an object")?;
            let name = pool.get("name").and_then(Value::as_str).ok_or("loot injector pool needs a name")?.to_owned();
            if pool_index(&name).is_none() {
                return Err(format!("loot injector pool name {name} is not main or poolN"));
            }
            let entries = pool.get("entries").and_then(Value::as_array).ok_or("loot injector pool needs entries")?.clone();
            for entry in &entries {
                validate_entry(entry)?;
            }
            decoded_pools.push(InjectorPool { name, entries });
        }
        Ok(Self { target, conditions, pools: decoded_pools })
    }

    /// Whether the injector's conditions hold against the live config.
    fn conditions_hold(&self, context: &ConditionContext) -> bool {
        self.conditions.iter().all(|condition| context.test(condition).unwrap_or(false))
    }
}

/// A pool name to its index in the table: the vanilla rule, the first pool is `main` and the rest
/// are `pool1`, `pool2`, ..., because the host's tables carry no pool names.
fn pool_index(name: &str) -> Option<usize> {
    if name == "main" {
        Some(0)
    } else {
        name.strip_prefix("pool")?.parse().ok()
    }
}

/// The shapes the merge and the function callbacks understand; anything else is a problem at load,
/// not a silent no-op at roll time.
fn validate_entry(entry: &Value) -> Result<(), String> {
    let object = entry.as_object().ok_or("loot injector entry must be an object")?;
    let kind = object.get("type").and_then(Value::as_str).ok_or("loot injector entry needs a type")?;
    match kind {
        "minecraft:item" => {
            if object.get("name").and_then(Value::as_str).is_none() {
                return Err("loot injector item entry needs a name".into());
            }
        }
        "tconstruct:tool_part" => {
            if object.get("tag").and_then(Value::as_str).is_none() {
                return Err("tool part entry needs a tag".into());
            }
            let material = object.get("material").ok_or("tool part entry needs a material")?;
            tool_definitions::valid_random_material(material)?;
        }
        _ => return Err(format!("loot injector entry type {kind} is not supported")),
    }
    for function in object.get("functions").and_then(Value::as_array).into_iter().flatten() {
        validate_function(function)?;
    }
    Ok(())
}

fn validate_function(function: &Value) -> Result<(), String> {
    let object = function.as_object().ok_or("loot function must be an object")?;
    let kind = object.get("function").and_then(Value::as_str).ok_or("loot function needs a function")?;
    match kind {
        "tconstruct:add_tool_data" => {
            if let Some(damage) = object.get("damage_percent") {
                let damage = damage.as_f64().ok_or("damage_percent must be a number")?;
                if !(0.0..=1.0).contains(&damage) {
                    return Err("damage_percent must be between 0 and 1".into());
                }
            }
            for material in object.get("materials").and_then(Value::as_array).into_iter().flatten() {
                tool_definitions::valid_random_material(material)?;
            }
        }
        "mantle:set_fluid" => {
            let fluid = object.get("fluid").and_then(Value::as_object).ok_or("set_fluid needs a fluid")?;
            if fluid.get("fluid").and_then(Value::as_str).is_none() {
                return Err("set_fluid needs a fluid id".into());
            }
            if fluid.get("amount").and_then(Value::as_u64).is_none() {
                return Err("set_fluid needs an amount".into());
            }
        }
        // The game's own functions run natively in the host; a mod's own kind must be answered by
        // its own FUNCTION callback, so an unknown kind here is a problem.
        "tconstruct:tool_part_material" => return Err("tool_part_material is synthesized at merge time, never written".into()),
        _ if kind.starts_with("minecraft:") => {}
        _ => return Err(format!("loot injector function {kind} is not supported")),
    }
    Ok(())
}

/// The loaded injectors, in file order.
#[derive(Default)]
pub(crate) struct Injectors {
    injectors: Vec<Injector>,
    pub problems: Vec<String>,
    pub loaded: bool,
}

impl Injectors {
    pub fn load(files: &[(Identifier, Value)]) -> Self {
        let mut result = Self { loaded: true, ..Self::default() };
        let mut sorted: Vec<&(Identifier, Value)> = files.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (id, value) in sorted {
            match Injector::decode(value) {
                Ok(injector) => result.injectors.push(injector),
                Err(error) => result.problems.push(format!("{id}: {error}")),
            }
        }
        result
    }
}

/// Reload listener, shaped like the other data listeners: the injector files, decoded once per
/// reload, with problems reported and the broken files skipped (not a failed load).
pub(crate) fn register() -> Result<Arc<RwLock<Injectors>>, String> {
    let state = Arc::new(RwLock::new(Injectors::default()));
    let output = state.clone();
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "loot_injectors").unwrap(),
        FOLDER,
        move |files| {
            let mut state = output.write().map_err(|_| "loot injectors poisoned")?;
            *state = Injectors::load(files);
            for problem in &state.problems {
                eprintln!("Tinkers: {problem}");
            }
            Ok(())
        },
    )?;
    let stopped = state.clone();
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| {
        *stopped.write().expect("loot injectors poisoned") = Injectors::default()
    });
    Ok(state)
}

/// The live config values, for the injectors' `tconstruct:config` conditions; None before the
/// config loads, and then the tables reload and the merge runs again with them.
fn live_config() -> Option<BTreeMap<String, bool>> {
    let config = common::config_values()?;
    config.read().ok()?.clone().ok()
}

/// The table-load merge and the loot-function answers.
pub(crate) fn register_hooks() {
    LootTableEvents::MODIFY.register(|context| {
        let Some(injectors) = common::loot_injectors() else { return };
        let Ok(injectors) = injectors.read() else { return };
        if !injectors.loaded {
            return;
        }
        let Some(config) = live_config() else { return };
        let condition_context = ConditionContext { items: None, config };
        merge_table(&context.id, &mut context.table.borrow_mut(), &injectors.injectors, &condition_context);
    });
    LootTableEvents::FUNCTION.register(|context| {
        match context.kind.as_str() {
            "tconstruct:add_tool_data" => apply_add_tool_data(context),
            "mantle:set_fluid" => apply_set_fluid(context),
            TOOL_PART_MATERIAL => apply_tool_part_material(context),
            _ => {}
        }
    });
}

/// Merges every injector that targets this table into the table's JSON, in file order. A pool the
/// target does not have is appended whole, rolling once. Returns whether anything merged.
fn merge_table(table_id: &str, table: &mut Value, injectors: &[Injector], context: &ConditionContext) -> bool {
    let Some(pools) = table.get_mut("pools").and_then(Value::as_array_mut) else { return false };
    let mut changed = false;
    for injector in injectors {
        if injector.target != table_id || !injector.conditions_hold(context) {
            continue;
        }
        for pool in &injector.pools {
            let Some(index) = pool_index(&pool.name) else { continue };
            let entries: Vec<Value> = pool.entries.iter().map(translate_entry).collect();
            if let Some(target) = pools.get_mut(index) {
                let Some(target_entries) = target.get_mut("entries").and_then(Value::as_array_mut) else { continue };
                target_entries.extend(entries);
            } else {
                pools.push(json!({"name": pool.name, "rolls": 1, "entries": entries}));
            }
            changed = true;
        }
    }
    changed
}

/// One injector entry from the vanilla loot dialect into the host's: the entry's `functions`
/// become its `modifier`, and a `tconstruct:tool_part` entry becomes a `minecraft:tag` entry over
/// the part tag (expanded, so the roll picks one part) carrying a `tconstruct:tool_part_material`
/// function with the entry's material; the host drops no entry type but the game's own.
fn translate_entry(entry: &Value) -> Value {
    let mut entry = entry.clone();
    let object = entry.as_object_mut().expect("validated injector entry is an object");
    let functions = object.remove("functions").and_then(|functions| functions.as_array().cloned()).unwrap_or_default();
    let mut translated: Vec<Value> = functions.iter().map(translate_function).collect();
    if object.get("type").and_then(Value::as_str) == Some("tconstruct:tool_part") {
        // The tag entry's `items` is the tag itself; `expand` makes one choice per member, so the
        // roll picks one part uniformly, as the original's tag-member draw does. The entry's own
        // functions, if any, run before the material function, in order.
        let tag = object.get("tag").and_then(Value::as_str).unwrap_or("").to_owned();
        let material = object.remove("material").unwrap_or(Value::Null);
        let weight = object.get("weight").cloned().unwrap_or(json!(1));
        translated.push(json!({"type": TOOL_PART_MATERIAL, "material": material}));
        let mut translated_entry = serde_json::Map::new();
        translated_entry.insert("type".to_owned(), json!("minecraft:tag"));
        translated_entry.insert("items".to_owned(), json!(format!("#{tag}")));
        translated_entry.insert("expand".to_owned(), json!(true));
        translated_entry.insert("weight".to_owned(), weight);
        if let Some(quality) = object.get("quality") {
            translated_entry.insert("quality".to_owned(), quality.clone());
        }
        if let Some(conditions) = object.get("conditions") {
            translated_entry.insert("condition".to_owned(), conditions.clone());
        }
        set_modifier(&mut translated_entry, translated);
        return Value::Object(translated_entry);
    }
    set_modifier(object, translated);
    entry
}

/// One function as the entry's `modifier`, several under a `minecraft:sequence`; none leaves the
/// entry without a modifier, as the host reads `modifier` and ignores `functions`.
fn set_modifier(object: &mut serde_json::Map<String, Value>, translated: Vec<Value>) {
    if translated.len() == 1 {
        object.insert("modifier".to_owned(), translated.into_iter().next().unwrap());
    } else if !translated.is_empty() {
        object.insert("modifier".to_owned(), json!({"type": "minecraft:sequence", "functions": translated}));
    }
}

/// One function object from the vanilla dialect (`function`) to the host's (`type`), recursing
/// into a sequence's own functions.
fn translate_function(function: &Value) -> Value {
    let mut function = function.clone();
    let object = function.as_object_mut().expect("validated loot function is an object");
    if let Some(kind) = object.remove("function").and_then(|kind| kind.as_str().map(str::to_owned)) {
        object.insert("type".to_owned(), Value::String(kind));
    }
    if let Some(functions) = object.get("functions").and_then(Value::as_array).cloned() {
        let translated: Vec<Value> = functions.iter().map(translate_function).collect();
        object.insert("functions".to_owned(), Value::Array(translated));
    }
    function
}

/// A [0, 1) draw from the roll's seed: splitmix64, the same draw for the same roll, without
/// touching the shared random state the table's own rolls use.
struct SeededRoll(u64);
impl SeededRoll {
    fn new(seed: i64) -> Self {
        Self(seed as u64)
    }
    fn next_f32(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D049BB133111EB);
        value ^= value >> 31;
        ((value >> 40) as f32) / 16_777_216.0
    }
}

/// Whether the stack's item is a modifiable tool (AddToolDataFunction's `stack.is(MODIFIABLE)`).
fn is_modifiable(item: &Identifier, items: &pluto_lib::RegistryLookup) -> bool {
    let Some(tag) = Identifier::new("tconstruct", "modifiable") else { return false };
    items.get_tag(&tag).is_some_and(|members| members.contains(item))
}

/// AddToolDataFunction: a modifiable tool gets random materials for its definition's stat types
/// (first-with-stat beyond the list) and is rebuilt; a tool without material slots, or with an
/// empty list, is only rebuilt. Damage follows, as a share of the rebuilt durability. Anything
/// unready (no item snapshot, no materials, no definitions, no modifiers) leaves the stack alone.
fn apply_add_tool_data(context: &LootFunctionContext) {
    let mut stack = context.stack.borrow_mut();
    let Some(items) = common::item_snapshot() else { return };
    if !is_modifiable(&stack.item, &items) {
        return;
    }
    let definitions = tool_definitions::register();
    let Ok(definitions) = definitions.read() else { return };
    let definition_id = tool_definitions::definition_id_for_item(&stack.item);
    let Some(definition) = definitions.data.get(&definition_id) else { return };
    let entries = context.data.get("materials").and_then(Value::as_array).cloned().unwrap_or_default();
    let types = definition.material_stat_types();
    let damage_percent = context.data.get("damage_percent").and_then(Value::as_f64).unwrap_or(0.0);
    common::with_client_materials(|materials| {
        let Some(materials) = materials else { return };
        let Some(modifiers) = common::server_modifiers() else { return };
        let Ok(modifiers) = modifiers.read() else { return };
        if let Some(types) = types {
            if !entries.is_empty() {
                let mut roll = SeededRoll::new(context.seed);
                let Ok(list) = tool_definitions::build_random_materials(types, &entries, materials, &mut || roll.next_f32()) else { return };
                tool_stack::set_materials_raw(&mut stack.custom_data, &list);
            }
        }
        let env = tool_stack::Environment {
            definition: Some(definition),
            has_materials: true,
            materials,
            modifiers: &modifiers,
            item_tags_loaded: true,
            item: Some(&stack.item),
            item_tags: Some(&items),
        };
        if tool_stack::rebuild_stats(&mut stack.custom_data, &env).is_err() {
            return;
        }
        if (0.0..=1.0).contains(&damage_percent) && damage_percent > 0.0 {
            // setDamage((int)(durability * damage)): the stack is fresh, so damaging by that
            // amount sets it; the cap at remaining durability is a no-op on a fresh stack.
            let amount = (tool_stack::durability(&stack.custom_data) as f64 * damage_percent) as i32;
            if amount > 0 {
                tool_damage::direct_damage(&mut stack.custom_data, amount, true);
            }
        }
    });
}

/// SetFluidLootFunction (Mantle): the fluid goes into the stack's tank — a registered tank item
/// (a lantern) through FluidItems, a tool with a tank through its own tank, capped by the tank's
/// capacity. A stack with no tank keeps nothing. A tool's tank only holds fluid after its stats
/// rebuild, so a swasher's set_fluid must follow its add_tool_data, as the shipped entries do.
fn apply_set_fluid(context: &LootFunctionContext) {
    let Some(fluid) = context.data.get("fluid").and_then(Value::as_object) else { return };
    let Some(fluid_id) = fluid.get("fluid").and_then(Value::as_str).and_then(|text| Identifier::parse(text).ok()) else { return };
    let amount = fluid.get("amount").and_then(Value::as_u64).unwrap_or(0) as u32;
    if amount == 0 {
        return;
    }
    let resource = FluidStack { fluid: fluid_id, amount, tag: None };
    let mut stack = context.stack.borrow_mut();
    let filled = FluidItems::fill(&stack, &resource);
    if filled.amount > 0 {
        *stack = filled.container;
        return;
    }
    tool_tank::fill(&mut stack.custom_data, &resource, true);
}

/// ToolPartLootEntry's material half: the entry itself became a `minecraft:tag` entry at merge
/// time, so the roll already picked one part; this sets the part's material, and when the part
/// cannot take the material the stack drops to nothing (the original's createItemStack accepts
/// nothing in that case, and the host drops a count-0 stack).
fn apply_tool_part_material(context: &LootFunctionContext) {
    let Some(material) = context.data.get("material") else { return };
    let mut stack = context.stack.borrow_mut();
    let parts = crate::tool_parts::registry();
    let Some(part) = parts.get(&stack.item) else { return };
    let set = common::with_client_materials(|materials| {
        let Some(materials) = materials else { return false };
        let mut roll = SeededRoll::new(context.seed);
        let Ok(material) = tool_definitions::pick_random_material(material, &part.stat, materials, &mut || roll.next_f32()) else { return false };
        part.set_material(&mut stack, &material, materials)
    });
    if !set {
        stack.count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn context() -> ConditionContext {
        ConditionContext { items: None, config: crate::config::parse("").unwrap() }
    }

    fn injector(json: Value) -> Injector {
        Injector::decode(&json).unwrap()
    }

    fn table(pools: Value) -> Value {
        json!({"type": "minecraft:chest", "pools": pools, "random_sequence": "minecraft:test"})
    }

    #[test]
    fn merge_puts_entries_in_the_named_pools_and_translates_functions() {
        let injectors = vec![injector(json!({
            "name": "minecraft:chests/simple_dungeon",
            "pools": [
                {"name": "main", "entries": [
                    {"type": "minecraft:item", "name": "tconstruct:melting_pan", "weight": 10, "functions": [
                        {"function": "tconstruct:add_tool_data", "materials": [
                            {"type": "tconstruct:ancient"}, {"type": "tconstruct:ancient"}]}]}
                ]},
                {"name": "pool1", "entries": [
                    {"type": "minecraft:item", "name": "tconstruct:hand_axe", "weight": 2, "functions": [
                        {"function": "tconstruct:add_tool_data", "materials": [{"type": "tconstruct:first"}]},
                        {"function": "minecraft:set_damage", "damage": {"type": "minecraft:uniform", "min": 0.0, "max": 0.5}}]}
                ]}
            ]
        }))];
        let mut dungeon = table(json!([
            {"rolls": 1, "entries": [{"type": "minecraft:item", "name": "minecraft:leather", "weight": 20}]},
            {"rolls": 1, "entries": [{"type": "minecraft:item", "name": "minecraft:wheat", "weight": 20}]},
            {"rolls": 3, "entries": [{"type": "minecraft:item", "name": "minecraft:bone", "weight": 10}]}
        ]));
        assert!(merge_table("minecraft:chests/simple_dungeon", &mut dungeon, &injectors, &context()));
        let pools = dungeon["pools"].as_array().unwrap();
        assert_eq!(pools[0]["entries"].as_array().unwrap().len(), 2, "the pan joins main beside the leather");
        let pan = &pools[0]["entries"][1];
        assert_eq!(pan["modifier"]["type"], "tconstruct:add_tool_data", "one function stays a single modifier");
        assert_eq!(pan["modifier"]["materials"].as_array().unwrap().len(), 2);
        assert!(pan.get("functions").is_none(), "the vanilla key is gone");
        let axe = &pools[1]["entries"][1];
        assert_eq!(axe["modifier"]["type"], "minecraft:sequence", "two functions become a sequence");
        assert_eq!(axe["modifier"]["functions"][0]["type"], "tconstruct:add_tool_data");
        assert_eq!(axe["modifier"]["functions"][1]["type"], "minecraft:set_damage");
        assert_eq!(pools[2]["entries"].as_array().unwrap().len(), 1, "pool2 is untouched");
        assert!(!merge_table("minecraft:chests/other", &mut dungeon.clone(), &injectors, &context()), "another table is untouched");
    }

    #[test]
    fn tool_part_entries_become_expanded_tag_entries() {
        let injectors = vec![injector(json!({
            "name": "minecraft:gameplay/piglin_bartering",
            "pools": [
                {"name": "main", "entries": [
                    {"type": "tconstruct:tool_part", "tag": "tconstruct:parts/bartered", "weight": 8,
                     "material": {"type": "tconstruct:random", "allow_hidden": true,
                                  "material": {"type": "mantle:tag", "tag": "tconstruct:bartered"}}}
                ]}
            ]
        }))];
        let mut bartering = table(json!([{"rolls": 1, "entries": []}]));
        assert!(merge_table("minecraft:gameplay/piglin_bartering", &mut bartering, &injectors, &context()));
        let entry = &bartering["pools"][0]["entries"][0];
        assert_eq!(entry["type"], "minecraft:tag");
        assert_eq!(entry["items"], "#tconstruct:parts/bartered");
        assert_eq!(entry["expand"], true, "one choice per part, so the roll picks one uniformly");
        assert_eq!(entry["weight"], 8);
        assert_eq!(entry["modifier"]["type"], "tconstruct:tool_part_material");
        assert_eq!(entry["modifier"]["material"]["type"], "tconstruct:random");
    }

    #[test]
    fn conditions_gate_the_whole_injection_and_missing_pools_append() {
        let slimy = injector(json!({
            "name": "minecraft:chests/simple_dungeon",
            "conditions": [{"type": "tconstruct:config", "prop": "slimy_loot_chests"}],
            "pools": [{"name": "main", "entries": [
                {"type": "minecraft:item", "name": "tconstruct:earth_slime_sapling", "weight": 3}
            ]}]
        }));
        let labyrinth = injector(json!({
            "name": "twilightforest:chests/labyrinth_room",
            "conditions": [{"type": "forge:mod_loaded", "modid": "twilightforest"}],
            "pools": [{"name": "pool1", "entries": [
                {"type": "minecraft:item", "name": "tconstruct:minotaur_axe", "weight": 1}
            ]}]
        }));
        let mut dungeon = table(json!([{"rolls": 1, "entries": []}]));
        let mut off = context();
        off.config.insert("slimy_loot_chests".to_owned(), false);
        assert!(!merge_table("minecraft:chests/simple_dungeon", &mut dungeon, &[slimy], &off), "the gate off skips the injection");
        assert!(merge_table("minecraft:chests/simple_dungeon", &mut dungeon, &[slimy], &context()), "the gate on (the default) merges it");
        assert_eq!(dungeon["pools"][0]["entries"].as_array().unwrap().len(), 1);
        // The labyrinth target never loads here: its mod is not loaded, so nothing merges even
        // for its own table id.
        let mut labyrinth_table = table(json!([{"rolls": 1, "entries": []}, {"rolls": 1, "entries": []}]));
        assert!(!merge_table("twilightforest:chests/labyrinth_room", &mut labyrinth_table, &[labyrinth], &context()));
        // A pool the target does not have is appended whole, rolling once.
        let buried = injector(json!({
            "name": "minecraft:chests/buried_treasure",
            "pools": [{"name": "pool3", "entries": [
                {"type": "minecraft:item", "name": "tconstruct:swasher", "weight": 2}
            ]}]
        }));
        let mut treasure = table(json!([{"rolls": 1, "entries": []}]));
        assert!(merge_table("minecraft:chests/buried_treasure", &mut treasure, &[buried], &context()));
        assert_eq!(treasure["pools"].as_array().unwrap().len(), 2);
        assert_eq!(treasure["pools"][1]["name"], "pool3");
        assert_eq!(treasure["pools"][1]["rolls"], 1);
    }

    #[test]
    fn broken_injectors_are_problems_not_a_failed_load() {
        let files = vec![
            (Identifier::new("tconstruct", "good").unwrap(), json!({
                "name": "minecraft:chests/simple_dungeon",
                "pools": [{"name": "main", "entries": [{"type": "minecraft:item", "name": "tconstruct:melting_pan", "weight": 10}]}]
            })),
            (Identifier::new("tconstruct", "bad-pool").unwrap(), json!({
                "name": "minecraft:chests/simple_dungeon",
                "pools": [{"name": "side", "entries": []}]
            })),
            (Identifier::new("tconstruct", "bad-function").unwrap(), json!({
                "name": "minecraft:chests/simple_dungeon",
                "pools": [{"name": "main", "entries": [{"type": "minecraft:item", "name": "tconstruct:melting_pan",
                    "functions": [{"function": "tconstruct:add_tool_data", "damage_percent": 2.0}]}]}]
            })),
        ];
        let loaded = Injectors::load(&files);
        assert!(loaded.loaded);
        assert_eq!(loaded.injectors.len(), 1, "only the good file loads");
        assert_eq!(loaded.problems.len(), 2);
        assert!(loaded.problems.iter().any(|problem| problem.contains("bad-pool")), "{}", loaded.problems.join("; "));
        assert!(loaded.problems.iter().any(|problem| problem.contains("bad-function")), "{}", loaded.problems.join("; "));
    }

    #[test]
    fn the_seed_draws_are_deterministic_and_inside_the_unit_interval() {
        let draws = |seed: i64| {
            let mut roll = SeededRoll::new(seed);
            (roll.next_f32(), roll.next_f32(), roll.next_f32())
        };
        let (a, b, c) = draws(12345);
        assert!(a >= 0.0 && a < 1.0 && b >= 0.0 && b < 1.0 && c >= 0.0 && c < 1.0);
        assert_ne!((a, b), (b, c), "consecutive draws differ");
        assert_eq!(draws(12345), (a, b, c), "the same seed draws the same values");
        assert_ne!(draws(54321).0, a, "another seed draws another first value");
    }

    #[test]
    fn a_function_for_another_kind_leaves_the_stack_alone() {
        let stack = ItemSlot::new(Identifier::parse("tconstruct:melting_pan").unwrap(), 1);
        let context = LootFunctionContext {
            kind: "example:other".into(), data: json!({}), origin: None, dimension: None, seed: 0,
            block_entity: None, stack: std::cell::RefCell::new(stack),
        };
        LootTableEvents::FUNCTION.invoke(&context);
        assert_eq!(context.stack.borrow().count, 1);
    }
}
