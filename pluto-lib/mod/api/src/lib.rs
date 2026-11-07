//! PlutoLib: Horizon's Fabric API.
//!
//! An independent mod that other mods call (they declare `pluto_lib` under `[dependencies]` in `horizon-mixin.toml`). Names and
//! behaviour follow Fabric API; see `pluto-lib/docs/design-principles.md` and the specification. Modules mirror Fabric API
//! artifacts (`networking_api_v1` is `fabric-networking-api-v1`). The host-independent core is in [`base`].

pub mod base;
pub mod networking_api_v1;

pub use crate::base::{
    ActionResult, Callback, Combiner, DEFAULT_NAMESPACE, Event, Identifier, IdentifierError, PhaseOrderingCycle, Support, SupportEntry,
    default_phase,
};
pub use crate::networking_api_v1::{PlayerConnection, ServerPlayConnectionEvents};

use horizon_client::mod_api::{Hooks, Host, PlayerLoggedInEvent};

/// PlutoLib's entry point: the launcher calls it before the hooks of every mod that depends on PlutoLib. It turns Horizon's hook
/// points into Fabric-like events.
pub fn hooks() -> Box<dyn Hooks> {
    Box::new(PlutoHooks)
}

struct PlutoHooks;

impl Hooks for PlutoHooks {
    fn player_logged_in(&self, _host: &mut dyn Host, event: &mut PlayerLoggedInEvent<'_>) {
        networking_api_v1::player_joined(event.name.to_string());
    }
}
