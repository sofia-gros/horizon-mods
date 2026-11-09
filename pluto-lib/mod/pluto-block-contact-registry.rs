fn contact_registry(game: &crate::game::Game<'_>) {
    crate::entry::pluto_block::set_contact_registry(game.pack.assets.registry);
}
