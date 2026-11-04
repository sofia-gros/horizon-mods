fn tconstruct_blazing_barter_pack_loot(
    _value: (),
    overlay: &PackOverlay<'_>,
    kind: &str,
    out: &mut std::collections::HashMap<String, serde_json::Value>,
) {
    if kind != "loot_table" {
        return;
    }
    use self::tconstruct_source_blazing_barter_pack as proof;
    // shared_tables hashes the overlay before reaching this point, so changing
    // the gate also changes its cache key. Never publish the internal table.
    out.remove(proof::SENTINEL_ID);
    out.remove(proof::NATIVE_SENTINEL_ID);
    if proof::gate(overlay.files) {
        let id = "minecraft:gameplay/piglin_bartering";
        if let Some(table) = out.get_mut(id) {
            self::tconstruct_source_blazing_barter::inject(id, table);
        }
    }
}
