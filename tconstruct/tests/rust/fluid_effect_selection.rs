// Use the actual Pluto catalog adapter backed by official client-core,
// without linking the full rendering client.
extern crate self as horizon_client;
#[path = "../../../pluto-lib/mixins/pluto_native_creative/potion-facts.rs"]
pub mod pluto_potions;
#[path = "../../mixins/tconstruct/source-fluid-effects.rs"]
mod tconstruct_source_fluid_effects;
