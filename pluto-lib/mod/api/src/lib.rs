//! PlutoLib: Horizon's Fabric API.
//!
//! An independent mod that other mods call (they declare `pluto_lib` under `[dependencies]` in `horizon-mixin.toml`). Names and
//! behaviour follow Fabric API; see `pluto-lib/docs/design-principles.md` and the specification. Modules mirror Fabric API
//! artifacts (`lifecycle_events_v1` is `fabric-lifecycle-events-v1`). The host-independent part is [`pure`]; [`hooks`] is the
//! one file that connects it to Horizon.

mod exports;
mod hooks;
pub mod pure;

pub use crate::exports::*;
pub use crate::hooks::hooks;
pub use crate::hooks::{no_hooks, ModHooks};
