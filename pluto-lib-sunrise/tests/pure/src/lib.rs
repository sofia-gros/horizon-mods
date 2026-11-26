//! The host-independent half of PlutoLib, built without Horizon, plus the compiled documentation examples.
#[path = "../../../mod/src/pure/mod.rs"]
pub mod pure;
#[path = "../../../mod/src/exports.rs"]
mod exports;

pub use exports::*;

/// Block movement factors (speed and jump) for mod blocks.
pub use pluto_movement_api as movement;

/// Harness-only fixtures: construct through the real snapshot validator without
/// exposing a production API or duplicating its registry-membership rules.
pub mod fixtures {
    use super::pure::block_api_v1 as blocks;

    pub fn slime_flags(state: &super::BlockState) -> (Option<bool>, Option<bool>) {
        (blocks::slime_block(state), blocks::sticky_block(state))
    }
    pub fn stick_to(state: &super::BlockState, other: &super::BlockState) -> Option<bool> {
        blocks::stick_to(state, other)
    }
    pub fn landing_damage(c: &mut super::FallOnContext) {
        blocks::fall_on(&c.state.block.to_string(), c);
    }
    pub fn after_fall(c: &mut super::AfterFallContext) {
        let handler = blocks::after_fall_handler(&c.state.block.to_string()).unwrap();
        blocks::apply_after_fall(&handler, c);
    }
    pub fn inside_motion(c: &mut super::EntityInsideMotionContext) {
        let handler = blocks::inside_motion_handler(&c.state.block.to_string()).unwrap();
        blocks::apply_inside_motion(&handler, c);
    }
    pub fn path_node(c: &super::PathNodeTypeContext<'_>) -> Option<super::PathNodeType> {
        blocks::path_node_type(c)
    }
    /// Same dispatch the native random-tick hook performs; true when a behaviour answered.
    pub fn random_tick(block: &str, c: &mut super::TickContext<'_>) -> bool {
        blocks::random_tick(block, c)
    }
    pub fn can_bonemeal(block: &str, c: &super::BonemealContext<'_>) -> Option<bool> {
        blocks::can_bonemeal(block, c)
    }
    pub fn bonemeal_success(block: &str, c: &mut super::TickContext<'_>) -> Option<bool> {
        blocks::bonemeal_success(block, c)
    }
    pub fn on_bonemeal(block: &str, c: &mut super::TickContext<'_>) {
        blocks::on_bonemeal(block, c)
    }
    pub fn survives(block: &str, c: &super::SurviveContext<'_>) -> Option<bool> {
        blocks::survives(block, c)
    }
    pub fn placement(block: &str, c: &super::PlacementContext<'_>) -> super::Placement {
        blocks::placement(block, c)
    }
    pub fn shape_update(block: &str, c: &super::ShapeUpdateContext<'_>) -> Option<super::BlockState> {
        blocks::shape_update(block, c)
    }
    /// Same dispatch called by the native server-payload hook; no socket simulation.
    pub fn client_payload(channel: &str, data: &[u8]) {
        super::pure::networking_api_v1::from_server(channel, data);
    }
    pub fn item_registry(stacks: Vec<super::StackData>, remainders: Vec<(String, Option<String>)>) -> Result<super::ItemRegistrySnapshot, String> {
        let entries=stacks.iter().map(|stack|stack.item.clone()).collect::<Vec<_>>();
        let lookup=super::RegistryLookup::from_parts(&entries,&[])?;
        super::ItemRegistrySnapshot::decode(&lookup,stacks)?.with_crafting_remainders(remainders)
    }
    /// Deliver a captured batch through the production reload scope/dispatcher.
    pub fn server_json_reload(listener: &str, directory: &str, files: &[(String, serde_json::Value)],
        items: Option<(&[String], &[(String, Vec<String>)])>) -> Result<bool, String> {
        server_json_reload_folders(listener, directory, files, items, Default::default())
    }
    pub fn server_json_reload_folders(listener: &str, directory: &str, files: &[(String, serde_json::Value)],
        items: Option<(&[String], &[(String, Vec<String>)])>,
        folders: std::collections::BTreeMap<String, Vec<super::JsonResourceStack>>) -> Result<bool, String> {
        use super::pure::resource_loader_v0 as resources;
        let mut registries = std::collections::BTreeMap::new();
        if let Some((entries, tags)) = items {
            registries.insert(super::Identifier::new("minecraft", "item").unwrap(),
                super::RegistryLookup::from_parts(entries, tags)?);
        }
        resources::with_reload_folders(registries, folders, || resources::apply_json_reload_listener(
            super::ResourceType::ServerData, listener, directory, files))
    }
    /// A captured registry with tags: `tags` pairs a tag id with its members, which must be entries.
    pub fn registry_with_tags(entries: &[&str], tags: &[(&str, &[&str])]) -> Result<super::RegistryLookup, String> {
        let tags = tags.iter().map(|(tag, members)| ((*tag).to_owned(), members.iter().map(|id| (*id).to_owned()).collect())).collect::<Vec<_>>();
        super::RegistryLookup::from_parts(&entries.iter().map(|id| (*id).to_owned()).collect::<Vec<_>>(), &tags)
    }
    pub fn registry(entries: &[&str]) -> Result<super::RegistryLookup, String> {
        super::RegistryLookup::from_parts(&entries.iter().map(|id| (*id).to_owned()).collect::<Vec<_>>(), &[])
    }
}

