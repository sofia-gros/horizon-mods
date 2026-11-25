// Records the level's world border for the highlight transport: the game keeps the border of the level in place — on a remote client, the one its server syncs to it — and every tick runs with it in place, while the host's border read states a border only on a server, and only for the three vanilla dimensions.
fn pluto_highlight_border_head(game: &mut crate::game::Game<'_>) {
    crate::entry::pluto_highlight_border::set_border(game.border.bounds());
}
