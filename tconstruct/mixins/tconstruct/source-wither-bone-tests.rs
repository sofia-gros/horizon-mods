#[cfg(test)]
mod tests {
    use super::super::{tconstruct_source_wither_bone as proof, *};
    use horizon_core::random::RandomSource;
    use horizon_world::loot::PackOverlay;
    use serde_json::json;

    #[test]
    fn wither_bone_reload_counts_components_rng_and_other_tables() {
        let root = std::env::temp_dir().join(format!("wither-bone-{}", std::process::id()));
        let dir = root.join("data/minecraft/loot_table/entities");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(root.join("items.json"), "{}").unwrap();
        // Synthetic fixture, not redistributed Minecraft data. Includes count
        // randomness and old components to prove whole-stack replacement.
        let table = json!({"random_sequence":"test:wither", "pools":[{"rolls":1,"entries":[{"type":"minecraft:item","name":"minecraft:bone","modifier":[
            {"type":"minecraft:set_count","count":{"type":"minecraft:uniform","min":1,"max":6}},
            {"type":"minecraft:set_components","components":{"minecraft:custom_name":"old"}}
        ]}]},{"rolls":1,"entries":[{"type":"minecraft:item","name":"minecraft:coal"}]}]});
        for name in ["wither_skeleton", "skeleton"] {
            std::fs::write(
                dir.join(format!("{name}.json")),
                serde_json::to_vec(&table).unwrap(),
            )
            .unwrap();
        }
        let baseline = LootData::load(&root, &PackOverlay::NONE).unwrap();
        for enabled in [true, false, true, false] {
            let files = vec![
                (
                    proof::CONFIG.into(),
                    vec![if enabled {
                        b"true".to_vec()
                    } else {
                        b"false".to_vec()
                    }],
                ),
                (proof::PACK.into(), vec![b"true".to_vec()]),
            ];
            let data = LootData::load(&root, &PackOverlay { files: &files }).unwrap();
            for table_id in [proof::TABLE, "minecraft:entities/skeleton"] {
                for seed in 0..32 {
                    let mut before = RandomSequences::default();
                    let mut after = before.clone();
                    let params = EntityLootParams::default();
                    let original = roll(&death(&baseline, seed), table_id, &params, &mut before);
                    let actual = roll(&death(&data, seed), table_id, &params, &mut after);
                    assert_eq!(
                        before.get("test:wither", seed).next_long(),
                        after.get("test:wither", seed).next_long()
                    );
                    assert_eq!(original.len(), 2);
                    assert_eq!(original.len(), actual.len());
                    for (old, new) in original.iter().zip(&actual) {
                        if enabled && table_id == proof::TABLE && old.item == "minecraft:bone" {
                            let expected = loot_stack("tconstruct:necrotic_bone", old.count);
                            assert_eq!(*new, expected);
                        } else {
                            assert_eq!(old, new);
                        }
                    }
                }
            }
        }
        // A world-pack replacement closes the gate even with spoofed proofs.
        let replaced = table.clone();
        let files = vec![
            (
                format!("loot_table/{}", proof::MARKER),
                vec![b"{}".to_vec()],
            ),
            (proof::CONFIG.into(), vec![b"true".to_vec()]),
            (proof::PACK.into(), vec![b"true".to_vec()]),
            (
                format!("loot_table/{}", proof::TABLE),
                vec![serde_json::to_vec(&replaced).unwrap()],
            ),
        ];
        let data = LootData::load(&root, &PackOverlay { files: &files }).unwrap();
        assert_eq!(
            roll(
                &death(&data, 7),
                proof::TABLE,
                &EntityLootParams::default(),
                &mut RandomSequences::default()
            ),
            roll(
                &death(&baseline, 7),
                proof::TABLE,
                &EntityLootParams::default(),
                &mut RandomSequences::default()
            )
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    fn death(loot: &LootData, world_seed: i64) -> Death<'_> {
        Death {
            loot,
            damage: None,
            player: None,
            luck: 0.0,
            vehicle: None,
            mob_drops: true,
            world_seed,
        }
    }
}
