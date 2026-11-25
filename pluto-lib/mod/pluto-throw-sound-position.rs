fn pluto_throw_sound_position(value: Option<[f64; 3]>, stack: &crate::items::ItemStack, game: &crate::game::Game<'_>) -> Option<[f64; 3]> {
    crate::game::creatures::pluto_throw_sound::throw_position(value, stack, game)
}
