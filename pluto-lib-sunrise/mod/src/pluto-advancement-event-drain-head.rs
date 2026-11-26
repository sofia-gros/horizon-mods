// Queued custom advancement events are handed to the native sink at the
// head of the advancement tick, in the scope of the current actor. Owner
// tag snapshots are driven first, so this tick's evaluations resolve tags
// against the snapshots that were current when the tick began.
fn pluto_advancement_event_drain(game: &mut Game<'_>) {
    crate::game::pluto_advancement_tags::drive(game);
    let events = crate::game::pluto_advancement_event_source::take_events();
    if events.is_empty() {
        return;
    }
    crate::game::pluto_advancement_events::with_game(game, |sink| {
        use crate::pluto_inventory::advancements::events::Event;
        for event in &events {
            let _ = sink.submit(Event {
                trigger: &event.trigger,
                payload: &event.payload,
                matches: event.matches,
            });
        }
    });
}
