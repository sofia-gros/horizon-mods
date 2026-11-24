// The hand's use is entered: a RETURN on it cannot plan (the body works through
// closures and `?`), so the seam is a cancellable HEAD that runs the use itself
// under the module's guard and reports the outcome the use leaves.
fn pluto_sleeves_takeover_return(game: &mut Game<'_>, block_hit: Option<&Hit>) -> std::ops::ControlFlow<Result<bool>> {
    crate::game::use_hand::pluto_sleeves_takeover::use_hand(game, block_hit)
}
