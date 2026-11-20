//! The fluids of SlimeKnights TinkerFluids (MIT; see LICENSE), defined through PlutoLib's `FluidRegistry`.
//!
//! Each of the 72 source fluids takes the attributes its FluidType.Properties give it (tools/import_original_data.py reads them
//! into `fluid_defs`), and its name is the original's `fluid.tconstruct.<name>` translation. World blocks, flowing variants and
//! buckets are not declared: the host has none for a mod fluid, and `with_world_blocks`/`with_bucket` would only record a wish.
//! The fluid tags (`data/tconstruct/tags/fluid`) are read by the host and reach this mod through `reload_registry("minecraft:fluid")`.
use pluto_lib::{FluidDef, FluidRegistry, Identifier};
use crate::fluid_defs::{FLUIDS, FluidSpec};

/// The Tinkers namespace.
const NAMESPACE: &str = "tconstruct";

fn definition(spec: &FluidSpec) -> FluidDef {
    FluidDef::new(Identifier::new(NAMESPACE, spec.name).expect("generated fluid ids are valid"))
        .temperature(spec.temperature)
        .luminance(spec.luminance)
        .density(spec.density)
        .viscosity(spec.viscosity)
        .color(spec.color)
}

/// All fluid definitions in the original's registration order.
pub(crate) fn definitions() -> Vec<FluidDef> { FLUIDS.iter().map(definition).collect() }

/// Defines every fluid. Registering an id again replaces it, so calling this twice is harmless.
pub(crate) fn register() {
    for def in definitions() { FluidRegistry::register(def); }
}
