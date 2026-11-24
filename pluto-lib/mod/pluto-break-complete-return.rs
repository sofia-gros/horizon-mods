// The break-completion seam (BLOCKITEM-3), on the funnel every player break returns through:
// player_break_block wraps the game's own break, so its return carries the break's result
// unchanged, with the break's actor state put back the way the break itself left it — the
// completed break is reported and the queued swings and broadcasts play inside that state,
// exactly as they did at the return of the break itself.
fn pluto_break_complete(value: Result<bool>, game: &mut crate::game::Game<'_>, pos: [i32; 3]) -> Result<bool> {
    // The break went through: its AFTER runs now, inside the break, while the breaking player is still
    // the acting one. A break that did not remove the block hears nothing here; its pending break, if
    // any, waits for the next tick as before.
    let completed = matches!(value, Ok(true));
    let outer = game.lan_break_pos.replace(pos);
    game.with_player_actor(|game| {
        if completed {
            crate::game::pluto_break_complete::dispatch(game, pos);
        }
        // Whatever the break did, the arm swings and break broadcasts the mods queued for the acting player
        // play now: they belong to this turn, not to a later tick.
        crate::game::pluto_player_animate_flush::flush(game);
    });
    game.lan_break_pos = outer;
    value
}
