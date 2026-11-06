//! Opt-in, borrowed observations of clientbound play custom payloads.
//!
//! This is not request/reply transport. Bytes are untrusted remote input; no
//! sender identity, session ownership, permission, send or cancel capability is
//! implied. The native adapter defines which packets reach this phase.
use std::collections::BTreeMap;
use std::sync::RwLock;

/// A library observation ceiling, not the protocol's accepted packet limit.
/// Larger packets still follow the unchanged native path, without notification.
pub const MAX_BYTES: usize = 262_144;
pub const MAX_ID_BYTES: usize = 256;

#[derive(Clone, Copy, Debug)]
pub struct Payload<'a> {
    pub channel: &'a str,
    pub bytes: &'a [u8],
}
pub type Callback = for<'a> fn(Payload<'a>);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistrationError {
    InvalidId,
    InvalidChannel,
    InvalidLimit,
    DuplicateId,
    Poisoned,
}
#[derive(Clone)]
struct Subscription {
    channel: String,
    max_bytes: usize,
    callback: Callback,
}
#[derive(Default)]
struct Registry(RwLock<BTreeMap<String, Subscription>>);

pub(crate) fn valid_id(id: &str) -> bool {
    if id.len() > MAX_ID_BYTES {
        return false;
    }
    let Some((namespace, path)) = id.split_once(':') else {
        return false;
    };
    !namespace.is_empty()
        && !path.is_empty()
        && namespace
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.-".contains(&c))
        && path
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c))
        && !path
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
}
impl Registry {
    const fn new() -> Self {
        Self(RwLock::new(BTreeMap::new()))
    }
    fn register(
        &self,
        id: &str,
        channel: &str,
        max_bytes: usize,
        callback: Callback,
    ) -> Result<(), RegistrationError> {
        if !valid_id(id) {
            return Err(RegistrationError::InvalidId);
        }
        if !valid_id(channel) {
            return Err(RegistrationError::InvalidChannel);
        }
        if max_bytes > MAX_BYTES {
            return Err(RegistrationError::InvalidLimit);
        }
        let mut entries = self.0.write().map_err(|_| RegistrationError::Poisoned)?;
        if entries.contains_key(id) {
            return Err(RegistrationError::DuplicateId);
        }
        entries.insert(
            id.to_owned(),
            Subscription {
                channel: channel.to_owned(),
                max_bytes,
                callback,
            },
        );
        Ok(())
    }
    fn remove(&self, id: &str) -> Result<bool, RegistrationError> {
        Ok(self
            .0
            .write()
            .map_err(|_| RegistrationError::Poisoned)?
            .remove(id)
            .is_some())
    }
    fn enabled(&self) -> bool {
        self.0.read().is_ok_and(|entries| !entries.is_empty())
    }
    fn observe(&self, payload: Payload<'_>) {
        if payload.bytes.len() > MAX_BYTES {
            return;
        }
        let callbacks = match self.0.read() {
            Ok(entries) => entries
                .values()
                .filter(|entry| {
                    entry.channel == payload.channel && payload.bytes.len() <= entry.max_bytes
                })
                .map(|entry| entry.callback)
                .collect::<Vec<_>>(),
            Err(_) => return,
        };
        // No payload or channel is copied. Empty registries/matches allocate no
        // callback storage. Release the registry lock before invoking consumers.
        // IDs determine lexical order; registration changes affect the next call.
        for callback in callbacks {
            callback(payload);
        }
    }
}
static SUBSCRIPTIONS: Registry = Registry::new();

/// Read-only event entry for the existing clientbound play-payload observer.
///
/// This adds no registry or native hook. The installed adapter observes after
/// native mod observation; calling [`observe`] directly does not prove that phase.
/// Registration grants no send, cancel, reply, screen or identity capability.
pub const OBSERVED: ObservedEvent = ObservedEvent { _private: () };

/// A zero-sized entry point. Every copy uses the same existing subscriptions.
#[derive(Clone, Copy, Debug)]
pub struct ObservedEvent {
    _private: (),
}

/// Borrowed registration settings, not ownership of a channel or a subscription.
/// The same ID/channel/limit validation runs only when `register` is called.
#[derive(Clone, Copy, Debug)]
pub struct ChannelObservation<'a> {
    channel: &'a str,
    max_bytes: usize,
}

impl ObservedEvent {
    /// Select an exact channel and an explicit byte ceiling; zero means empty
    /// payloads only. No allocation, validation or registration happens here.
    pub const fn channel(self, channel: &str, max_bytes: usize) -> ChannelObservation<'_> {
        ChannelObservation { channel, max_bytes }
    }

    /// Explicitly remove a subscriber, including one created by the old API.
    /// The ID is a shared registry key, not a private ownership token.
    pub fn remove(self, id: &str) -> Result<bool, RegistrationError> {
        remove(id)
    }
}

