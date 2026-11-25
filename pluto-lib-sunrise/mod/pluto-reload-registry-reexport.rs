// The trait crate depends on horizon-client, which re-exports client_lib.
// Keep one owner of reload snapshots in client-core; this is only its public path.
pub use sunrise_client_core::registries::*;
