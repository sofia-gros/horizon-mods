//! Compile production domain modules against the actual host-independent API.
#[path = "../../../tconstruct/mixins/tconstruct/api/src/tables/mod.rs"]
mod tables;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/materials.rs"]
mod materials;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/material_traits.rs"]
mod material_traits;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/modifier_entries.rs"]
mod modifier_entries;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/modifiers.rs"]
mod modifiers;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/material_stats.rs"]
mod material_stats;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/material_snapshot.rs"]
mod material_snapshot;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/material_sync.rs"]
mod material_sync;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/tool_materials.rs"]
mod tool_materials;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/tool_stats.rs"]
mod tool_stats;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/tool_definitions.rs"]
mod tool_definitions;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/tool_rebuild.rs"]
mod tool_rebuild;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/tool_stack.rs"]
mod tool_stack;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/tool_damage.rs"]
mod tool_damage;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/tool_repair.rs"]
mod tool_repair;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/tool_swap.rs"]
mod tool_swap;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/tool_build.rs"]
mod tool_build;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/tool_parts.rs"]
mod tool_parts;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/tool_mining.rs"]
mod tool_mining;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/tool_slot.rs"]
mod tool_slot;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/tool_station.rs"]
mod tool_station;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/config.rs"]
mod config;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/conditions.rs"]
mod conditions;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/common.rs"]
mod common;
#[cfg(test)]
mod reload_tests;
