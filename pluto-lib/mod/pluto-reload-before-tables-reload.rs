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

// A reload: a remote client builds no server tables, and its body
// returns before building any; keep that shape.
fn pluto_reload_before_tables_reload(game: &mut Game<'_>) {
    if game.remote.is_some() {
        return;
    }
    game.native_reload_listeners();
}
