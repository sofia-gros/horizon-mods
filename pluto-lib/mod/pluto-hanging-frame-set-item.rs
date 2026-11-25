// HEAD on Game::set_frame_item: the command path's setItem(stack, true)
// resets a rotating variant's rotation first, as every setItem does (the
// body never touches the rotation, so head and return agree).
fn pluto_hanging_frame_set_item(game: &mut Game<'_>, i: usize, stack: &Option<ItemStack>) {
    let _ = stack;
    crate::game::pluto_hanging_game::fancy_frame_set_item(game, i);
}
