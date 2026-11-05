// Use the same actual host potion registry re-exported by horizon-client's
// brewing module, without linking the full rendering client.
extern crate self as horizon_client;
pub mod brewing { pub use horizon_client_core::potions::{potion, effect, is_mod_effect}; }
#[path = "../../mixins/tconstruct/source-fluid-effects.rs"]
mod tconstruct_source_fluid_effects;
