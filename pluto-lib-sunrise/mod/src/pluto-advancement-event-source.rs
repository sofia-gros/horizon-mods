// Transport for queued custom advancement events: the submitting library
// installs a source, and the advancement tick takes the queued events from
// it (see advancement-event-drain). No actor or progress state lives here.
use crate::pluto_inventory::advancements::events::EventMatcher;
use std::sync::OnceLock;

/// One queued custom advancement event, owned for transport from the
/// submitting library to the native sink. The matcher travels with the
/// event; neither the trigger nor the payload is interpreted here.
pub struct QueuedEvent {
    pub trigger: String,
    pub payload: serde_json::Value,
    pub matches: EventMatcher,
}

static EVENT_SOURCE: OnceLock<fn() -> Vec<QueuedEvent>> = OnceLock::new();

/// Install the process-lifetime source of queued custom advancement events.
/// The source is called once per advancement tick and returns every event
/// queued since the previous call, oldest first. Installing this native
/// package alone submits nothing: without a source the queue stays empty.
pub fn set_event_source(source: fn() -> Vec<QueuedEvent>) -> Result<(), String> {
    EVENT_SOURCE
        .set(source)
        .map_err(|_| "advancement event source already installed".into())
}

/// Take every event the installed source has queued for this tick.
pub(crate) fn take_events() -> Vec<QueuedEvent> {
    EVENT_SOURCE.get().map_or_else(Vec::new, |source| source())
}
