// Flowing vanilla fluids share their source fluid's visual attributes and name.
// Keep this display-only: tank contents and transfer identities remain unchanged.
pub(super) fn display_id(fluid: &str) -> &str {
    match fluid {
        "minecraft:flowing_water" => "minecraft:water",
        "minecraft:flowing_lava" => "minecraft:lava",
        _ => fluid,
    }
}

pub(super) fn sprite(fluid: &str) -> String {
    match display_id(fluid) {
        "minecraft:lava" => "block/lava_still".to_owned(),
        "minecraft:water" => "block/water_still".to_owned(),
        _ => format!("block/tconstruct_fluid_{}", fluid.strip_prefix("tconstruct:").unwrap_or("")),
    }
}

#[cfg(test)]
mod tests {
    use super::{display_id, sprite};
    #[test]
    fn vanilla_flowing_aliases_use_source_visuals() {
        for (flowing, still, texture) in [
            ("minecraft:flowing_water", "minecraft:water", "block/water_still"),
            ("minecraft:flowing_lava", "minecraft:lava", "block/lava_still"),
        ] {
            assert_eq!(display_id(flowing), still);
            assert_eq!(sprite(flowing), texture);
            assert_eq!(sprite(flowing), sprite(still));
        }
    }
    #[test]
    fn other_fluid_ids_are_not_normalized() {
        for fluid in ["tconstruct:flowing_honey", "other:flowing_water", "minecraft:milk", ""] {
            assert_eq!(display_id(fluid), fluid);
        }
        assert_eq!(sprite("tconstruct:flowing_honey"), "block/tconstruct_fluid_flowing_honey");
    }
}
