fn tconstruct_trace_station_send(game: &crate::game::Game<'_>, container: i32, clicks: &[Predicted]) {
    if crate::game::tconstruct_ui::kind(game) != Some(true) || std::env::var_os("TCONSTRUCT_STATION_TRACE").is_none() { return; }
    for (slot, button, input, _, _) in clicks {
        eprintln!("station trace send window={container} slot={slot} button={button} input={}", input_ordinal(*input));
    }
}
