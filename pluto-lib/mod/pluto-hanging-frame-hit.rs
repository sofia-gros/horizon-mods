// RETURN on Game::hit_frame: when a filled fancy frame's item was taken
// out, the original's setItem(EMPTY) has reset a rotating variant's
// rotation; a frame that broke outright is already gone.
fn pluto_hanging_frame_hit(value: (), game: &mut Game<'_>, i: usize, by_player: bool, creative: bool) {
    let _ = (value, by_player, creative);
    crate::game::pluto_hanging_game::fancy_frame_hit(game, i);
}

// HEAD on Game::set_frame_item: the command path's setItem(stack, true)
// resets a rotating variant's rotation first, as every setItem does (the
// body never touches the rotation, so head and return agree).
fn pluto_hanging_frame_set_item(game: &mut Game<'_>, i: usize, stack: &Option<ItemStack>) {
    let _ = stack;
    crate::game::pluto_hanging_game::fancy_frame_set_item(game, i);
}
