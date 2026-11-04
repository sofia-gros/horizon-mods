fn tconstruct_slimy_loot_world(
    _value: (),
    overlay: &PackOverlay<'_>,
    kind: &str,
    out: &mut std::collections::HashMap<String, serde_json::Value>,
) {
    if kind != "loot_table" {
        return;
    }
    use self::tconstruct_source_slimy_loot as proof;
    out.remove(proof::CONFIG_KEY.strip_prefix("loot_table/").unwrap());
    for (id, name, _, _) in proof::SPECS {
        out.remove(&format!("!tconstruct:slimy_loot_{name}"));
        if let Some(table) = out.get_mut(id) {
            proof::inject(id, table, overlay.files);
        }
    }
}
