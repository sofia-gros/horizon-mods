//! Clean port: common config/material reload registration; tool gameplay is unfinished.
mod tables;
mod materials;
mod material_traits;
mod modifier_entries;
mod modifier_module_types;
mod formula;
mod material_recipes;
mod modifier_recipes;
mod modifier_removal;
mod overslime;
mod worktable_recipes;
mod worktable_menu;
mod part_builder_menu;
mod part_builder_recipes;
mod tool_predicate;
mod modifier_modules;
mod modifiers;
mod material_stats;
mod material_snapshot;
mod material_sync;
mod tool_materials;
mod tool_stats;
mod tool_definitions;
mod tool_rebuild;
mod tool_stack;
mod tool_damage;
mod tool_repair;
mod tool_swap;
mod tool_build;
mod tool_parts;
mod tool_mining;
mod mining_speed;
mod melee_damage;
mod melee_hook;
mod part_recycling;
mod tool_actions;
mod tool_action_hook;
mod tool_slot;
mod tool_station;
mod station_menu;
mod break_speed_hook;
mod creative_parts;
mod creative;
mod conditions;
mod config;
mod common;

pub fn hooks() -> pluto_lib::ModHooks {
    common::register();
    station_menu::register();
    part_builder_menu::register();
    worktable_menu::register();
    creative::register();
    break_speed_hook::register();
    melee_hook::register();
    tool_action_hook::register();
    pluto_lib::no_hooks()
}
