//! `library.tools.nbt.ToolStack.rebuildStats` and `ModifierNBT.Builder` without the item: from a tool definition, the materials of its
//! parts and the upgrades, the list of modifiers the tool has (upgrades, tool traits and material traits, merged and sorted by
//! priority) and the stats the definition, the materials and the modifiers give it.
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use super::definition::{part_stat_type, PartStatsModule, ToolDefinitionData, ToolTraitsModule};
use super::stat::{ModifierStatsBuilder, StatValue, ToolStatId, ToolStats};
use crate::materials::{MaterialId, MaterialRegistry};
use crate::modifiers::stat_modules::{ModifierTraitModule, StatBoostModule, ToolContext};
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

/// `ModifierTraitHook.TraitBuilder`: adds modifiers and, for each, the traits its modules bring along (which may bring more).
struct TraitBuilder<'a> {
    manager: &'a ModifierManager,
    context: &'a dyn ToolContext,
    entries: Vec<ModifierEntry>,
    seen: HashSet<ModifierId>,
    stack: Vec<ModifierId>,
}

impl TraitBuilder<'_> {
    fn add(&mut self, entry: ModifierEntry) {
        if entry.modifier == ModifierId::empty() || entry.level <= 0 || !self.manager.contains(&entry.modifier) {
            return;
        }
        self.entries.push(entry.clone());
        let Some(modifier) = self.manager.get(&entry.modifier) else { return };
        if self.stack.contains(&entry.modifier) {
            // the original logs "Encountered X as a child of itself" and stops
            return;
        }
        self.stack.push(entry.modifier.clone());
        let first = !self.seen.contains(&entry.modifier);
        for module in &modifier.modules {
            if let Some(module) = module.as_any().downcast_ref::<ModifierTraitModule>() {
                if let Some(extra) = module.trait_for(self.context, &entry, first) {
                    self.add(extra);
                }
            }
        }
        self.seen.insert(entry.modifier.clone());
        self.stack.pop();
    }
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
    let before = merge_modifiers(upgrades.iter().cloned().chain(definition.tool_traits(materials, registry)), modifiers);
    // modifiers that bring traits along (`ModifierTraitModule`): every modifier of the list is added again through the trait builder
    let mut traits = TraitBuilder { manager: modifiers, context, entries: Vec::new(), seen: HashSet::new(), stack: Vec::new() };
    for entry in before {
        traits.add(entry);
    }
    let all = merge_modifiers(traits.entries, modifiers);
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

    struct Anything;
    impl ToolContext for Anything {
        fn item_in_tag(&self, _: &str) -> bool {
            true
        }
    }

    fn with_modules(files: Vec<(&str, serde_json::Value)>) -> ModifierManager {
        let files: Vec<_> = files.into_iter().map(|(n, v)| (ResourceId::try_parse(n).unwrap(), v)).collect();
        let mut modules = ModuleRegistry::with_original_types();
        modules.register_stat_modules(Arc::new(ToolStats::tinkers(crate::tools::stat::HarvestTiers::vanilla())));
        let (manager, problems) = ModifierManager::load(&files, &[], &[], &modules, &EmptyContext);
        assert!(problems.is_empty(), "{problems:?}");
        manager
    }

    fn expand(manager: &ModifierManager, start: ModifierEntry) -> Vec<String> {
        let mut builder = TraitBuilder { manager, context: &Anything, entries: Vec::new(), seen: HashSet::new(), stack: Vec::new() };
        builder.add(start);
        merge_modifiers(builder.entries, manager).iter().map(|e| format!("{}x{}", e.modifier, e.level)).collect()
    }

    #[test]
    fn trait_modules_bring_modifiers_along_by_level_and_stay_out_of_loops() {
        let manager = with_modules(vec![
            ("tconstruct:parent", json!({"modules": [{"type": "tconstruct:trait", "name": "tconstruct:scaled", "level": 2, "fixed_level": false}, {"type": "tconstruct:trait", "name": "tconstruct:once", "level": 1, "fixed_level": true}]})),
            ("tconstruct:scaled", json!({})),
            ("tconstruct:once", json!({})),
            ("tconstruct:loop_a", json!({"modules": [{"type": "tconstruct:trait", "name": "tconstruct:loop_b", "fixed_level": false}]})),
            ("tconstruct:loop_b", json!({"modules": [{"type": "tconstruct:trait", "name": "tconstruct:loop_a", "fixed_level": false}]})),
        ]);
        let entry = |id: &str, level| ModifierEntry { modifier: mid(id), level };
        // not fixed: the trait's level times the parent's (2 x 3); fixed: once, at its own level
        assert_eq!(expand(&manager, entry("tconstruct:parent", 3)), ["tconstruct:parentx3", "tconstruct:scaledx6", "tconstruct:oncex1"]);
        // at level 1 the trait is added at its own level
        assert_eq!(expand(&manager, entry("tconstruct:parent", 1)), ["tconstruct:parentx1", "tconstruct:scaledx2", "tconstruct:oncex1"]);
        let looped = expand(&manager, entry("tconstruct:loop_a", 1));
        assert!(looped.len() <= 3, "a modifier that is its own child stops the expansion: {looped:?}");
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