#[cfg(test)]
#[path = "../../../mod/src/pluto-entity-spawn-state.rs"]
mod entity_spawn_state_tests;

#[cfg(test)]
mod spawn_caller_tests;
#[cfg(test)]
mod item_prototype_tests;
#[cfg(test)]
mod item_color_tests;
#[cfg(test)]
mod item_mask_host_tests;
#[cfg(test)]
mod reload_registry_host_tests;
#[cfg(test)]
mod particle_color_tests;
#[cfg(test)]
mod item_particle_factory_tests;
#[cfg(test)]
#[path = "../../../mod/src/pluto-block-settings-access.rs"]
mod block_settings_access_tests;
#[cfg(test)]
mod head_policy_tests;
#[cfg(test)]
mod living_drop_gateway_tests;
#[cfg(test)]
mod living_query_host_tests;
#[cfg(test)]
mod living_attribute_host_tests;
#[cfg(test)]
mod block_settings_host_tests;
#[cfg(test)]
mod living_load_host_tests;
#[cfg(test)]
mod mod_content {
    pub(crate) use crate::block_settings_host_tests::bridge as pluto_item_placement_export;
}
#[cfg(test)]
mod mobs {
    pub(crate) use crate::living_query_host_tests::pluto_living_entity_access;
    pub(crate) use crate::head_policy_tests::nav;
    pub(crate) use crate::item_particle_factory_tests::pluto_entity_particles;
    pub(crate) use crate::player_visibility_access_tests as pluto_player_visibility_access;
}

#[cfg(test)]
mod game {
    pub(crate) use crate::spawn_caller_tests::bridge as pluto_entity_spawn_game;
    pub(crate) use crate::item_particle_factory_tests::bridge as pluto_item_particles;
    pub(crate) use crate::living_drop_gateway_tests::bridge as pluto_living_drops;
}

#[cfg(test)]
#[path = "../../../mod/src/pluto-block-access.rs"]
mod block_access_tests;

#[cfg(test)]
#[path = "../../../mod/src/pluto-structure-access.rs"]
mod structure_access_tests;

// Exercise the actual host-independent bridge bookkeeping, not a copied model.
#[cfg(test)]
#[path = "../../../mod/src/pluto-block.rs"]
mod block_bridge_tests;

#[cfg(test)]
#[path = "../../../mod/src/pluto-item-placement-access.rs"]
mod item_placement_access_tests;
#[cfg(test)]
#[path = "../../../mod/src/pluto-entity-type-access.rs"]
mod entity_type_access_tests;

#[cfg(test)]
#[path = "../../../mod/src/pluto-trade-pool.rs"]
mod trade_pool_tests;

#[cfg(test)]
#[path = "../../../mod/src/pluto-item-mask-tests.rs"]
mod item_mask_adapter_tests;

#[cfg(test)]
#[path = "../../../mod/src/pluto-item-behavior-tests.rs"]
mod item_behavior_adapter_tests;

#[cfg(test)]
#[path = "../../../mod/src/pluto-player-visibility-access.rs"]
mod player_visibility_access_tests;

#[cfg(test)]
#[path = "../../../mod/src/pluto-sky-access.rs"]
mod sky_access_tests;
#[cfg(test)]
#[path = "../../../mod/src/pluto-player-visibility-tests.rs"]
mod player_visibility_adapter_tests;

#[cfg(test)]
#[path = "../../../mod/src/pluto-block-entity-access.rs"]
mod block_entity_access_tests;

#[cfg(test)]
#[path = "../../../mod/src/pluto-client-block-entity-state.rs"]
mod client_block_entity_state_tests;

#[cfg(test)]
#[path = "../../../mod/src/pluto-block-entity-cache.rs"]
mod block_entity_cache_tests;

/// Every page of `pluto-lib/docs/api` is included here, so each ```rust block in it is compiled and run by `cargo test --doc`.
#[cfg(doctest)]
mod api_pages {
    macro_rules! page {
        ($name:ident, $file:literal) => {
            #[doc = include_str!(concat!("../../../docs/api/", $file))]
            pub struct $name;
        };
    }
    page!(Index, "index.md");
    page!(Base, "base.md");
    page!(Lifecycle, "lifecycle.md");
    page!(Networking, "networking.md");
    page!(Interaction, "interaction.md");
    page!(Entities, "entities.md");
    page!(Loot, "loot.md");
    page!(Resources, "resources.md");
    page!(Items, "items.md");
    page!(Trades, "trades.md");
    page!(Creative, "creative.md");
    page!(BlockEntity, "block-entity.md");
    page!(Color, "color.md");
    page!(BlockBehavior, "block.md");
    page!(Screen, "screen.md");
    page!(Renderer, "renderer.md");
    page!(Fluid, "fluid.md");
    page!(Neighbours, "neighbours.md");
    page!(FluidItems, "fluid-items.md");
    page!(I18n, "i18n.md");
    page!(World, "world.md");
    page!(Worldgen, "worldgen.md");
    page!(Projectile, "projectile.md");
    page!(Book, "book.md");
    page!(Recipe, "recipe.md");
    page!(Player, "player.md");
    page!(Attachments, "attachments.md");
    page!(Config, "config.md");
    page!(Commands, "commands.md");
    page!(Particles, "particles.md");
    page!(Sky, "sky.md");
    page!(Retexture, "retexture.md");
    page!(Map, "map.md");
    page!(Movement, "movement.md");
}

