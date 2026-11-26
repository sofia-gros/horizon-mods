// The trait crate depends on horizon-client, which re-exports client_lib.
// The reload snapshots' owner module (pluto_reload_registry) is woven into
// client_lib itself — sunrise-client-core takes no support modules — and
// this is only its public path under `registries`, beside the core module's
// own contents.
pub use sunrise_client_core::registries::*;
pub use crate::pluto_reload_registry;
