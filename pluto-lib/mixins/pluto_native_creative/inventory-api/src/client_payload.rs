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
}
