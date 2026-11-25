//! PlutoLib: Horizon's Fabric API.
//!
//! An independent mod that other mods call (they declare `pluto_lib` under `[dependencies]` in `sunrise-mod.toml`). Names and
//! behaviour follow Fabric API; see `pluto-lib/docs/design-principles.md` and the specification. Modules mirror Fabric API
//! artifacts (`lifecycle_events_v1` is `fabric-lifecycle-events-v1`). The host-independent part is [`pure`]; [`hooks`] is the
//! one file that connects it to the game.

mod exports;
mod hooks;
pub mod pure;

pub use crate::exports::*;

/// Block movement factors (speed and jump) for mod blocks.
pub use pluto_movement_api as movement;

/// PlutoLib's native entry point. The hook bodies live in [`hooks`], which is
/// being ported from the Horizon host bridge to this crate's `mod_api`
/// surface; methods are wired up there as the port lands.
#[derive(Default)]
pub struct Mod;

#[mod_api::sunrise_mod]
impl mod_api::Hooks for Mod {}
