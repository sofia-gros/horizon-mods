// Compile against a host staged with this mod's loot hooks.
#[cfg(test)]
mod tests {
    use horizon_core::random::LegacyRandom;
    use horizon_world::loot::{LootData, PackOverlay};
    #[test]
    fn real_loot_rolls_and_pack_proofs() {
        let root = std::path::PathBuf::from(
            std::env::var("HORIZON_RESOURCES").expect("prepared 26.3 resources"),
        );
        let mut files = vec![(
            "loot_table/!tconstruct:slimy_loot_config".to_owned(),
            vec![b"true".to_vec()],
        )];
        for n in ["dungeon", "bastion", "nether_fortress", "end_city"] {
            files.push((
                format!("loot_table/!tconstruct:slimy_loot_{n}"),
                vec![b"true".to_vec()],
            ));
        }
        let data = LootData::load(&root, &PackOverlay { files: &files }).unwrap();
        let mut rng = LegacyRandom::new(441);
        for (id, expected) in [
            (
                "minecraft:chests/simple_dungeon",
                vec![
                    "tconstruct:earth_slime_sapling",
                    "tconstruct:sky_slime_sapling",
                    "tconstruct:earth_slime_grass_seeds",
                    "tconstruct:sky_slime_grass_seeds",
                ],
            ),
            (
                "minecraft:chests/nether_bridge",
                vec!["tconstruct:blood_slime_grass_seeds"],
            ),
            (
                "minecraft:chests/bastion_bridge",
                vec!["tconstruct:blood_slime_sapling"],
            ),
            (
                "minecraft:chests/end_city_treasure",
                vec![
                    "tconstruct:ender_slime_grass_seeds",
                    "tconstruct:ender_slime_sapling",
                ],
            ),
        ] {
            let mut seen = std::collections::HashSet::new();
            for _ in 0..1000 {
                for stack in data.roll(id, &mut rng, 0.0, None, None) {
                    if expected.contains(&stack.item.as_str()) {
                        if stack.item.ends_with("_seeds") {
                            assert!((2..=4).contains(&stack.count));
                        }
                        seen.insert(stack.item);
                    }
                }
            }
            for item in expected {
                assert!(seen.contains(item), "{id} missed {item}");
            }
        }
        let disabled = vec![(
            "loot_table/!tconstruct:slimy_loot_config".to_owned(),
            vec![b"false".to_vec()],
        )];
        let old =
            horizon_world::loot::shared_tables(&root, &PackOverlay { files: &disabled }).unwrap();
        let on = horizon_world::loot::shared_tables(&root, &PackOverlay { files: &files }).unwrap();
        let key = "minecraft:chests/simple_dungeon";
        assert_eq!(
            on[key]["pools"][0]["entries"].as_array().unwrap().len(),
            old[key]["pools"][0]["entries"].as_array().unwrap().len() + 2
        );
        assert!(!on.contains_key("!tconstruct:slimy_loot_config"));
    }
}
