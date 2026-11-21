// Queued custom advancement events are handed to the native sink at the
// head of the advancement tick, in the scope of the current actor.
fn pluto_advancement_event_drain(game: &mut Game<'_>) {
    let events = super::pluto_advancement_event_source::take_events();
    if events.is_empty() {
        return;
    }
    super::pluto_advancement_events::with_game(game, |sink| {
        use horizon_client::pluto_inventory::advancements::events::Event;
        for event in &events {
            let _ = sink.submit(Event {
                trigger: &event.trigger,
                payload: &event.payload,
                matches: event.matches,
            });
        }
    });
}
