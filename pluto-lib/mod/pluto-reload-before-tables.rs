// The host builds a data load's content tables before it applies the
// mods' JSON reload listeners: finish_open_world and reload_world_tables
// both call build_world_tables (which parses the recipes) first and
// native_reload_listeners second. A recipe ingredient of a mod's own
// kind is answered at parse time from the mod's loaded data, so it came
// back empty and the host dropped every recipe using one. These heads
// apply the listeners first; the host's own application later in each
// flow is unchanged, and listener application replaces the loaded state
// wholesale, so the second application only refreshes the registry
// snapshots with the tables the load just built.

// World open: the listeners read the opening world's root (their pack
// stack is that world's datapacks), which the body assigns from the
// same prepared world a few lines later; assign it first so the early
// application reads this world's packs, not the previous one's.
fn pluto_reload_before_tables_open(game: &mut Game<'_>, prepared: &world_loading::PreparedWorld) {
    game.world_root = prepared.save_root.clone();
    game.native_reload_listeners();
}
