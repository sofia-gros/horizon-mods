// The break's sampled experience is replaced by what the block experience
// provider answers (pluto_block_exp_access): the game's own silk-touch rule
// applies to the answer, as the original's break event runs before it too.
fn pluto_block_exp(value: u32, game: &mut Game<'_>, pos: [i32; 3]) -> u32 {
    super::pluto_block_exp_access::adjust(game, pos, value)
}