impl ChannelObservation<'_> {
    /// Register a non-capturing synchronous callback during the consumer's
    /// explicit startup. Dropping this settings value does not unregister it.
    /// Duplicate IDs never replace entries, even across old and event APIs.
    pub fn register(self, id: &str, callback: Callback) -> Result<(), RegistrationError> {
        register(id, self.channel, self.max_bytes, callback)
    }
}

/// Subscribe during setup to one exact channel and a consumer-selected byte cap.
/// A zero cap observes only empty payloads. IDs are unique; duplicate registration
/// never replaces a subscriber. A channel is a selector, not ownership of it.
pub fn register(
    id: &str,
    channel: &str,
    max_bytes: usize,
    callback: Callback,
) -> Result<(), RegistrationError> {
    SUBSCRIPTIONS.register(id, channel, max_bytes, callback)
}
pub fn remove(id: &str) -> Result<bool, RegistrationError> {
    SUBSCRIPTIONS.remove(id)
}
pub fn enabled() -> bool {
    SUBSCRIPTIONS.enabled()
}
/// Adapter entry. Call synchronously only after the native client packet observer.
/// The API itself neither authenticates these bytes nor proves that phase.
pub fn observe(payload: Payload<'_>) {
    SUBSCRIPTIONS.observe(payload);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    };
    static SERIAL: Mutex<()> = Mutex::new(());
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    static POINTER: AtomicUsize = AtomicUsize::new(0);
    fn noop(_: Payload<'_>) {}
    fn first(payload: Payload<'_>) {
        assert_eq!(CALLS.fetch_add(1, Ordering::SeqCst), 0);
        assert_eq!(payload.channel, "example:guide");
        assert_eq!(payload.bytes, &[0, 255, 128]);
        assert_eq!(
            payload.bytes.as_ptr() as usize,
            POINTER.load(Ordering::SeqCst)
        );
    }
    fn second(_: Payload<'_>) {
        assert_eq!(CALLS.fetch_add(1, Ordering::SeqCst), 1);
    }
    fn count(_: Payload<'_>) {
        CALLS.fetch_add(1, Ordering::SeqCst);
    }
    #[test]
    fn validates_registration_without_replacement() {
        let registry = Registry::new();
        for id in [
            "",
            "example",
            ":path",
            "Bad:path",
            "example:../path",
            "example:a//b",
        ] {
            assert_eq!(
                registry.register(id, "example:guide", 0, noop),
                Err(RegistrationError::InvalidId)
            );
        }
        assert_eq!(
            registry.register("example:ok", "bad", 0, noop),
            Err(RegistrationError::InvalidChannel)
        );
        assert_eq!(
            registry.register("example:ok", "example:guide", MAX_BYTES + 1, noop),
            Err(RegistrationError::InvalidLimit)
        );
        assert_eq!(
            registry.register(
                &format!("example:{}", "a".repeat(MAX_ID_BYTES)),
                "example:guide",
                0,
                noop
            ),
            Err(RegistrationError::InvalidId)
        );
        registry
            .register("example:ok", "example:guide", 0, noop)
            .unwrap();
        assert_eq!(
            registry.register("example:ok", "other:channel", 1, count),
            Err(RegistrationError::DuplicateId)
        );
        assert_eq!(
            registry.0.read().unwrap()["example:ok"].channel,
            "example:guide"
        );
    }
    #[test]
    fn exact_channel_limits_and_unmodified_borrow_in_lexical_order() {
        let _lock = SERIAL.lock().unwrap();
        CALLS.store(0, Ordering::SeqCst);
        let bytes = [0, 255, 128];
        POINTER.store(bytes.as_ptr() as usize, Ordering::SeqCst);
        let registry = Registry::new();
        registry
            .register("example:b", "example:guide", 3, second)
            .unwrap();
        registry
            .register("example:a", "example:guide", 3, first)
            .unwrap();
        registry
            .register("example:c", "example:guide", 2, count)
            .unwrap();
        registry
            .register("example:d", "example:other", 3, count)
            .unwrap();
        registry.observe(Payload {
            channel: "example:guide",
            bytes: &bytes,
        });
        assert_eq!(CALLS.load(Ordering::SeqCst), 2);
    }
    #[test]
    fn empty_registry_zero_cap_and_hard_ceiling() {
        let _lock = SERIAL.lock().unwrap();
        CALLS.store(0, Ordering::SeqCst);
        let registry = Registry::new();
        assert!(!registry.enabled());
        registry.observe(Payload {
            channel: "example:guide",
            bytes: &[],
        });
        registry
            .register("example:zero", "example:guide", 0, count)
            .unwrap();
        registry.observe(Payload {
            channel: "example:guide",
            bytes: &[1],
        });
        registry.observe(Payload {
            channel: "example:guide",
            bytes: &[],
        });
        assert_eq!(CALLS.load(Ordering::SeqCst), 1);
        registry.remove("example:zero").unwrap();
        registry
            .register("example:bounded", "example:guide", MAX_BYTES, count)
            .unwrap();
        let bytes = vec![0; MAX_BYTES + 1];
        registry.observe(Payload {
            channel: "example:guide",
            bytes: &bytes,
        });
        assert_eq!(CALLS.load(Ordering::SeqCst), 1);
        registry.observe(Payload {
            channel: "example:guide",
            bytes: &bytes[..MAX_BYTES],
        });
        assert_eq!(CALLS.load(Ordering::SeqCst), 2);
    }
    fn remove_self(_: Payload<'_>) {
        CALLS.fetch_add(1, Ordering::SeqCst);
        assert!(remove("test:remove_self").unwrap());
    }
    #[test]
    fn callbacks_run_unlocked_and_changes_affect_later_dispatches() {
        let _lock = SERIAL.lock().unwrap();
        CALLS.store(0, Ordering::SeqCst);
        register("test:remove_self", "example:guide", 0, remove_self).unwrap();
        observe(Payload {
            channel: "example:guide",
            bytes: &[],
        });
        observe(Payload {
            channel: "example:guide",
            bytes: &[],
        });
        assert_eq!(CALLS.load(Ordering::SeqCst), 1);
        assert!(!remove("test:remove_self").unwrap());
    }
    #[test]
    fn poisoned_registry_is_not_observed_or_silently_recovered() {
        let registry = Registry::new();
        let _ = std::panic::catch_unwind(|| {
            let _lock = registry.0.write().unwrap();
            panic!("poison");
        });
        assert!(!registry.enabled());
        registry.observe(Payload {
            channel: "example:guide",
            bytes: &[],
        });
        assert_eq!(
            registry.register("example:id", "example:guide", 0, noop),
            Err(RegistrationError::Poisoned)
        );
    }

    #[test]
    fn event_entry_uses_old_registry_and_explicit_removal() {
        let _lock = SERIAL.lock().unwrap();
        CALLS.store(0, Ordering::SeqCst);
        let channel = String::from("example:guide");
        OBSERVED.channel(&channel, 0).register("test:event", count).unwrap();
        drop(channel); // Registration owns its selectors; the builder only borrowed them.
        assert_eq!(register("test:event", "example:other", 1, noop), Err(RegistrationError::DuplicateId));
        observe(Payload { channel: "example:guide", bytes: &[1] });
        observe(Payload { channel: "example:guide", bytes: &[] });
        assert_eq!(CALLS.load(Ordering::SeqCst), 1);
        assert!(remove("test:event").unwrap());
        register("test:legacy", "example:guide", 0, count).unwrap();
        assert_eq!(OBSERVED.channel("example:other", 1).register("test:legacy", noop), Err(RegistrationError::DuplicateId));
        assert!(OBSERVED.remove("test:legacy").unwrap());
        assert!(!OBSERVED.remove("test:legacy").unwrap());
    }

    #[test]
    fn event_settings_preserve_registration_error_order_and_bounds() {
        assert_eq!(std::mem::size_of_val(&OBSERVED), 0);
        let invalid = OBSERVED.channel("invalid", MAX_BYTES + 1);
        assert_eq!(invalid.register("invalid", noop), Err(RegistrationError::InvalidId));
        assert_eq!(invalid.register("example:valid", noop), Err(RegistrationError::InvalidChannel));
        assert_eq!(OBSERVED.channel("example:guide", MAX_BYTES + 1).register("example:valid", noop), Err(RegistrationError::InvalidLimit));
    }

    #[test]
    fn event_and_legacy_subscribers_share_order_and_borrowed_bytes() {
        let _lock = SERIAL.lock().unwrap();
        CALLS.store(0, Ordering::SeqCst);
        let bytes = [0, 255, 128];
        POINTER.store(bytes.as_ptr() as usize, Ordering::SeqCst);
        register("test:event_b", "example:guide", 3, second).unwrap();
        OBSERVED.channel("example:guide", 3).register("test:event_a", first).unwrap();
        observe(Payload { channel: "example:guide", bytes: &bytes });
        assert_eq!(CALLS.load(Ordering::SeqCst), 2);
        OBSERVED.remove("test:event_a").unwrap();
        OBSERVED.remove("test:event_b").unwrap();
    }

    fn install_late(_: Payload<'_>) {
        CALLS.fetch_add(1, Ordering::SeqCst);
        OBSERVED.remove("test:event_install").unwrap();
        OBSERVED.channel("example:guide", 0).register("test:event_late", count).unwrap();
    }

    #[test]
    fn event_registration_during_callback_changes_only_next_snapshot() {
        let _lock = SERIAL.lock().unwrap();
        CALLS.store(0, Ordering::SeqCst);
        OBSERVED.channel("example:guide", 0).register("test:event_install", install_late).unwrap();
        observe(Payload { channel: "example:guide", bytes: &[] });
        assert_eq!(CALLS.load(Ordering::SeqCst), 1);
        observe(Payload { channel: "example:guide", bytes: &[] });
        assert_eq!(CALLS.load(Ordering::SeqCst), 2);
        OBSERVED.remove("test:event_late").unwrap();
    }
}
