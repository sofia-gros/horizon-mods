//! Host-independent PlutoLib: no dependency on Horizon, so it builds and tests in seconds.
//! (`pluto-lib/tests/pure` compiles this module on its own; `hooks.rs` connects it to Horizon.)

pub mod base;
pub mod entity_events_v1;
pub mod events_interaction_v0;
pub mod item_api_v1;
pub mod lifecycle_events_v1;
pub mod loot_api_v2;
pub mod networking_api_v1;
pub mod resource_loader_v0;
