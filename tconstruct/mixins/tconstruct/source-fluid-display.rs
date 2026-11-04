// Pinned flowing fluids share their source fluid's visual attributes and name.
// Used for display and pinned bucket lookup only; tank identities remain unchanged.
pub(super) fn display_id(fluid: &str) -> &str {
    match fluid {
        "minecraft:flowing_water" => "minecraft:water",
        "minecraft:flowing_lava" => "minecraft:lava",
        "tconstruct:flowing_beetroot_soup" => "tconstruct:beetroot_soup",
        "tconstruct:flowing_blazing_blood" => "tconstruct:blazing_blood",
        "tconstruct:flowing_earth_slime" => "tconstruct:earth_slime",
        "tconstruct:flowing_ender_slime" => "tconstruct:ender_slime",
        "tconstruct:flowing_fiery_liquid" => "tconstruct:fiery_liquid",
        "tconstruct:flowing_honey" => "tconstruct:honey",
        "tconstruct:flowing_ichor" => "tconstruct:ichor",
        "tconstruct:flowing_liquid_soul" => "tconstruct:liquid_soul",
        "tconstruct:flowing_magma" => "tconstruct:magma",
        "tconstruct:flowing_meat_soup" => "tconstruct:meat_soup",
        "tconstruct:flowing_molten_aluminum" => "tconstruct:molten_aluminum",
        "tconstruct:flowing_molten_amethyst" => "tconstruct:molten_amethyst",
        "tconstruct:flowing_molten_amethyst_bronze" => "tconstruct:molten_amethyst_bronze",
        "tconstruct:flowing_molten_bendalloy" => "tconstruct:molten_bendalloy",
        "tconstruct:flowing_molten_brass" => "tconstruct:molten_brass",
        "tconstruct:flowing_molten_bronze" => "tconstruct:molten_bronze",
        "tconstruct:flowing_molten_cadmium" => "tconstruct:molten_cadmium",
        "tconstruct:flowing_molten_chromium" => "tconstruct:molten_chromium",
        "tconstruct:flowing_molten_cinderslime" => "tconstruct:molten_cinderslime",
        "tconstruct:flowing_molten_clay" => "tconstruct:molten_clay",
        "tconstruct:flowing_molten_cobalt" => "tconstruct:molten_cobalt",
        "tconstruct:flowing_molten_constantan" => "tconstruct:molten_constantan",
        "tconstruct:flowing_molten_copper" => "tconstruct:molten_copper",
        "tconstruct:flowing_molten_debris" => "tconstruct:molten_debris",
        "tconstruct:flowing_molten_diamond" => "tconstruct:molten_diamond",
        "tconstruct:flowing_molten_duralumin" => "tconstruct:molten_duralumin",
        "tconstruct:flowing_molten_electrum" => "tconstruct:molten_electrum",
        "tconstruct:flowing_molten_emerald" => "tconstruct:molten_emerald",
        "tconstruct:flowing_molten_ender" => "tconstruct:molten_ender",
        "tconstruct:flowing_molten_enderium" => "tconstruct:molten_enderium",
        "tconstruct:flowing_molten_glass" => "tconstruct:molten_glass",
        "tconstruct:flowing_molten_gold" => "tconstruct:molten_gold",
        "tconstruct:flowing_molten_hepatizon" => "tconstruct:molten_hepatizon",
        "tconstruct:flowing_molten_invar" => "tconstruct:molten_invar",
        "tconstruct:flowing_molten_iron" => "tconstruct:molten_iron",
        "tconstruct:flowing_molten_knightmetal" => "tconstruct:molten_knightmetal",
        "tconstruct:flowing_molten_knightslime" => "tconstruct:molten_knightslime",
        "tconstruct:flowing_molten_lead" => "tconstruct:molten_lead",
        "tconstruct:flowing_molten_lumium" => "tconstruct:molten_lumium",
        "tconstruct:flowing_molten_manyullyn" => "tconstruct:molten_manyullyn",
        "tconstruct:flowing_molten_netherite" => "tconstruct:molten_netherite",
        "tconstruct:flowing_molten_nickel" => "tconstruct:molten_nickel",
        "tconstruct:flowing_molten_nicrosil" => "tconstruct:molten_nicrosil",
        "tconstruct:flowing_molten_obsidian" => "tconstruct:molten_obsidian",
        "tconstruct:flowing_molten_osmium" => "tconstruct:molten_osmium",
        "tconstruct:flowing_molten_pewter" => "tconstruct:molten_pewter",
        "tconstruct:flowing_molten_pig_iron" => "tconstruct:molten_pig_iron",
        "tconstruct:flowing_molten_platinum" => "tconstruct:molten_platinum",
        "tconstruct:flowing_molten_porcelain" => "tconstruct:molten_porcelain",
        "tconstruct:flowing_molten_quartz" => "tconstruct:molten_quartz",
        "tconstruct:flowing_molten_queens_slime" => "tconstruct:molten_queens_slime",
        "tconstruct:flowing_molten_refined_glowstone" => "tconstruct:molten_refined_glowstone",
        "tconstruct:flowing_molten_refined_obsidian" => "tconstruct:molten_refined_obsidian",
        "tconstruct:flowing_molten_rose_gold" => "tconstruct:molten_rose_gold",
        "tconstruct:flowing_molten_signalum" => "tconstruct:molten_signalum",
        "tconstruct:flowing_molten_silver" => "tconstruct:molten_silver",
        "tconstruct:flowing_molten_slimesteel" => "tconstruct:molten_slimesteel",
        "tconstruct:flowing_molten_soulsteel" => "tconstruct:molten_soulsteel",
        "tconstruct:flowing_molten_steel" => "tconstruct:molten_steel",
        "tconstruct:flowing_molten_steeleaf" => "tconstruct:molten_steeleaf",
        "tconstruct:flowing_molten_tin" => "tconstruct:molten_tin",
        "tconstruct:flowing_molten_tungsten" => "tconstruct:molten_tungsten",
        "tconstruct:flowing_molten_uranium" => "tconstruct:molten_uranium",
        "tconstruct:flowing_molten_zinc" => "tconstruct:molten_zinc",
        "tconstruct:flowing_mushroom_stew" => "tconstruct:mushroom_stew",
        "tconstruct:flowing_rabbit_stew" => "tconstruct:rabbit_stew",
        "tconstruct:flowing_scorched_stone" => "tconstruct:scorched_stone",
        "tconstruct:flowing_seared_stone" => "tconstruct:seared_stone",
        "tconstruct:flowing_sky_slime" => "tconstruct:sky_slime",
        "tconstruct:flowing_venom" => "tconstruct:venom",
        _ => fluid,
    }
}

pub(super) fn sprite(fluid: &str) -> String {
    let fluid = display_id(fluid);
    match fluid {
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
        for fluid in ["tconstruct:flowing_unknown", "other:flowing_water", "minecraft:milk", ""] {
            assert_eq!(display_id(fluid), fluid);
        }
        assert_eq!(sprite("tconstruct:flowing_unknown"), "block/tconstruct_fluid_flowing_unknown");
    }
    #[test]
    fn tconstruct_flowing_aliases_use_source_visuals() {
        for (flowing, still) in [
            ("tconstruct:flowing_honey", "tconstruct:honey"),
            ("tconstruct:flowing_molten_copper", "tconstruct:molten_copper"),
            ("tconstruct:flowing_scorched_stone", "tconstruct:scorched_stone"),
        ] {
            assert_eq!(display_id(flowing), still);
            assert_eq!(sprite(flowing), sprite(still));
        }
    }
}
