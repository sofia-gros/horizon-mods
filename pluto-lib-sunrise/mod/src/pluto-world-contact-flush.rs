fn pluto_world_contact_flush(game: &mut crate::game::Game<'_>) {
    crate::game::potions::pluto_contact_sound::flush_world(game);
    crate::game::native_explosions::pluto_explosion_flush::flush(game);
}
