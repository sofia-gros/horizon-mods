extern crate self as horizon_client;
pub mod containers {
    pub enum BlockEntity { Chest(Vec<Option<horizon_items::items::ItemStack>>) }
}
#[path = "faucet_tank_runtime.rs"]
mod runtime;
