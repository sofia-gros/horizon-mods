//! Tools, on the original's library (`tconstruct-library`): the registries the tool code works from, loaded from the original's data.
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, OnceLock};

use serde_json::Value;
use tconstruct_library::condition::ConditionContext;
use tconstruct_library::materials::MaterialId;
use tconstruct_library::modifiers::stat_modules::ToolContext;
use tconstruct_library::tools::stack::rebuild_stats;
use tconstruct_library::tools::stat::StatValue;
use tconstruct_library::materials::manager::MaterialManager;
use tconstruct_library::materials::MaterialRegistry;
use tconstruct_library::modifiers::{ModifierManager, ModuleRegistry};
use tconstruct_library::resource::ResourceId;
use tconstruct_library::tools::definition::{ToolDefinitionData, ToolModuleRegistry};
use tconstruct_library::tools::stat::{HarvestTiers, ToolStats};

use crate::tinkering_data;

/// What the data's conditions ask the game. The port has no mod integration: nothing else is installed, no tag of another mod has
/// members and the integration switches are off.
struct GameConditions;

impl ConditionContext for GameConditions {
    fn mod_loaded(&self, _: &str) -> bool {
        false
    }
    fn tag_filled(&self, _: &str) -> bool {
        false
    }
    fn config(&self, _: &str) -> bool {
        false
    }
}

/// Everything the tool code reads: stats, materials (with their stats and traits), modifiers and tool definitions.
pub struct ToolLibrary {
    pub stats: Arc<ToolStats>,
    pub materials: MaterialRegistry,
    pub modifiers: ModifierManager,
    pub definitions: HashMap<ResourceId, ToolDefinitionData>,
    /// The members of the `modifiable/*` item tags, with nested tags resolved (an item that does not exist is not checked).
    pub item_tags: BTreeMap<ResourceId, Vec<ResourceId>>,
    /// What loading found wrong, as the original would log it.
    pub problems: Vec<String>,
}

fn files(kind: &str) -> Vec<(ResourceId, Value)> {
    tinkering_data::FILES
        .iter()
        .filter(|(k, _, _)| *k == kind)
        .filter_map(|(_, id, json)| Some((ResourceId::try_parse(id)?, serde_json::from_str(json).ok()?)))
        .collect()
}

impl ToolLibrary {
    /// Loads the library from the data that is built into the mod.
    pub fn load() -> Self {
        let stats = Arc::new(ToolStats::tinkers(HarvestTiers::vanilla()));
        let mut problems = Vec::new();
        let mut materials = MaterialRegistry::with_tinkers_stats();
        let (manager, material_problems) = MaterialManager::load(&files("materials/definition"), &files("tags/materials"), &GameConditions);
        materials.materials = manager;
        problems.extend(material_problems);
        problems.extend(materials.stats.load(&files("materials/stats")));
        problems.extend(materials.traits.load(&files("materials/traits")));

        let mut modules = ModuleRegistry::with_original_types();
        modules.register_stat_modules(stats.clone());
        let enchantments: Vec<Value> = serde_json::from_str(tinkering_data::ENCHANTMENTS_TO_MODIFIERS).into_iter().collect();
        let (modifiers, modifier_problems) = ModifierManager::load(&files("modifiers"), &files("tags/modifiers"), &enchantments, &modules, &GameConditions);
        problems.extend(modifier_problems);

        let tool_modules = ToolModuleRegistry::with_original_types();
        let mut definitions = HashMap::new();
        for (id, json) in files("tool_definitions") {
            match ToolDefinitionData::from_json(&json, &tool_modules, &stats) {
                Ok(data) => {
                    definitions.insert(id, data);
                }
                Err(error) => problems.push(format!("Failed to load tool definition {id}: {error}")),
            }
        }
        let (item_tags, tag_problems) = tconstruct_library::tags::load(&files("tags/item"), &|_| true);
        problems.extend(tag_problems);
        Self { stats, materials, modifiers, definitions, item_tags, problems }
    }
}

/// The item tags of one tool item, for the conditions of the modules.
struct ToolItem<'a> {
    library: &'a ToolLibrary,
    item: ResourceId,
}

impl ToolContext for ToolItem<'_> {
    fn item_in_tag(&self, tag: &str) -> bool {
        ResourceId::try_parse(tag).and_then(|tag| self.library.item_tags.get(&tag)).is_some_and(|members| members.contains(&self.item))
    }
}

impl ToolLibrary {
    /// The stack of a tool made of `material` in every part: the tool's item, its durability as the stack's max damage and its materials,
    /// modifiers and stats as the stack's data (`tconstruct:materials`, `tconstruct:modifiers`, `tconstruct:stats` in the custom data).
    pub fn build(&self, tool: &str, material: &str) -> Result<pluto_lib::StackData, String> {
        use pluto_lib::Nbt;
        let id = ResourceId::try_parse_in("tconstruct", tool).ok_or("bad tool id")?;
        let definition = self.definitions.get(&id).ok_or_else(|| format!("no tool {id}"))?;
        let material = MaterialId::try_parse(&format!("tconstruct:{}", material.trim_start_matches("tconstruct:"))).ok_or("bad material id")?;
        if self.materials.materials.material(&material).is_none() {
            return Err(format!("no material {material}"));
        }
        let materials = vec![material; definition.parts().len()];
        let rebuilt = rebuild_stats(definition, &materials, &[], &self.materials, &self.modifiers, &self.stats, &ToolItem { library: self, item: id.clone() });
        let mut stats = pluto_lib::NbtCompound::new();
        for (stat, value) in &rebuilt.stats {
            stats.insert(stat.0.to_string(), match value {
                StatValue::Float(v) => Nbt::Float(*v),
                StatValue::Tier(t) => Nbt::String(t.to_string()),
            });
        }
        let durability = match rebuilt.stats.get(&tconstruct_library::tools::stat::ToolStatId::tconstruct("durability")) {
            Some(StatValue::Float(v)) => Some(*v as i32),
            _ => None,
        };
        let mut data = pluto_lib::NbtCompound::new();
        data.insert("tconstruct:materials".to_owned(), Nbt::List(materials.iter().map(|m| Nbt::String(m.to_string())).collect()));
        data.insert("tconstruct:modifiers".to_owned(), Nbt::List(rebuilt.modifiers.iter().map(|m| Nbt::String(format!("{}#{}", m.modifier, m.level))).collect()));
        data.insert("tconstruct:stats".to_owned(), Nbt::Compound(stats));
        Ok(pluto_lib::StackData { item: id.to_string(), count: 1, damage: 0, max_damage: durability, custom_data: data })
    }

