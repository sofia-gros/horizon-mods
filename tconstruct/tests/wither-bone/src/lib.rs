#![allow(dead_code)]
#[path = "../../../mixins/tconstruct/source-wither-bone.rs"]
mod tconstruct_source_wither_bone;
mod game {
    pub struct Game<'a> {
        pub world_root: std::path::PathBuf,
        pub unused: std::marker::PhantomData<&'a ()>,
    }
    impl Game<'_> {
        pub fn standalone_server(&self) -> bool {
            false
        }
    }
}
include!("../../../mixins/tconstruct/source-wither-bone-native.rs");
#[derive(Clone)]
struct DataPack(std::path::PathBuf);
impl DataPack {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        std::fs::read(self.0.join(path)).ok()
    }
    fn list(&self, prefix: &str) -> Vec<String> {
        fn walk(root: &std::path::Path, path: &std::path::Path, files: &mut Vec<String>) {
            if let Ok(entries) = std::fs::read_dir(path) {
                for e in entries.flatten() {
                    let p = e.path();
                    if p.is_dir() {
                        walk(root, &p, files);
                    } else {
                        files.push(p.strip_prefix(root).unwrap().to_str().unwrap().into());
                    }
                }
            }
        }
        let mut files = Vec::new();
        walk(&self.0, &self.0.join(prefix), &mut files);
        files
    }
}
struct Stack {
    builtin: Option<DataPack>,
    packs: Vec<DataPack>,
}
impl Stack {
    fn enabled_packs(&self) -> impl Iterator<Item = &DataPack> {
        self.packs.iter()
    }
}
fn pack_stack(
    world: &std::path::Path,
    resources: &std::path::Path,
    _: bool,
    _: &[String],
) -> Stack {
    Stack {
        builtin: Some(DataPack(resources.into())),
        packs: vec![DataPack(world.join("pack"))],
    }
}
include!("../../../mixins/tconstruct/source-wither-bone-pack.rs");
#[cfg(test)]
mod tests {
    use super::*;
    use tconstruct_source_wither_bone as proof;
    #[test]
    fn source_config_pack_rebuild_and_fail_closed() {
        let root = std::env::temp_dir().join(format!("wither-proof-{}", std::process::id()));
        let world = root.join("saves/world");
        let resources = root.join("resources");
        std::fs::create_dir_all(&world).unwrap();
        std::fs::create_dir_all(root.join("config")).unwrap();
        let source =
            include_bytes!("../../../content/data/tconstruct/loot_modifiers/wither_bone.json");
        assert!(proof::canonical(source));
        assert!(!proof::canonical(b"{}"));
        let path = resources.join(proof::SOURCE);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, source).unwrap();
        let game = game::Game {
            world_root: world.clone(),
            unused: std::marker::PhantomData,
        };
        for (config, enabled) in [
            ("", true),
            ("[recipes]\nwitherBoneDrop = false", false),
            ("[recipes]\nwitherBoneDrop = true", true),
            ("[recipes]\nwitherBoneDrop = 1", false),
            ("[recipes", false),
        ] {
            std::fs::write(root.join("config/tconstruct-common.toml"), config).unwrap();
            let mut native = Vec::new();
            tconstruct_wither_bone_native((), &game, &mut native);
            let mut overlay =
                tconstruct_wither_bone_pack(Vec::new(), &world, &resources, false, &[]);
            overlay.extend(native.into_iter().map(|(p, v)| (p, vec![v])));
            assert_eq!(proof::enabled(&overlay), enabled, "{config}");
        }
        std::fs::write(root.join("config/tconstruct-common.toml"), "").unwrap();
        for override_path in [
            proof::SOURCE,
            proof::TABLE_PATH,
            "data/forge/loot_modifiers/global_loot_modifiers.json",
        ] {
            let path = world.join("pack").join(override_path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, b"malformed").unwrap();
            let overlay = tconstruct_wither_bone_pack(
                vec![(proof::CONFIG.into(), vec![b"true".to_vec()])],
                &world,
                &resources,
                false,
                &[],
            );
            assert!(!proof::proven(&overlay, proof::PACK));
            assert!(!proof::proven(&overlay, proof::CONFIG));
            std::fs::remove_file(path).unwrap();
        }
        let no_vanilla = tconstruct_wither_bone_pack(Vec::new(), &world, &resources, true, &[]);
        assert!(!proof::proven(&no_vanilla, proof::PACK));
        std::fs::remove_file(resources.join(proof::SOURCE)).unwrap();
        let missing = tconstruct_wither_bone_pack(Vec::new(), &world, &resources, false, &[]);
        assert!(!proof::proven(&missing, proof::PACK));
        let mut native = vec![(
            "loot_table/minecraft:entities/wither_skeleton".into(),
            b"bad".to_vec(),
        )];
        tconstruct_wither_bone_native((), &game, &mut native);
        assert_eq!(native.last().unwrap().1, b"false");
        std::fs::remove_dir_all(root).unwrap();
    }
}
