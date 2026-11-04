fn tconstruct_slimy_loot_native(
    _value: (),
    game: &crate::game::Game<'_>,
    files: &mut Vec<(String, Vec<u8>)>,
) {
    const KEY: &str = "loot_table/!tconstruct:slimy_loot_config";
    files.retain(|(path, _)| path != KEY);
    let dedicated = game.standalone_server();
    let cwd = if dedicated {
        std::env::current_dir().ok()
    } else {
        Some(std::path::PathBuf::new())
    };
    let config = cwd
        .and_then(|cwd| tconstruct_source_config::root_for_world(&game.world_root, dedicated, &cwd))
        .and_then(|root| tconstruct_source_config::load(&root).ok())
        .and_then(|config| config.get("slimy_loot_chests").copied())
        == Some(true);
    let unsafe_native = files.iter().any(|(path, _)| {
        path.contains("loot_injectors")
            || [
                "loot_table/minecraft:chests/simple_dungeon",
                "loot_table/minecraft:chests/nether_bridge",
                "loot_table/minecraft:chests/bastion_bridge",
                "loot_table/minecraft:chests/end_city_treasure",
            ]
            .contains(&path.as_str())
    });
    files.push((
        KEY.into(),
        if config && !unsafe_native {
            b"true".to_vec()
        } else {
            b"false".to_vec()
        },
    ));
}
