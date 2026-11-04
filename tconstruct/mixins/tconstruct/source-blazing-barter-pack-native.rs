fn tconstruct_blazing_barter_pack_native(_value: (), files: &mut Vec<(String, Vec<u8>)>) {
    // Run before overlay_files discards malformed JSON. Native injector
    // definitions and barter-table replacements close the gate, even malformed.
    const KEY: &str = "loot_table/!tconstruct:blazing_barter_native_state";
    files.retain(|(path, _)| path != KEY);
    let enabled = !files.iter().any(|(path, _)| {
        path.contains("loot_injectors") || path == "loot_table/minecraft:gameplay/piglin_bartering"
    });
    files.push((
        KEY.into(),
        if enabled {
            b"true".to_vec()
        } else {
            b"false".to_vec()
        },
    ));
}
