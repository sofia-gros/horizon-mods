//! The host-independent half of PlutoLib, built without Horizon, plus the compiled documentation examples.
#[path = "../../../mod/api/src/pure/mod.rs"]
pub mod pure;
#[path = "../../../mod/api/src/exports.rs"]
mod exports;

pub use exports::*;

/// Harness-only fixtures: construct through the real snapshot validator without
/// exposing a production API or duplicating its registry-membership rules.
pub mod fixtures {
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
#[path = "../../../mod/pluto-entity-spawn-state.rs"]
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
#[path = "../../../mod/pluto-block-settings-access.rs"]
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
#[path = "../../../mod/pluto-skull-model-access.rs"]
pub(crate) mod skull_model_access_tests;
#[cfg(test)]
mod skull_render_host_tests;
#[cfg(test)]
mod item_model {pub(crate) use crate::skull_model_access_tests as pluto_skull_model_access;}
#[cfg(test)]
mod block_entity_render {pub(crate) use crate::skull_render_host_tests::ModelPart;}
#[cfg(test)]
mod item_render {pub(crate) use crate::skull_render_host_tests::parts as pluto_skull_item_models;}
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
#[path = "../../../mod/pluto-block-access.rs"]
mod block_access_tests;

#[cfg(test)]
#[path = "../../../mod/pluto-structure-access.rs"]
mod structure_access_tests;

// Exercise the actual host-independent bridge bookkeeping, not a copied model.
#[cfg(test)]
#[path = "../../../mod/pluto-block.rs"]
mod block_bridge_tests;

#[cfg(test)]
#[path = "../../../mod/pluto-item-placement-access.rs"]
mod item_placement_access_tests;
#[cfg(test)]
#[path = "../../../mod/pluto-entity-type-access.rs"]
mod entity_type_access_tests;

#[cfg(test)]
#[path = "../../../mod/pluto-trade-pool.rs"]
mod trade_pool_tests;

#[cfg(test)]
#[path = "../../../mod/pluto-item-mask-tests.rs"]
mod item_mask_adapter_tests;

#[cfg(test)]
#[path = "../../../mod/pluto-player-visibility-access.rs"]
mod player_visibility_access_tests;
#[cfg(test)]
#[path = "../../../mod/pluto-player-visibility-tests.rs"]
mod player_visibility_adapter_tests;

#[cfg(test)]
#[path = "../../../mod/pluto-block-entity-access.rs"]
mod block_entity_access_tests;

#[cfg(test)]
#[path = "../../../mod/pluto-client-block-entity-state.rs"]
mod client_block_entity_state_tests;

#[cfg(test)]
#[path = "../../../mod/pluto-block-entity-cache.rs"]
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
    page!(Player, "player.md");
    page!(Attachments, "attachments.md");
    page!(Config, "config.md");
    page!(Commands, "commands.md");
}
