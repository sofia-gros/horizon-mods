//! Explicit, owned correlation for in-process asynchronous work.
//!
//! This tracker does not observe a window, world, actor, connection or packet.
//! Its caller decides when to restart/invalidate it and separately validates
//! native context before any effect. Tokens are not authority or secrets, are
//! not persistent, and have no wire encoding. No registry or callback is added.
use crate::client_payload::{self, Payload};
use std::sync::atomic::{AtomicU64, Ordering};

// One allocator in the one shared API crate. MAX permanently means exhausted.
static NEXT_SCOPE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RequestId {
    scope: u64,
    sequence: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigError {
    InvalidOwner,
    InvalidChannel,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IssueError {
    Inactive,
    Exhausted,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejected {
    WrongOwner,
    WrongChannel,
    TooLarge,
    Inactive,
    NotPending,
}
struct Scope {
    id: u64,
    sequence: u64,
    pending: Option<RequestId>,
}
/// Non-cloneable owned correlation state, initially inactive.
///
/// Owner/channel are exact bounded labels, not identity or ownership proofs.
/// Moving the tracker preserves it; dropping it drops pending work. No native
/// event automatically invalidates it. Never restore it across process starts.
pub struct RequestTracker {
    owner: String,
    channel: String,
    scope: Option<Scope>,
}
impl RequestTracker {
    pub fn new(owner: &str, channel: &str) -> Result<Self, ConfigError> {
        if !client_payload::valid_id(owner) {
            return Err(ConfigError::InvalidOwner);
        }
        if !client_payload::valid_id(channel) {
            return Err(ConfigError::InvalidChannel);
        }
        Ok(Self {
            owner: owner.into(),
            channel: channel.into(),
            scope: None,
        })
    }
    pub fn owner(&self) -> &str {
        &self.owner
    }
    pub fn channel(&self) -> &str {
        &self.channel
    }
    /// Forget pending work and issue a fresh process-local correlation scope.
    /// This does not open, acquire, identify or authorize any native resource.
    pub fn restart(&mut self) -> Result<(), IssueError> {
        self.restart_using(&NEXT_SCOPE)
    }
    fn restart_using(&mut self, counter: &AtomicU64) -> Result<(), IssueError> {
        self.invalidate();
        let id = counter
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| IssueError::Exhausted)?;
        self.scope = Some(Scope {
            id,
            sequence: 0,
            pending: None,
        });
        Ok(())
    }
    /// Explicitly revoke this tracker's pending work. No native state changes.
    pub fn invalidate(&mut self) {
        self.scope = None;
    }
    /// Whether this tracker was restarted and has not since been invalidated.
    /// This says nothing about whether a native window/actor/connection exists.
    pub fn is_active(&self) -> bool {
        self.scope.is_some()
    }
    pub fn has_pending(&self) -> bool {
        self.scope.as_ref().is_some_and(|s| s.pending.is_some())
    }
    /// Issue the newest request, superseding any previous pending request.
    /// Counter exhaustion invalidates the tracker instead of wrapping.
    pub fn issue(&mut self) -> Result<RequestId, IssueError> {
        let scope = self.scope.as_mut().ok_or(IssueError::Inactive)?;
        let Some(sequence) = scope.sequence.checked_add(1) else {
            self.invalidate();
            return Err(IssueError::Exhausted);
        };
        scope.sequence = sequence;
        let id = RequestId {
            scope: scope.id,
            sequence,
        };
        scope.pending = Some(id);
        Ok(id)
    }
    /// Consume the matching pending result once, before the caller's effect.
    /// Returns exactly the original borrowed payload without copying its bytes.
    ///
    /// Owner/channel/size/stale failures do not consume a valid pending request.
    /// To drop a matching reply when native context is unavailable, call discard;
    /// do not queue it for automatic replay when the context becomes available.
    /// Payload bytes remain untrusted and require separate semantic validation.
    pub fn consume<'a>(
        &mut self,
        id: RequestId,
        owner: &str,
        payload: Payload<'a>,
    ) -> Result<Payload<'a>, Rejected> {
        if owner != self.owner {
            return Err(Rejected::WrongOwner);
        }
        if payload.channel != self.channel {
            return Err(Rejected::WrongChannel);
        }
        if payload.bytes.len() > client_payload::MAX_BYTES {
            return Err(Rejected::TooLarge);
        }
        let scope = self.scope.as_mut().ok_or(Rejected::Inactive)?;
        if scope.pending != Some(id) {
            return Err(Rejected::NotPending);
        }
        scope.pending = None;
        Ok(payload)
    }
    /// Drop this exact pending result without granting an effect. A stale ID
    /// cannot discard a newer pending result; false means nothing changed.
    pub fn discard(&mut self, id: RequestId) -> bool {
        let Some(scope) = self.scope.as_mut() else {
            return false;
        };
        if scope.pending != Some(id) {
            return false;
        }
        scope.pending = None;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const OWNER: &str = "example:guide";
    const CHANNEL: &str = "example:reply";
    fn tracker() -> RequestTracker {
        RequestTracker::new(OWNER, CHANNEL).unwrap()
    }
    fn payload(bytes: &[u8]) -> Payload<'_> {
        Payload {
            channel: CHANNEL,
            bytes,
        }
    }
    #[test]
    fn validates_labels_and_starts_inactive() {
        for bad in ["", "Bad:name", "x:a//b", "x:../b", "x:"] {
            assert!(matches!(
                RequestTracker::new(bad, CHANNEL),
                Err(ConfigError::InvalidOwner)
            ));
            assert!(matches!(
                RequestTracker::new(OWNER, bad),
                Err(ConfigError::InvalidChannel)
            ));
        }
        let mut t = tracker();
        assert_eq!(t.owner(), OWNER);
        assert_eq!(t.channel(), CHANNEL);
        assert!(!t.is_active());
        assert!(!t.has_pending());
        assert_eq!(t.issue(), Err(IssueError::Inactive));
    }
    #[test]
    fn latest_request_wins_and_payload_borrow_is_unchanged() {
        let mut t = tracker();
        t.restart().unwrap();
        let old = t.issue().unwrap();
        let newest = t.issue().unwrap();
        let bytes = [0, 255, 128];
        assert!(matches!(
            t.consume(old, OWNER, payload(&bytes)),
            Err(Rejected::NotPending)
        ));
        let accepted = t.consume(newest, OWNER, payload(&bytes)).unwrap();
        assert_eq!(accepted.bytes, bytes);
        assert_eq!(accepted.bytes.as_ptr(), bytes.as_ptr());
        assert!(!t.has_pending());
        assert!(matches!(
            t.consume(newest, OWNER, payload(&bytes)),
            Err(Rejected::NotPending)
        ));
    }
    #[test]
    fn mismatched_labels_and_size_do_not_consume_pending() {
        let mut t = tracker();
        t.restart().unwrap();
        let id = t.issue().unwrap();
        assert!(matches!(
            t.consume(id, "other:guide", payload(&[])),
            Err(Rejected::WrongOwner)
        ));
        assert!(matches!(
            t.consume(
                id,
                OWNER,
                Payload {
                    channel: "other:reply",
                    bytes: &[]
                }
            ),
            Err(Rejected::WrongChannel)
        ));
        let big = vec![0; client_payload::MAX_BYTES + 1];
        assert!(matches!(
            t.consume(id, OWNER, payload(&big)),
            Err(Rejected::TooLarge)
        ));
        assert!(t.consume(id, OWNER, payload(&[])).is_ok());
    }
    #[test]
    fn restart_and_invalidation_reject_old_work() {
        let mut t = tracker();
        t.restart().unwrap();
        let old = t.issue().unwrap();
        t.restart().unwrap();
        let new = t.issue().unwrap();
        assert_ne!(old, new);
        assert!(matches!(
            t.consume(old, OWNER, payload(&[])),
            Err(Rejected::NotPending)
        ));
        t.invalidate();
        assert!(matches!(
            t.consume(new, OWNER, payload(&[])),
            Err(Rejected::Inactive)
        ));
    }
    #[test]
    fn discarded_hidden_result_cannot_replay_or_discard_newer_work() {
        let mut t = tracker();
        t.restart().unwrap();
        let old = t.issue().unwrap();
        let new = t.issue().unwrap();
        assert!(!t.discard(old));
        assert!(t.discard(new));
        assert!(!t.discard(new));
        assert!(matches!(
            t.consume(new, OWNER, payload(&[])),
            Err(Rejected::NotPending)
        ));
    }
    #[test]
    fn independent_trackers_never_share_scope_even_with_same_labels() {
        let mut a = tracker();
        let mut b = tracker();
        a.restart().unwrap();
        b.restart().unwrap();
        let x = a.issue().unwrap();
        let y = b.issue().unwrap();
        assert_ne!(x, y);
        assert!(matches!(
            b.consume(x, OWNER, payload(&[])),
            Err(Rejected::NotPending)
        ));
        assert!(b.consume(y, OWNER, payload(&[])).is_ok());
    }
    #[test]
    fn scope_counter_exhaustion_fails_closed_permanently() {
        let counter = AtomicU64::new(u64::MAX - 1);
        let mut t = tracker();
        t.restart_using(&counter).unwrap();
        let last = t.issue().unwrap();
        assert_eq!(t.restart_using(&counter), Err(IssueError::Exhausted));
        assert!(!t.is_active());
        assert_eq!(counter.load(Ordering::Relaxed), u64::MAX);
        assert_eq!(t.restart_using(&counter), Err(IssueError::Exhausted));
        assert!(matches!(
            t.consume(last, OWNER, payload(&[])),
            Err(Rejected::Inactive)
        ));
    }
    #[test]
    fn request_counter_exhaustion_revokes_scope() {
        let mut t = tracker();
        t.restart().unwrap();
        t.scope.as_mut().unwrap().sequence = u64::MAX;
        assert_eq!(t.issue(), Err(IssueError::Exhausted));
        assert!(!t.is_active());
    }
    #[test]
    fn moving_tracker_preserves_only_its_owned_pending_work() {
        let mut t = tracker();
        t.restart().unwrap();
        let id = t.issue().unwrap();
        let mut moved = t;
        assert!(moved.consume(id, OWNER, payload(&[])).is_ok());
    }
}
