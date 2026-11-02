fn tconstruct_startup_resources(root: &mut std::path::PathBuf) {
    if root.join("tconstruct-original-pack").is_dir() { return; }
    if !root.join("assets/tconstruct/source-packs/tconstruct-original-pack").is_dir() {
        eprintln!("TConstruct resources are missing; enable the TConstruct content mod before opening a world.");
        log::error!("TConstruct content or prepared resource pack is missing; stopping startup");
        std::process::exit(1);
    }
    if let Err(error) = horizon_client::entry::tconstruct_resource_tables::startup(root) {
        eprintln!("TConstruct resource preparation failed; the game cannot safely open a world: {error}");
        log::error!("TConstruct resource preparation failed; stopping startup: {error}");
        std::process::exit(1);
    }
}
