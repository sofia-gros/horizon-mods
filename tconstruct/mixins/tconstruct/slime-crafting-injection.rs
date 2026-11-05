fn tconstruct_slime_crafting(value: (), game: &crate::game::Game<'_>, files: &mut Vec<(String, Vec<u8>)>) -> () {
    if game.remote.is_some() || game.world_root.as_os_str().is_empty() { return value; }
    let dedicated = game.standalone_server();
    let working_dir = if dedicated {
        let Ok(path) = std::env::current_dir() else { return value; };
        path
    } else { std::path::PathBuf::new() };
    let Some(root) = tconstruct_source_config::root_for_world(&game.world_root, dedicated, &working_dir) else { return value; };
    crate::game::tconstruct_slime_crafting::contribute(&root, files);
    value
}
