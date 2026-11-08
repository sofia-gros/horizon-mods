//! `library.tools.nbt.ToolStack.rebuildStats` and `ModifierNBT.Builder` without the item: from a tool definition, the materials of its
//! parts and the upgrades, the list of modifiers the tool has (upgrades, tool traits and material traits, merged and sorted by
//! priority) and the stats the definition, the materials and the modifiers give it.
use std::collections::BTreeMap;
use std::sync::Arc;

use super::definition::{part_stat_type, PartStatsModule, ToolDefinitionData, ToolTraitsModule};
use super::stat::{ModifierStatsBuilder, StatValue, ToolStatId, ToolStats};
use crate::materials::{MaterialId, MaterialRegistry};
use crate::modifiers::stat_modules::{StatBoostModule, ToolContext};
use crate::modifiers::{ModifierEntry, ModifierId, ModifierManager};

/// `ModifierNBT.Builder`: entries of the same modifier add their levels; the result is sorted by priority, highest first, and entries
/// of equal priority stay in the order they were added. Entries of modifiers the manager does not know are dropped (`isBound`).
pub fn merge_modifiers(entries: impl IntoIterator<Item = ModifierEntry>, manager: &ModifierManager) -> Vec<ModifierEntry> {
    let mut merged: Vec<ModifierEntry> = Vec::new();
    for entry in entries {
        if entry.modifier == ModifierId::empty() || entry.level <= 0 || !manager.contains(&entry.modifier) {
            continue;
        }
        match merged.iter_mut().find(|e| e.modifier == entry.modifier) {
            Some(existing) => existing.level += entry.level,
            None => merged.push(entry),
        }
    }
    merged.sort_by_key(|entry| std::cmp::Reverse(manager.get(&entry.modifier).map_or(0, |m| m.priority)));
    merged
}

impl ToolDefinitionData {
    /// The `TOOL_TRAITS` hook: the tool's own traits and those of the materials of its parts (`MaterialStatsModule.addTraits`), in module order.
    pub fn tool_traits(&self, materials: &[MaterialId], registry: &MaterialRegistry) -> Vec<ModifierEntry> {
        let mut traits = Vec::new();
        for module in &self.modules {
            if let Some(own) = module.as_any().downcast_ref::<ToolTraitsModule>() {
                traits.extend(own.traits.iter().cloned());
            } else if let Some(parts) = module.as_any().downcast_ref::<PartStatsModule>() {
                for (index, part) in parts.parts.iter().enumerate() {
                    if let (Some(material), Some(kind)) = (materials.get(index), part_stat_type(&part.item)) {
                        traits.extend(registry.traits(material, &kind).iter().cloned());
                    }
                }
            }
        }
        traits
    }
}

/// What `rebuildStats` works out.
#[derive(Debug, Clone, PartialEq)]
pub struct RebuiltTool {
    /// The tool's modifiers: upgrades, tool traits and material traits, merged and sorted by priority.
    pub modifiers: Vec<ModifierEntry>,
    /// The stats that differ from their default.
    pub stats: BTreeMap<ToolStatId, StatValue>,
    /// The factors kept for values added later (`multiplyAll`, `MULTIPLY_CONDITIONAL`).
    pub multipliers: BTreeMap<ToolStatId, f32>,
    /// Modifier slots from the definition (`VOLATILE_DATA`).
    pub slots: BTreeMap<String, i32>,
}

/// `ToolStack.rebuildStats`: `upgrades` are the modifiers added by recipes.
pub fn rebuild_stats(
    definition: &ToolDefinitionData,
    materials: &[MaterialId],
    upgrades: &[ModifierEntry],
    registry: &MaterialRegistry,
    modifiers: &ModifierManager,
    stats: &Arc<ToolStats>,
    context: &dyn ToolContext,
) -> RebuiltTool {
    let all = merge_modifiers(upgrades.iter().cloned().chain(definition.tool_traits(materials, registry)), modifiers);
    let mut builder = ModifierStatsBuilder::new(stats);
    definition.add_material_stats(&mut builder, materials, registry);
    definition.add_tool_stats(&mut builder);
    for entry in &all {
        let Some(modifier) = modifiers.get(&entry.modifier) else { continue };
        for module in &modifier.modules {
            if let Some(boost) = module.as_any().downcast_ref::<StatBoostModule>() {
                boost.add_tool_stats(context, entry, stats, &mut builder);
            }
        }
    }
    RebuiltTool { modifiers: all, stats: builder.build(), multipliers: builder.build_multipliers(), slots: definition.slots() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::condition::EmptyContext;
    use crate::modifiers::ModuleRegistry;
    use crate::resource::ResourceId;
    use serde_json::json;

    fn mid(text: &str) -> ModifierId {
        ModifierId::try_parse(text).unwrap()
    }

    fn manager(files: Vec<(&str, serde_json::Value)>) -> ModifierManager {
        let files: Vec<_> = files.into_iter().map(|(n, v)| (ResourceId::try_parse(n).unwrap(), v)).collect();
        let (manager, problems) = ModifierManager::load(&files, &[], &[], &ModuleRegistry::with_original_types(), &EmptyContext);
        assert!(problems.is_empty(), "{problems:?}");
        manager
    }

    #[test]
    fn merging_adds_levels_drops_unknown_modifiers_and_sorts_by_priority() {
        let manager = manager(vec![("tconstruct:low", json!({"priority": 10})), ("tconstruct:high", json!({"priority": 200})), ("tconstruct:also_low", json!({"priority": 10}))]);
        let entry = |id: &str, level| ModifierEntry { modifier: mid(id), level };
        let merged = merge_modifiers([entry("tconstruct:low", 1), entry("tconstruct:high", 1), entry("tconstruct:low", 2), entry("tconstruct:unknown", 1), entry("tconstruct:also_low", 1)], &manager);
        let names: Vec<String> = merged.iter().map(|e| format!("{}x{}", e.modifier, e.level)).collect();
        assert_eq!(names, ["tconstruct:highx1", "tconstruct:lowx3", "tconstruct:also_lowx1"], "higher priority first, then the order added");
    }
}
