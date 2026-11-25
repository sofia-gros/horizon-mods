// Adapted from pinned MIT Horizon natural_spawning.rs::spawn_natural.
// License: pluto-lib/LICENSE.horizon. The game's own pass runs first, untouched.
fn pluto_spawn_placement_permit(game: &mut Game<'_>, value: ()) {
    let _ = value;
    let pending = crate::game::pluto_spawn_placement_game::permit_add_spawns(game);
    if !pending.is_empty() {
        game.add_spawned(pending, false);
    }
}
