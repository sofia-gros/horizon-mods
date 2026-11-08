//! Tinkers' own fluids, defined through PlutoLib. The table (`fluids_table.rs`) is generated from the original `TinkerFluids.java`.

use pluto_lib::{FluidDef, FluidRegistry, Identifier};

use crate::fluids_table::FLUIDS;

/// The fluids the original tints in code; the others carry their colour in their texture.
fn tint(name: &str) -> Option<u32> {
    Some(match name {
        "beetroot_soup" => 0xFF84160D,
        "mushroom_stew" => 0xFFCD8C6F,
        "rabbit_stew" => 0xFF984A2C,
        "meat_soup" => 0xFFE03E35,
        "seared_stone" => 0xFF4F4A47,
        "scorched_stone" => 0xFF3E3029,
        "molten_clay" => 0xFF9B6045,
        "molten_ender" => 0xFF105E51,
        "molten_debris" => 0xFF411E15,
        _ => return None,
    })
}

pub(crate) fn register() {
    for &(name, temperature, luminance, density, viscosity) in FLUIDS {
        let mut def = FluidDef::new(Identifier::new("tconstruct", name).expect("valid fluid id"))
            .temperature(temperature)
            .luminance(luminance)
            .density(density)
            .viscosity(viscosity);
        if let Some(argb) = tint(name) {
            def = def.color(argb);
        }
        // the original gives every fluid but powdered snow (which uses vanilla's bucket) a bucket and blocks in the world
        if name != "powdered_snow" {
            def = def.with_world_blocks().with_bucket();
        }
        FluidRegistry::register(def);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_has_unique_valid_ids_and_the_tints_name_real_fluids() {
        let mut names: Vec<&str> = FLUIDS.iter().map(|f| f.0).collect();
        let count = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), count, "no fluid is listed twice");
        assert!(FLUIDS.iter().all(|f| Identifier::new("tconstruct", f.0).is_ok() && f.2 <= 15));
        assert!(["seared_stone", "molten_debris", "meat_soup"].iter().all(|n| names.contains(n) && tint(n).is_some()));
    }
}
