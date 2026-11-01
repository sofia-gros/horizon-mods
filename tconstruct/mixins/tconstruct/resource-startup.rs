fn tconstruct_startup_resources(root: &mut std::path::PathBuf) {
    if root.join("tconstruct-original-pack").is_dir() { return; }
    if !root.join("assets/tconstruct/source-packs/tconstruct-original-pack").is_dir() { return; }
    if let Err(error) = horizon_client::logging::tconstruct_resource_tables::startup(root) {
        log::error!("TConstruct resource startup failed: {error}");
    }
}
