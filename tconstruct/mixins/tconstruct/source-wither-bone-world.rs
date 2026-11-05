fn tconstruct_wither_bone_world(
    _value: (),
    overlay: &PackOverlay<'_>,
    kind: &str,
    out: &mut std::collections::HashMap<String, serde_json::Value>,
) {
    if kind != "loot_table" {
        return;
    }
    use self::tconstruct_source_wither_bone as proof;
    out.remove(proof::CONFIG.strip_prefix("loot_table/").unwrap());
    out.remove(proof::PACK.strip_prefix("loot_table/").unwrap());
    out.remove(proof::MARKER);
    if out.contains_key(proof::TABLE) && proof::enabled(overlay.files) {
        // Invalid resource location, held only as per-loaded-table state.
        out.insert(proof::MARKER.into(), serde_json::json!({"pools":[]}));
    }
}
