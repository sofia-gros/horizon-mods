// Plant support rules informed by Tinkers' Construct SlimeTallGrassBlock,
// Copyright (c) 2022 SlimeKnights, MIT; see tconstruct/third_party/tconstruct/LICENSE.

/// Recognize only the five variants of the two short slime plants.
pub(super) fn is_plant(name: &str) -> bool {
    let Some(name) = name.strip_prefix("tconstruct:") else {
        return false;
    };
    let Some((foliage, plant)) = name.split_once('_') else {
        return false;
    };
    matches!(foliage, "earth" | "sky" | "ichor" | "ender" | "blood")
        && matches!(plant, "slime_fern" | "slime_tall_grass")
}

/// Original plants accept any of the four slime dirts and all 25 grass variants.
pub(super) fn supports(name: &str) -> bool {
    if matches!(
        name,
        "tconstruct:earth_slime_dirt"
            | "tconstruct:sky_slime_dirt"
            | "tconstruct:ichor_slime_dirt"
            | "tconstruct:ender_slime_dirt"
    ) {
        return true;
    }
    let Some(name) = name
        .strip_prefix("tconstruct:")
        .and_then(|s| s.strip_suffix("_slime_grass"))
    else {
        return false;
    };
    let Some((foliage, soil)) = name.split_once('_') else {
        return false;
    };
    matches!(foliage, "earth" | "sky" | "ichor" | "ender" | "blood")
        && matches!(soil, "earth" | "sky" | "ichor" | "ender" | "vanilla")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_variants_and_ground_combinations() {
        for foliage in ["earth", "sky", "ichor", "ender", "blood"] {
            for plant in ["slime_fern", "slime_tall_grass"] {
                assert!(is_plant(&format!("tconstruct:{foliage}_{plant}")));
            }
            for soil in ["earth", "sky", "ichor", "ender", "vanilla"] {
                assert!(supports(&format!(
                    "tconstruct:{foliage}_{soil}_slime_grass"
                )));
            }
        }
        for soil in ["earth", "sky", "ichor", "ender"] {
            assert!(supports(&format!("tconstruct:{soil}_slime_dirt")));
        }
    }
    #[test]
    fn rejects_vanilla_and_similarly_named_blocks() {
        for name in [
            "minecraft:fern",
            "tconstruct:earth_slime_sapling",
            "other:earth_slime_fern",
            "tconstruct:purple_slime_fern",
        ] {
            assert!(!is_plant(name));
        }
        for name in [
            "minecraft:dirt",
            "minecraft:grass_block",
            "tconstruct:blood_slime_dirt",
            "other:earth_slime_dirt",
            "tconstruct:earth_bad_slime_grass",
        ] {
            assert!(!supports(name));
        }
    }
}
