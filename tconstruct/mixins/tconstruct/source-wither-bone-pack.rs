fn tconstruct_wither_bone_pack(
    mut files: Vec<(String, Vec<Vec<u8>>)>,
    world_root: &std::path::Path,
    resources: &std::path::Path,
    vanilla_disabled: bool,
    enabled_features: &[String],
) -> Vec<(String, Vec<Vec<u8>>)> {
    use self::tconstruct_source_wither_bone as proof;
    let stack = pack_stack(world_root, resources, vanilla_disabled, enabled_features);
    let source = stack
        .builtin
        .as_ref()
        .and_then(|pack| pack.read(proof::SOURCE));
    // Unknown global modifier lists/definitions or table replacements are not
    // approximated. Detect even listed-but-unreadable files before JSON parsing.
    let mut overridden = stack.builtin.as_ref().is_some_and(|pack| {
        pack.list("data/")
            .iter()
            .any(|p| p.contains("/loot_modifiers/") && p != proof::SOURCE)
    });
    for pack in stack.enabled_packs() {
        overridden |= pack
            .list("data/")
            .iter()
            .any(|p| p.contains("/loot_modifiers/") || p == proof::TABLE_PATH)
            || pack.read(proof::SOURCE).is_some()
            || pack.read(proof::TABLE_PATH).is_some()
            || pack
                .read("data/forge/loot_modifiers/global_loot_modifiers.json")
                .is_some()
            || pack
                .read("data/neoforge/loot_modifiers/global_loot_modifiers.json")
                .is_some();
    }
    let enabled =
        !vanilla_disabled && !overridden && source.is_some_and(|bytes| proof::canonical(&bytes));
    files.retain(|(p, _)| p != proof::PACK && p != proof::CONFIG);
    files.push((
        proof::PACK.into(),
        vec![if enabled {
            b"true".to_vec()
        } else {
            b"false".to_vec()
        }],
    ));
    files
}
