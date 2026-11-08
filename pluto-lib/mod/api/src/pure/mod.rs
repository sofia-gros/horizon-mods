//! Host-independent PlutoLib: no dependency on Horizon, so it builds and tests in seconds.
//! (`pluto-lib/tests/pure` compiles this module on its own; `hooks.rs` connects it to Horizon.)

pub mod base;
pub mod block_api_v1;
pub mod block_entity_api_v1;
pub mod block_entity_renderer_api_v1;
pub mod color_provider_api_v1;
pub mod command_api_v2;
pub mod config_api;
pub mod data_attachment_api_v1;
pub mod entity_events_v1;
pub mod events_interaction_v0;
pub mod fluid_api_v1;
pub mod i18n_api;
pub mod item_api_v1;
pub mod item_group_api_v1;
pub mod lifecycle_events_v1;
pub mod loot_api_v2;
pub mod networking_api_v1;
pub mod player_api;
pub mod projectile_events_v1;
pub mod resource_loader_v0;
pub mod screen_handler_api_v1;
pub mod world_api_v1;
