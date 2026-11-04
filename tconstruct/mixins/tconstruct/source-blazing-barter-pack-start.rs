fn tconstruct_blazing_barter_pack_start(
    files: Vec<(String, Vec<Vec<u8>>)>,
    world_root: &std::path::Path,
    resources: &std::path::Path,
    vanilla_disabled: bool,
    enabled_features: &[String],
) -> Vec<(String, Vec<Vec<u8>>)> {
    use self::tconstruct_source_blazing_barter_pack as proof;
    let stack = pack_stack(world_root, resources, vanilla_disabled, enabled_features);
    let mut highest = None;
    let read_pack = |highest: &mut Option<Option<Vec<u8>>>, pack: &DataPack| {
        let listed = pack
            .list("data/tconstruct/mantle/loot_injectors/")
            .iter()
            .any(|path| path == proof::INJECTOR_PATH);
        proof::observe(highest, listed, pack.read(proof::INJECTOR_PATH));
    };
    if let Some(builtin) = &stack.builtin {
        read_pack(&mut highest, builtin);
    }
    let mut custom_table = false;
    for pack in stack.enabled_packs() {
        read_pack(&mut highest, pack);
        // Forge does not fire LootTableLoadEvent for a user pack's table,
        // even when its bytes are identical to vanilla. Observe before JSON
        // parsing so a malformed replacement cannot expose the fallback.
        custom_table |= pack
            .list("data/minecraft/loot_table/gameplay/")
            .iter()
            .any(|path| path == proof::TABLE_PATH)
            || pack.read(proof::TABLE_PATH).is_some();
    }
    proof::stamp(
        files,
        !vanilla_disabled && !custom_table && proof::enabled(highest),
    )
}
