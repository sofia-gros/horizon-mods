#[path = "../../mixins/tconstruct/source-slime-seed-rules.rs"]
mod rules;
use rules::{conversion, ResultKind};
#[test]
fn every_seed_converts_each_supported_soil() {
    for foliage in ["earth", "sky", "ender", "blood", "ichor"] {
        for soil in ["vanilla", "earth", "sky", "ender", "ichor"] {
            let item = format!("tconstruct:{foliage}_slime_grass_seeds");
            let block = if soil == "vanilla" {
                "minecraft:dirt".into()
            } else {
                format!("tconstruct:{soil}_slime_dirt")
            };
            assert_eq!(
                conversion(&item, &block),
                Some(ResultKind::Grass(foliage, soil))
            );
        }
    }
}
#[test]
fn vine_conversion_only_for_sky_and_ender() {
    for foliage in ["earth", "sky", "ender", "blood", "ichor"] {
        let item = format!("tconstruct:{foliage}_slime_grass_seeds");
        let expected = if matches!(foliage, "sky" | "ender") {
            Some(ResultKind::Vine(foliage))
        } else {
            None
        };
        assert_eq!(conversion(&item, "minecraft:vine"), expected);
    }
}
#[test]
fn other_items_and_blocks_are_not_claimed() {
    for item in ["minecraft:wheat_seeds", "tconstruct:not_slime_grass_seeds"] {
        assert_eq!(conversion(item, "minecraft:dirt"), None);
    }
    for block in [
        "minecraft:grass_block",
        "minecraft:coarse_dirt",
        "minecraft:cave_vines",
        "tconstruct:earth_earth_slime_grass",
    ] {
        assert_eq!(
            conversion("tconstruct:earth_slime_grass_seeds", block),
            None
        );
    }
}
