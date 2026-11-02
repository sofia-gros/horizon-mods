// BuddingCrystalBlock.randomTick at source revision f4e77d43.
pub(super) const DIRECTIONS: [([i32; 3], &str); 6] = [
    ([0, -1, 0], "down"), ([0, 1, 0], "up"),
    ([0, 0, -1], "north"), ([0, 0, 1], "south"),
    ([-1, 0, 0], "west"), ([1, 0, 0], "east"),
];

pub(super) fn next(source: &str, neighbour: &str, air: bool, source_water: bool) -> Option<String> {
    let kind = match source {
        "tconstruct:budding_sky_slime_crystal" => "sky",
        "tconstruct:budding_earth_slime_crystal" => "earth",
        "tconstruct:budding_ichor_slime_crystal" => "ichor",
        "tconstruct:budding_ender_slime_crystal" => "ender",
        _ => return None,
    };
    let prefix = "tconstruct:";
    if air || source_water { return Some(format!("{prefix}small_{kind}_slime_crystal_bud")); }
    for (from, to) in [("small", "medium"), ("medium", "large"), ("large", "cluster")] {
        if neighbour == format!("{prefix}{from}_{kind}_slime_crystal_bud") {
            return Some(if to == "cluster" { format!("{prefix}{kind}_slime_crystal_cluster") }
                        else { format!("{prefix}{to}_{kind}_slime_crystal_bud") });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grows_all_source_geodes_and_preserves_kind() {
        for kind in ["sky", "earth", "ichor", "ender"] {
            let source = format!("tconstruct:budding_{kind}_slime_crystal");
            let small = next(&source, "minecraft:air", true, false).unwrap();
            let medium = next(&source, &small, false, false).unwrap();
            let large = next(&source, &medium, false, false).unwrap();
            let cluster = next(&source, &large, false, false).unwrap();
            assert_eq!(cluster, format!("tconstruct:{kind}_slime_crystal_cluster"));
            assert_eq!(next(&source, &cluster, false, false), None);
            assert_eq!(next(&source, "minecraft:water", false, true), Some(small));
            assert_eq!(next(&source, "minecraft:stone", false, false), None);
        }
    }
    #[test]
    fn rejects_other_crystal_types_and_unknown_sources() {
        assert_eq!(next("tconstruct:budding_sky_slime_crystal", "tconstruct:small_earth_slime_crystal_bud", false, false), None);
        assert_eq!(next("minecraft:budding_amethyst", "minecraft:air", true, false), None);
    }
}
