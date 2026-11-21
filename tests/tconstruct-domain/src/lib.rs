//! Compile production domain modules against the actual host-independent API.
#[path = "../../../tconstruct/mixins/tconstruct/api/src/tables/mod.rs"]
mod tables;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/materials.rs"]
mod materials;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/material_traits.rs"]
mod material_traits;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/modifier_entries.rs"]
mod modifier_entries;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/modifier_module_types.rs"]
mod modifier_module_types;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/formula.rs"]
mod formula;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/material_recipes.rs"]
mod material_recipes;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/modifier_recipes.rs"]
mod modifier_recipes;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/worktable_recipes.rs"]
mod worktable_recipes;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/overslime.rs"]
mod overslime;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/modifier_removal.rs"]
mod modifier_removal;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/part_builder_recipes.rs"]
mod part_builder_recipes;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/tool_predicate.rs"]
mod tool_predicate;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/modifier_modules.rs"]
mod modifier_modules;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/modifiers.rs"]
mod modifiers;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/material_stats.rs"]
mod material_stats;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/material_snapshot.rs"]
mod material_snapshot;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/material_sync.rs"]
mod material_sync;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/model_flags.rs"]
mod model_flags;
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
#[path = "../../../tconstruct/mixins/tconstruct/api/src/armor_textures.rs"]
mod armor_textures;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/armor_worn.rs"]
mod armor_worn;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/tool_mining.rs"]
mod tool_mining;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/mining_speed.rs"]
mod mining_speed;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/melee_damage.rs"]
mod melee_damage;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/entity_facts.rs"]
mod entity_facts;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/conditional_stat.rs"]
mod conditional_stat;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/knockback.rs"]
mod knockback;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/weapon_effect.rs"]
mod weapon_effect;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/thorns.rs"]
mod thorns;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/hit_actions.rs"]
mod hit_actions;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/lifesteal.rs"]
mod lifesteal;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/capacity.rs"]
mod capacity;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/effect_immunity.rs"]
mod effect_immunity;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/tool_inventory.rs"]
mod tool_inventory;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/effect_immunity_hook.rs"]
mod effect_immunity_hook;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/unequip_hook.rs"]
mod unequip_hook;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/max_armor.rs"]
mod max_armor;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/max_armor_hook.rs"]
mod max_armor_hook;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/wearer_modules.rs"]
mod wearer_modules;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/mob_disguise.rs"]
mod mob_disguise;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/mob_disguise_hook.rs"]
mod mob_disguise_hook;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/weapon_effect_hook.rs"]
mod weapon_effect_hook;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/block_break_hook.rs"]
mod block_break_hook;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/damage_vanilla.rs"]
mod damage_vanilla;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/instrument_tags.rs"]
mod instrument_tags;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/damage_tags.rs"]
mod damage_tags;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/protection.rs"]
mod protection;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/protection_hook.rs"]
mod protection_hook;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/tool_damage_hook.rs"]
mod tool_damage_hook;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/break_speed_hook.rs"]
mod break_speed_hook;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/melee_hook.rs"]
mod melee_hook;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/lifesteal_hook.rs"]
mod lifesteal_hook;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/severing.rs"]
mod severing;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/part_recycling.rs"]
mod part_recycling;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/tool_actions.rs"]
mod tool_actions;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/tool_action_hook.rs"]
mod tool_action_hook;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/tool_slot.rs"]
mod tool_slot;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/edible.rs"]
mod edible;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/edible_hook.rs"]
mod edible_hook;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/tool_station.rs"]
mod tool_station;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/station_menu.rs"]
mod station_menu;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/worktable_menu.rs"]
mod worktable_menu;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/part_builder_menu.rs"]
mod part_builder_menu;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/creative_parts.rs"]
mod creative_parts;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/creative_tools.rs"]
mod creative_tools;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/creative_fluids.rs"]
mod creative_fluids;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/convention_tags.rs"]
mod convention_tags;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/creative.rs"]
mod creative;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/config.rs"]
mod config;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/conditions.rs"]
mod conditions;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/common.rs"]
mod common;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/fluid_values.rs"]
mod fluid_values;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/fluid_defs.rs"]
mod fluid_defs;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/render_data.rs"]
mod render_data;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/fluids.rs"]
mod fluids;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/heating.rs"]
mod heating;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/fluid_ingredient.rs"]
mod fluid_ingredient;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/melting_recipes.rs"]
mod melting_recipes;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/casting_recipes.rs"]
mod casting_recipes;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/fluid_recipes.rs"]
mod fluid_recipes;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/smeltery_data.rs"]
mod smeltery_data;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/vanilla_fuels.rs"]
mod vanilla_fuels;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/melter.rs"]
mod melter;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/melter_menu.rs"]
mod melter_menu;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/melter_blocks.rs"]
mod melter_blocks;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/alloyer.rs"]
mod alloyer;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/alloyer_menu.rs"]
mod alloyer_menu;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/casting.rs"]
mod casting;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/faucet.rs"]
mod faucet;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/channel.rs"]
mod channel;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/casting_blocks.rs"]
mod casting_blocks;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/multiblock.rs"]
mod multiblock;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/smeltery_tank.rs"]
mod smeltery_tank;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/smeltery.rs"]
mod smeltery;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/smeltery_menu.rs"]
mod smeltery_menu;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/smeltery_blocks.rs"]
mod smeltery_blocks;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/tank.rs"]
mod tank;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/smeltery_io.rs"]
mod smeltery_io;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/rendering.rs"]
mod rendering;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/fluid_transfers.rs"]
mod fluid_transfers;
#[path = "../../../tconstruct/mixins/tconstruct/api/src/fluid_items.rs"]
mod fluid_items;
#[cfg(test)]
mod reload_tests;
#[cfg(test)]
mod original_data;
#[cfg(test)]
mod casting_loop;
#[cfg(test)]
mod station_dispatch;
#[cfg(test)]
mod visuals;
