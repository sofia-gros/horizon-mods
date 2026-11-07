//! Synchronous custom advancement-event submission, without host types.
//!
//! The caller establishes that the real action succeeded and supplies its policy.
//! The sink owns the current authoritative actor and native player conditions,
//! progress, rewards and notifications. Nothing is queued or globally cached.

/// Consumer-owned policy for event-specific conditions. The sink separately
/// evaluates the host's standard player conditions for the current actor.
pub type EventMatcher = fn(conditions: &serde_json::Value, payload: &serde_json::Value) -> bool;

/// Borrowed event facts valid only during this call. The library does not
/// interpret the trigger identifier, payload keys or event-specific conditions.
pub struct Event<'a> {
    pub trigger: &'a str,
    pub payload: &'a serde_json::Value,
    pub matches: EventMatcher,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Submission {
    /// Submitted to the native pipeline. This does not imply any criterion was
    /// matched, awarded, saved, sent, or displayed; the host owns those outcomes.
    Submitted,
    /// A remote client cannot award authoritative progress locally.
    RemoteClient,
    /// No authoritative actor is currently in scope (for example, an idle
    /// dedicated server). Do not substitute the host or remember a past guest.
    NoActor,
}

/// A synchronous capability supplied by the native adapter. For a local player
/// it uses that player; inside the host's guest-acting scope it uses that guest.
/// Submit only after the action's successful authoritative boundary. Do not
/// retain the sink for deferred work or treat submission as an award receipt.
pub trait EventSink {
    fn submit(&mut self, event: Event<'_>) -> Submission;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn borrowed_event_policy_and_submission_are_host_independent() {
        struct Sink(Vec<(String, serde_json::Value)>);
        impl EventSink for Sink {
            fn submit(&mut self, event: Event<'_>) -> Submission {
                // Consumer policy can decline every criterion while submission
                // still succeeds. No player/progress model is invented here.
                assert!(!(event.matches)(&serde_json::Value::Null, event.payload));
                self.0
                    .push((event.trigger.to_owned(), event.payload.clone()));
                Submission::Submitted
            }
        }
        let trigger = String::from("example:opened");
        let payload = serde_json::json!({"uninterpreted": 7});
        let mut sink = Sink(Vec::new());
        let capability: &mut dyn EventSink = &mut sink;
        assert_eq!(
            capability.submit(Event {
                trigger: &trigger,
                payload: &payload,
                matches: |_, _| false,
            }),
            Submission::Submitted
        );
        assert_eq!(sink.0, [(trigger, payload)]);
    }
}
