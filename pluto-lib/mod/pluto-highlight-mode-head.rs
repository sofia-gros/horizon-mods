// Records the local player's new game mode for the highlight transport: the game's mode setter runs on every path that sets the local mode (a world opening, /gamemode, a LAN mode), and no host read states the mode on the render path.
fn pluto_highlight_mode_head(mode: horizon_client::command::args::GameMode) {
    crate::entry::pluto_highlight_mode::set_mode(match mode {
        horizon_client::command::args::GameMode::Survival => 0,
        horizon_client::command::args::GameMode::Creative => 1,
        horizon_client::command::args::GameMode::Adventure => 2,
        horizon_client::command::args::GameMode::Spectator => 3,
    });
}
