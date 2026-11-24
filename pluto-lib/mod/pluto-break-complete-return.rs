fn pluto_break_complete(value: Result<bool>, game: &mut crate::game::Game<'_>, pos: [i32; 3]) -> Result<bool> {
    // The break went through: its AFTER runs now, inside the break, while the breaking player is still
    // the acting one. A break that did not remove the block hears nothing here; its pending break, if
    // any, waits for the next tick as before.
    if matches!(value, Ok(true)) {
        crate::game::pluto_break_complete::dispatch(game, pos);
    }
    // Whatever the break did, the arm swings and break broadcasts the mods queued for the acting player
    // play now: they belong to this turn, not to a later tick.
    crate::game::pluto_player_animate_flush::flush(game);
    value
}
