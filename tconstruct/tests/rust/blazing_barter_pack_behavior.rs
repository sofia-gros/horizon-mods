// Include this fixture inside a harness module containing the generated host
// PackStack, pack_stack, content_overlay, and overlay_files, plus the generated
// tconstruct_source_blazing_barter_pack module. Link the real host DataPack and
// DataStore types. Temporary data uses TMPDIR through std::env::temp_dir().
include!("../../mixins/tconstruct/source-blazing-barter-pack-native.rs");
#[cfg(test)]
mod integration {
    use super::tconstruct_source_blazing_barter_pack as proof;
    use super::*;
    use std::path::{Path, PathBuf};
    const CANONICAL: &[u8] = include_bytes!("../fixtures/blazing_barter_source.json");
    fn dirs(name: &str) -> (PathBuf, PathBuf) {
        let base = std::env::temp_dir()
            .join("blazing-barter-pack-behavior")
            .join(format!(
                "{}-{}-{name}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        let world = base.join("world");
        let resources = base.join("resources");
        std::fs::create_dir_all(&world).unwrap();
        std::fs::create_dir_all(&resources).unwrap();
        (world, resources)
    }
    fn write(root: &Path, path: &str, data: &[u8]) {
        let dest = root.join(path);
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        std::fs::write(dest, data).unwrap();
    }
    fn pack(world: &Path, name: &str, bytes: &[u8]) {
        let dir = world.join("datapacks").join(name);
        write(
            &dir,
            "pack.mcmeta",
            br#"{"pack":{"pack_format":1,"description":"test"}}"#,
        );
        write(&dir, proof::INJECTOR_PATH, bytes);
    }
    fn load(
        world: &Path,
        resources: &Path,
        disabled: bool,
        mut native: Vec<(String, Vec<u8>)>,
    ) -> proof::Files {
        tconstruct_blazing_barter_pack_native((), &mut native);
        overlay_files(world, resources, disabled, &[], native)
    }
    #[test]
    fn absent_deleted_and_builtin_override_are_gated() {
        let (world, root) = dirs("builtin");
        assert!(!proof::gate(&load(&world, &root, false, vec![])));
        assert!(!proof::gate(&load(&world, &root, true, vec![])));
        write(&root, proof::INJECTOR_PATH, b"{}");
        assert!(!proof::gate(&load(&world, &root, false, vec![])));
        write(&root, proof::INJECTOR_PATH, CANONICAL);
        assert!(proof::gate(&load(&world, &root, false, vec![])));
        std::fs::remove_file(root.join(proof::INJECTOR_PATH)).unwrap();
        assert!(!proof::gate(&load(&world, &root, false, vec![])));
        write(&root, proof::INJECTOR_PATH, b"{invalid");
        assert!(!proof::gate(&load(&world, &root, false, vec![])));
    }
    #[test]
    fn actual_enabled_pack_order_and_reload_win() {
        let (world, root) = dirs("order");
        pack(&world, "canonical", CANONICAL);
        pack(&world, "disabled", b"{}");
        write(
            &world,
            "datapacks.json",
            br#"{"enabled":["file/canonical","file/disabled"]}"#,
        );
        assert!(!proof::gate(&load(&world, &root, false, vec![])));
        write(
            &world,
            "datapacks.json",
            br#"{"enabled":["file/disabled","file/canonical"]}"#,
        );
        assert!(proof::gate(&load(&world, &root, false, vec![])));
        write(
            &world,
            "datapacks.json",
            br#"{"enabled":["file/canonical"],"disabled":["file/disabled"]}"#,
        );
        assert!(proof::gate(&load(&world, &root, false, vec![])));
        write(
            &world,
            "datapacks.json",
            br#"{"enabled":[],"disabled":["file/disabled","file/canonical"]}"#,
        );
        assert!(!proof::gate(&load(&world, &root, false, vec![])));
        write(
            &world,
            "datapacks.json",
            br#"{"enabled":["file/canonical"],"disabled":["file/disabled"]}"#,
        );
        pack(&world, "canonical", b"{}");
        assert!(!proof::gate(&load(&world, &root, false, vec![])));
    }
    #[test]
    fn malformed_native_override_is_not_lost_when_host_skips_json() {
        let (world, root) = dirs("native");
        write(&root, proof::INJECTOR_PATH, CANONICAL);
        let key = "mantle/tconstruct:loot_injectors/piglin_bartering";
        let files = load(
            &world,
            &root,
            false,
            vec![(key.into(), b"{invalid".to_vec())],
        );
        assert!(!files.iter().any(|(path, _)| path == key));
        assert!(!proof::gate(&files));
        let files = load(
            &world,
            &root,
            false,
            vec![("recipe/test:item".into(), b"{}".to_vec())],
        );
        assert!(proof::gate(&files));
        assert!(files.iter().any(|(path, _)| path == "recipe/test:item"));
    }
    #[test]
    fn custom_loot_table_origin_closes_gate_even_for_identical_bytes() {
        let (world, root) = dirs("table-origin");
        write(&root, proof::INJECTOR_PATH, CANONICAL);
        pack(&world, "custom", CANONICAL);
        let stock = std::fs::read(
            std::env::var_os("TCONSTRUCT_TEST_PIGLIN_BARTER_TABLE")
                .expect("set prepared vanilla 26.3 barter table fixture"),
        )
        .unwrap();
        let dir = world.join("datapacks/custom");
        for bytes in [stock.as_slice(), b"{invalid".as_slice()] {
            write(&dir, proof::TABLE_PATH, bytes);
            assert!(!proof::gate(&load(&world, &root, false, vec![])));
            let native = vec![(proof::TABLE_KEY.into(), bytes.to_vec())];
            assert!(!proof::gate(&load(&world, &root, false, native)));
        }
        std::fs::remove_file(dir.join(proof::TABLE_PATH)).unwrap();
        assert!(proof::gate(&load(&world, &root, false, vec![])));
        for bytes in [stock.as_slice(), b"{invalid".as_slice()] {
            let native = vec![(proof::TABLE_KEY.into(), bytes.to_vec())];
            assert!(!proof::gate(&load(&world, &root, false, native)));
        }
    }
    #[test]
    fn external_sentinels_cannot_enable_disabled_gate() {
        let (world, root) = dirs("spoof");
        write(&root, proof::INJECTOR_PATH, b"{}");
        let spoof = vec![
            (proof::SENTINEL_PATH.into(), b"true".to_vec()),
            (proof::NATIVE_SENTINEL_PATH.into(), b"true".to_vec()),
        ];
        assert!(!proof::gate(&load(&world, &root, false, spoof)));
        let mut native = vec![
            (proof::NATIVE_SENTINEL_PATH.into(), b"true".to_vec()),
            (
                "mantle/tconstruct:loot_injectors/piglin_bartering".into(),
                b"{invalid".to_vec(),
            ),
        ];
        tconstruct_blazing_barter_pack_native((), &mut native);
        assert_eq!(
            native
                .iter()
                .filter(|(key, _)| key == proof::NATIVE_SENTINEL_PATH)
                .count(),
            1
        );
        assert_eq!(native.last().unwrap().1, b"false");
    }
}
