//! Tools, on the original's library (`tconstruct-library`): the registries the tool code works from.
use std::sync::{Arc, OnceLock};

use tconstruct_library::materials::MaterialRegistry;
use tconstruct_library::tools::stat::{HarvestTiers, ToolStats};

/// The tool stats of the mod over the vanilla harvest tiers.
pub fn tool_stats() -> &'static Arc<ToolStats> {
    static STATS: OnceLock<Arc<ToolStats>> = OnceLock::new();
    STATS.get_or_init(|| Arc::new(ToolStats::tinkers(HarvestTiers::vanilla())))
}

/// The material registry with the stat types the mod registers (the material data is loaded into it when the data reaches the game).
pub fn material_registry() -> MaterialRegistry {
    MaterialRegistry::with_tinkers_stats()
}

/// Called once at start: builds the registries and says how big they are, so a broken link of the library shows in the log.
pub fn register() {
    let stats = tool_stats();
    let materials = material_registry();
    println!("tconstruct: library linked, {} tool stats, {} material stat types", stats.all().count(), materials.stats.stat_type_ids().count());
}
