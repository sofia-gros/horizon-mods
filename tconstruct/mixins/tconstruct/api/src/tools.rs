//! Tools, on the original's library (`tconstruct-library`): the registries the tool code works from, loaded from the original's data.
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use serde_json::Value;
use tconstruct_library::condition::ConditionContext;
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
        Self { stats, materials, modifiers, definitions, problems }
    }
}

/// The library, loaded once.
pub fn library() -> &'static ToolLibrary {
    static LIBRARY: OnceLock<ToolLibrary> = OnceLock::new();
    LIBRARY.get_or_init(ToolLibrary::load)
}

/// Called once at start: loads the library and says how much it holds, so a broken link or a broken data file shows in the log.
pub fn register() {
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
