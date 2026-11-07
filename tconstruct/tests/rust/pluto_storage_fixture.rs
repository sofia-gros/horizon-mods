// The fixture manifest depends on the actual pluto-inventory-api package.
// Link native adapters without duplicating the public API or its registries.
extern crate self as horizon_client;
pub use pluto_inventory_api as pluto_inventory;
#[path = "../../../pluto-lib/mixins/pluto_native_creative/stack-storage.rs"]
pub mod pluto_stack_storage;
#[path = "../../../pluto-lib/mixins/pluto_native_creative/nbt-storage.rs"]
pub mod pluto_nbt_storage;
#[path = "../../../pluto-lib/mixins/pluto_native_creative/stack-cells.rs"]
pub mod pluto_stack_cells;
