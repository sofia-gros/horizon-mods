fn tconstruct_wither_bone_native(
    _value: (),
    game: &crate::game::Game<'_>,
    files: &mut Vec<(String, Vec<u8>)>,
) {
    const KEY: &str = "loot_table/!tconstruct:wither_bone_config";
    files.retain(|(p, _)| p != KEY);
    let dedicated = game.standalone_server();
    let cwd = if dedicated {
        std::env::current_dir().ok()
    } else {
        Some(std::path::PathBuf::new())
    };
    let enabled = cwd
        .and_then(|cwd| tconstruct_source_config::root_for_world(&game.world_root, dedicated, &cwd))
        .and_then(|root| tconstruct_source_config::load(&root).ok())
        .and_then(|config| config.get("wither_bone_drop").copied())
        == Some(true)
        && !files.iter().any(|(p, _)| {
            p.contains("loot_modifiers") || p == "loot_table/minecraft:entities/wither_skeleton"
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
