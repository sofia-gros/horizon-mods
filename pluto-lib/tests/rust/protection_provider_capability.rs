// Host-independent source fixture: the real facade, native-policy source and
// scoped capability. This is not a compiled Horizon adapter or game test.
extern crate self as horizon_world;
extern crate pluto_inventory_api as pluto_inventory;
#[path = "../../mixins/pluto_native_creative/protection-context.rs"]
pub mod protection;
pub mod combat_rules {
    pub use crate::protection as pluto_protection;
}
#[path = "../../mixins/pluto_native_creative/protection-contribution.rs"]
mod contribution;
