fn tconstruct_slimy_loot_pack(
    mut files: Vec<(String, Vec<Vec<u8>>)>,
    world_root: &std::path::Path,
    resources: &std::path::Path,
    vanilla_disabled: bool,
    enabled_features: &[String],
) -> Vec<(String, Vec<Vec<u8>>)> {
    use self::tconstruct_source_slimy_loot as proof;
    let stack = pack_stack(world_root, resources, vanilla_disabled, enabled_features);
    for (_, name, _, expected) in proof::SPECS {
        let key = format!("{}{name}", proof::PREFIX);
        files.retain(|(path, _)| path != &key);
        let path = format!("data/tconstruct/mantle/loot_injectors/slimy_foliage_{name}.json");
        let mut top: Option<Option<Vec<u8>>> = None;
        let inspect = |top: &mut Option<Option<Vec<u8>>>, pack: &DataPack| {
            let listed = pack
                .list("data/tconstruct/mantle/loot_injectors/")
                .iter()
                .any(|f| f == &path);
            let data = pack.read(&path);
            if listed || data.is_some() {
                *top = Some(data);
            }
        };
        if let Some(builtin) = &stack.builtin {
            inspect(&mut top, builtin);
        }
        for pack in stack.enabled_packs() {
            inspect(&mut top, pack);
        }
        let enabled = !vanilla_disabled
            && top
                .flatten()
                .is_some_and(|data| proof::canonical(&data, expected));
        files.push((
            key,
            vec![if enabled {
                b"true".to_vec()
            } else {
                b"false".to_vec()
            }],
        ));
    }
    files.retain(|(p, _)| p != proof::CONFIG_KEY);
    files
}