    /// A tool of `definition` (a tool item such as `tconstruct:pickaxe`) with every part made of `material`: the stats and the modifiers it
    /// would have, as one line of text. Err with the reason when the tool or the material is unknown.
    pub fn preview(&self, tool: &str, material: &str) -> Result<String, String> {
        let id = ResourceId::try_parse_in("tconstruct", tool).ok_or("bad tool id")?;
        let definition = self.definitions.get(&id).ok_or_else(|| format!("no tool {id}"))?;
        let material = MaterialId::try_parse(&format!("tconstruct:{}", material.trim_start_matches("tconstruct:"))).ok_or("bad material id")?;
        if self.materials.materials.material(&material).is_none() {
            return Err(format!("no material {material}"));
        }
        let materials = vec![material; definition.parts().len()];
        let rebuilt = rebuild_stats(definition, &materials, &[], &self.materials, &self.modifiers, &self.stats, &ToolItem { library: self, item: id.clone() });
        let stats: Vec<String> = rebuilt
            .stats
            .iter()
            .map(|(stat, value)| match value {
                StatValue::Float(v) => format!("{}={v}", stat.0.path()),
                StatValue::Tier(t) => format!("{}={t}", stat.0.path()),
            })
            .collect();
        let modifiers: Vec<String> = rebuilt.modifiers.iter().map(|m| format!("{}x{}", m.modifier.resource().path(), m.level)).collect();
        Ok(format!("{id}: {}; modifiers {}; slots {:?}", stats.join(", "), modifiers.join(", "), rebuilt.slots))
    }
}

/// The library, loaded once.
pub fn library() -> &'static ToolLibrary {
    static LIBRARY: OnceLock<ToolLibrary> = OnceLock::new();
    LIBRARY.get_or_init(ToolLibrary::load)
}

/// Called once at start: loads the library and says how much it holds, so a broken link or a broken data file shows in the log.
pub fn register() {
    register_break_speed();
    let library = library();
    println!(
        "tconstruct: library linked, {} tool stats, {} material stat types",
        library.stats.all().count(),
        library.materials.stats.stat_type_ids().count()
    );
    println!(
        "tconstruct: library loaded, {} materials, {} modifiers, {} tool definitions, {} problems",
        library.materials.materials.all_materials().count(),
        library.modifiers.ids().len(),
        library.definitions.len(),
        library.problems.len()
    );
    for problem in library.problems.iter().take(10) {
        println!("tconstruct: library problem: {problem}");
    }
}

/// `/tconstruct preview <tool> <material>`: what a tool made of one material would be.
pub fn preview_command(ctx: &pluto_lib::CommandContext) -> i32 {
    let (Some(tool), Some(material)) = (ctx.args.first(), ctx.args.get(1)) else { return 0 };
    match library().preview(tool, material) {
        Ok(text) => {
            println!("tconstruct: preview {tool} {material}: {text}");
            ctx.source.send_message(&text);
            1
        }
        Err(reason) => {
            ctx.source.send_message(&reason);
            0
        }
    }
}

/// `/tconstruct build <tool> <material>`: gives a tool made of one material, with its stats on the stack.
pub fn build_command(ctx: &pluto_lib::CommandContext) -> i32 {
    let (Some(tool), Some(material)) = (ctx.args.first(), ctx.args.get(1)) else { return 0 };
    match library().build(tool, material) {
        Ok(stack) => {
            let (item, max) = (stack.item.clone(), stack.max_damage);
            if ctx.source.give_stack(stack).is_some() {
                ctx.source.send_message("no room for the tool");
                return 0;
            }
            println!("tconstruct: built {item} of {material} with max damage {max:?}");
            1
        }
        Err(reason) => {
            ctx.source.send_message(&reason);
            0
        }
    }
}

/// A tool digs with the mining speed of its own stats (`tconstruct:stats` in the stack's data) where the game says the dig is faster than
/// by hand, which it takes from the item the tool stands on. The original works out whether a tool is effective from its definition's
/// `is_effective` module and the harvest tier; that is not carried yet.
pub fn register_break_speed() {
    use pluto_lib::{BreakSpeedCallback, Nbt};
    use std::sync::atomic::{AtomicBool, Ordering};
    static SAID: AtomicBool = AtomicBool::new(false);
    BreakSpeedCallback::EVENT.register(|dig| {
        let Some(held) = &dig.held else { return };
        let Some(Nbt::Compound(stats)) = held.custom_data.get("tconstruct:stats") else { return };
        let Some(Nbt::Float(mining)) = stats.get("tconstruct:mining_speed") else { return };
        if dig.original > 1.0 {
            dig.speed.set(*mining);
            if !SAID.swap(true, Ordering::Relaxed) {
                println!("tconstruct: break speed original={} now={mining} on {}", dig.original, dig.block.block);
            }
        }
    });
}
