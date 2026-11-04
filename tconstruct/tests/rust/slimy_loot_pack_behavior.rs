// Include inside a harness containing the generated PackStack, pack_stack,
// content_overlay, overlay_files, and tconstruct_source_slimy_loot module.
// Link actual horizon_client_render::datapack DataPack/DataStore and the current
// tconstruct-source-config crate. The harness supplies crate::game::Game with
// world_root/dedicated/marker fields and the generated native_data_pack_files
// method; only its Game/Wasm dispatch boundary is a fixture, not a full client.
// Set HORIZON_RESOURCES to prepared 26.3 resources. Run --test-threads=1 because
// the dedicated-root test temporarily changes cwd. Temp data honors TMPDIR.
#[cfg(test)]
mod integration {
    use super::tconstruct_source_slimy_loot as proof;
    use super::*;
    use std::path::{Path, PathBuf};
    fn dirs(name: &str) -> (PathBuf, PathBuf, PathBuf) {
        let root = std::env::temp_dir()
            .join("slimy-loot-pack-check")
            .join(format!(
                "{}-{}-{name}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        let game = root.join("game");
        let world = game.join("saves/world");
        let resources = root.join("resources");
        std::fs::create_dir_all(&world).unwrap();
        std::fs::create_dir_all(&resources).unwrap();
        (game, world, resources)
    }
    fn write(root: &Path, path: &str, bytes: &[u8]) {
        let p = root.join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }
    fn injector(name: &str) -> String {
        format!("data/tconstruct/mantle/loot_injectors/slimy_foliage_{name}.json")
    }
    fn canonical(name: &str) -> Vec<u8> {
        let bytes: &[u8] = match name {
            "dungeon" => include_bytes!(
                "../../content/data/tconstruct/mantle/loot_injectors/slimy_foliage_dungeon.json"
            ),
            "nether_fortress" => include_bytes!(
                "../../content/data/tconstruct/mantle/loot_injectors/slimy_foliage_nether_fortress.json"
            ),
            "bastion" => include_bytes!(
                "../../content/data/tconstruct/mantle/loot_injectors/slimy_foliage_bastion.json"
            ),
            "end_city" => include_bytes!(
                "../../content/data/tconstruct/mantle/loot_injectors/slimy_foliage_end_city.json"
            ),
            _ => panic!("unsupported injector {name}"),
        };
        bytes.to_vec()
    }
    fn builtin(resources: &Path) {
        for (_, name, _, _) in proof::SPECS {
            write(resources, &injector(name), &canonical(name));
        }
    }
    fn pack(world: &Path, name: &str) -> PathBuf {
        let p = world.join("datapacks").join(name);
        write(
            &p,
            "pack.mcmeta",
            br#"{"pack":{"pack_format":1,"description":"test"}}"#,
        );
        p
    }
    fn load(
        world: &Path,
        res: &Path,
        disabled: bool,
        mut native: Vec<(String, Vec<u8>)>,
    ) -> Vec<(String, Vec<Vec<u8>>)> {
        let game = crate::game::Game {
            world_root: world.to_owned(),
            dedicated: false,
            marker: std::marker::PhantomData,
        };
        game.native_data_pack_files(&mut native);
        overlay_files(world, res, disabled, &[], native)
    }
    fn enabled(files: &[(String, Vec<Vec<u8>>)], name: &str) -> bool {
        proof::proven(files, proof::CONFIG_KEY)
            && proof::proven(files, &format!("{}{name}", proof::PREFIX))
    }
    fn stock(id: &str) -> serde_json::Value {
        serde_json::from_slice(
            &std::fs::read(
                PathBuf::from(
                    std::env::var_os("HORIZON_RESOURCES")
                        .expect("set HORIZON_RESOURCES to prepared 26.3 resources"),
                )
                .join(format!(
                    "data/minecraft/loot_table/{}.json",
                    id.strip_prefix("minecraft:").unwrap()
                )),
            )
            .unwrap(),
        )
        .unwrap()
    }
    #[test]
    fn builtin_presence_absence_canonical_malformed_and_vanilla_disabled() {
        let (_, world, res) = dirs("builtin");
        let files = load(&world, &res, false, vec![]);
        for (_, n, _, _) in proof::SPECS {
            assert!(!enabled(&files, n));
        }
        builtin(&res);
        let yes = load(&world, &res, false, vec![]);
        for (_, n, _, _) in proof::SPECS {
            assert!(enabled(&yes, n));
        }
        for (_, n, _, _) in proof::SPECS {
            write(&res, &injector(n), b"{}");
            let no = load(&world, &res, false, vec![]);
            assert!(!enabled(&no, n));
            assert_ne!(yes, no);
            write(&res, &injector(n), b"{invalid");
            assert!(!enabled(&load(&world, &res, false, vec![]), n));
            write(&res, &injector(n), &canonical(n));
        }
        let no = load(&world, &res, true, vec![]);
        for (_, n, _, _) in proof::SPECS {
            assert!(!enabled(&no, n));
        }
    }
    #[test]
    fn pack_priority_disabled_exclusion_and_reload_change_proof() {
        let (_, world, res) = dirs("priority");
        builtin(&res);
        let on = pack(&world, "canonical");
        let off = pack(&world, "disabled");
        for (_, n, _, _) in proof::SPECS {
            write(&on, &injector(n), &canonical(n));
            write(&off, &injector(n), b"{}");
        }
        write(
            &world,
            "datapacks.json",
            br#"{"enabled":["file/canonical","file/disabled"]}"#,
        );
        let no = load(&world, &res, false, vec![]);
        for (_, n, _, _) in proof::SPECS {
            assert!(!enabled(&no, n));
        }
        write(
            &world,
            "datapacks.json",
            br#"{"enabled":["file/disabled","file/canonical"]}"#,
        );
        let yes = load(&world, &res, false, vec![]);
        for (_, n, _, _) in proof::SPECS {
            assert!(enabled(&yes, n));
        }
        assert_ne!(yes, no);
        write(
            &world,
            "datapacks.json",
            br#"{"enabled":["file/canonical"],"disabled":["file/disabled"]}"#,
        );
        for (_, n, _, _) in proof::SPECS {
            assert!(enabled(&load(&world, &res, false, vec![]), n));
        }
        write(&on, &injector("dungeon"), b"{}");
        let changed = load(&world, &res, false, vec![]);
        assert!(!enabled(&changed, "dungeon"));
        assert!(enabled(&changed, "bastion"));
        assert_ne!(changed, yes);
    }
    #[test]
    fn live_config_defaults_true_false_invalid_and_reload() {
        let (game, world, res) = dirs("config");
        builtin(&res);
        let yes = load(&world, &res, false, vec![]);
        assert!(enabled(&yes, "dungeon"));
        for data in [
            b"[loot]\nslimy_loot = false\n".as_slice(),
            b"[loot]\nslimy_loot = 1",
            b"[loot",
            b"loot = 1",
        ] {
            write(&game, "config/tconstruct-common.toml", data);
            let no = load(&world, &res, false, vec![]);
            assert!(!enabled(&no, "dungeon"), "{data:?}");
            assert_ne!(yes, no);
        }
        write(
            &game,
            "config/tconstruct-common.toml",
            b"[loot]\nslimy_loot = true\n",
        );
        assert_eq!(yes, load(&world, &res, false, vec![]));
    }
    #[test]
    fn native_overrides_reject_even_malformed_and_cannot_spoof_proof() {
        let (game, world, res) = dirs("native");
        builtin(&res);
        for key in [
            "mantle/tconstruct:loot_injectors/slimy_foliage_dungeon",
            "loot_table/minecraft:chests/simple_dungeon",
        ] {
            for bytes in [b"{}".as_slice(), b"{invalid"] {
                let files = load(
                    &world,
                    &res,
                    false,
                    vec![
                        (key.into(), bytes.to_vec()),
                        (proof::CONFIG_KEY.into(), b"true".to_vec()),
                    ],
                );
                assert!(!enabled(&files, "dungeon"));
                if bytes == b"{invalid" {
                    assert!(!files.iter().any(|(p, _)| p == key));
                }
            }
        }
        let normal = load(
            &world,
            &res,
            false,
            vec![("recipe/test:ordinary".into(), b"{}".to_vec())],
        );
        assert!(enabled(&normal, "dungeon"));
        assert!(normal.iter().any(|(p, _)| p == "recipe/test:ordinary"));
        write(
            &game,
            "config/tconstruct-common.toml",
            b"[loot]\nslimy_loot=false",
        );
        let no = load(
            &world,
            &res,
            false,
            vec![(proof::CONFIG_KEY.into(), b"true".to_vec())],
        );
        assert!(!enabled(&no, "dungeon"));
        write(&res, &injector("dungeon"), b"{}");
        let no = load(
            &world,
            &res,
            false,
            vec![(format!("{}dungeon", proof::PREFIX), b"true".to_vec())],
        );
        assert!(!proof::proven(&no, &format!("{}dungeon", proof::PREFIX)));
    }
    #[test]
    fn pack_table_override_identical_or_malformed_prevents_injection() {
        let (_, world, res) = dirs("origin");
        builtin(&res);
        let p = pack(&world, "custom");
        for (id, n, _, _) in proof::SPECS {
            let original = stock(id);
            let path = format!(
                "data/minecraft/loot_table/{}.json",
                id.strip_prefix("minecraft:").unwrap()
            );
            for bytes in [serde_json::to_vec(&original).unwrap(), b"{invalid".to_vec()] {
                write(&p, &path, &bytes);
                let files = load(&world, &res, false, vec![]);
                assert!(enabled(&files, n));
                assert!(files.iter().any(|(k, _)| k == &format!("loot_table/{id}")));
                let mut table = original.clone();
                assert!(!proof::inject(id, &mut table, &files));
                assert_eq!(table, original);
            }
            std::fs::remove_file(p.join(path)).unwrap();
            let files = load(&world, &res, false, vec![]);
            let mut table = original.clone();
            assert!(proof::inject(id, &mut table, &files));
            assert!(!proof::inject(id, &mut table, &files));
        }
    }
    #[test]
    fn external_pack_config_marker_removed_and_all_source_entries_load() {
        let (_, world, res) = dirs("markers");
        builtin(&res);
        let p = pack(&world, "custom");
        write(
            &p,
            "data/!tconstruct/loot_table/slimy_loot_config.json",
            b"false",
        );
        let files = load(&world, &res, false, vec![]);
        assert!(enabled(&files, "dungeon"));
        assert_eq!(
            files.iter().filter(|(k, _)| k == proof::CONFIG_KEY).count(),
            1
        );
        for (id, n, _, _) in proof::SPECS {
            let original = stock(id);
            let mut table = original.clone();
            assert!(proof::inject(id, &mut table, &files));
            let source: serde_json::Value = serde_json::from_slice(&canonical(n)).unwrap();
            for pool in source["pools"].as_array().unwrap() {
                let idx = match pool["name"].as_str().unwrap() {
                    "main" => 0,
                    "pool1" => 1,
                    _ => panic!(),
                };
                let old = original["pools"][idx]["entries"].as_array().unwrap();
                let new = table["pools"][idx]["entries"].as_array().unwrap();
                assert!(new.starts_with(old));
                let additions = &new[old.len()..];
                assert_eq!(additions.len(), pool["entries"].as_array().unwrap().len());
                for (a, e) in additions.iter().zip(pool["entries"].as_array().unwrap()) {
                    assert_eq!(a["name"], e["name"]);
                    assert_eq!(
                        a["weight"].as_u64().unwrap_or(1),
                        e["weight"].as_u64().unwrap_or(1)
                    );
                }
            }
        }
    }
    #[test]
    fn dedicated_config_uses_cwd_not_universe() {
        let (game, world, res) = dirs("dedicated");
        builtin(&res);
        let old = std::env::current_dir().unwrap();
        std::env::set_current_dir(&game).unwrap();
        struct Restore(PathBuf);
        impl Drop for Restore {
            fn drop(&mut self) {
                std::env::set_current_dir(&self.0).unwrap();
            }
        }
        let _guard = Restore(old);
        let game_obj = crate::game::Game {
            world_root: world,
            dedicated: true,
            marker: std::marker::PhantomData,
        };
        write(
            &game,
            "config/tconstruct-common.toml",
            b"[loot]\nslimy_loot=false",
        );
        let mut files = vec![];
        game_obj.native_data_pack_files(&mut files);
        assert_eq!(files.last().unwrap().1, b"false");
        write(
            &game,
            "config/tconstruct-common.toml",
            b"[loot]\nslimy_loot=true",
        );
        let mut files = vec![];
        game_obj.native_data_pack_files(&mut files);
        assert_eq!(files.last().unwrap().1, b"true");
    }
}
