// Excluded reference model for a proposed native retained-presentation contract.
// This does not observe or mutate Game/BookScreen. Native identities do not yet
// exist at the reviewed official cutoff. Do not install this as a screen API.
#![allow(dead_code)]

// Reuse the actual ordinary correlation implementation, not a second registry.
#[path = "../../mixins/pluto_native_creative/inventory-api/src/client_payload.rs"]
mod client_payload;
#[path = "../../mixins/pluto_native_creative/inventory-api/src/guide/requests.rs"]
mod requests;

use requests::{RequestId, RequestTracker};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Error {
    Exhausted,
    NoIdentity,
    WrongOwner,
    WrongChannel,
    Envelope,
    NotPending,
    Revoked,
    CurrentChanged,
    InvalidContent,
}
fn allocate(counter: &AtomicU64) -> Result<u64, Error> {
    counter
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .map_err(|_| Error::Exhausted)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Identity {
    attachment: u64,
    opening: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Stamp {
    identity: Identity,
    revision: u64,
}
struct Entry<T> {
    opening: Option<u64>,
    revision: u64,
    owner: Option<String>,
    value: T,
}
// Not Clone/Copy; observation of a Stamp cannot construct this handle. The
// explicit owned-open path is the only constructor outside the test module.
struct OwnedOpening {
    identity: Identity,
    owner: String,
}
/// Proposed host semantics only. An overlay changes input visibility, not this
/// retained slot. Replacement/restoration always creates a fresh opening.
struct RetainedSlot<T> {
    attachment: Option<u64>,
    entry: Option<Entry<T>>,
    overlay: bool,
}
impl<T> RetainedSlot<T> {
    fn new() -> Self {
        Self {
            attachment: allocate(&NEXT_ID).ok(),
            entry: None,
            overlay: false,
        }
    }
    fn install(&mut self, value: T) -> Result<Stamp, Error> {
        self.install_using(value, &NEXT_ID)
    }
    fn install_owned(&mut self, value: T, owner: &str) -> Result<OwnedOpening, Error> {
        if !client_payload::valid_id(owner) { return Err(Error::WrongOwner); }
        let stamp = self.install(value)?;
        self.entry.as_mut().unwrap().owner = Some(owner.to_owned());
        Ok(OwnedOpening { identity: stamp.identity, owner: owner.to_owned() })
    }
    fn install_using(&mut self, value: T, counter: &AtomicU64) -> Result<Stamp, Error> {
        // Identity exhaustion must disable asynchronous ownership, not remove
        // the ordinary local presentation or stop native open/restore behavior.
        self.entry = Some(Entry {
            opening: allocate(counter).ok(),
            revision: 0,
            owner: None,
            value,
        });
        self.stamp()
    }
    fn stamp(&self) -> Result<Stamp, Error> {
        let entry = self.entry.as_ref().ok_or(Error::NoIdentity)?;
        Ok(Stamp {
            identity: Identity {
                attachment: self.attachment.ok_or(Error::NoIdentity)?,
                opening: entry.opening.ok_or(Error::NoIdentity)?,
            },
            revision: entry.revision,
        })
    }
    fn value(&self) -> Option<&T> {
        self.entry.as_ref().map(|entry| &entry.value)
    }
    fn take(&mut self) -> Option<T> {
        self.entry.take().map(|entry| entry.value)
    }
    fn overlay(&mut self, visible: bool) {
        self.overlay = visible;
    }
    fn reset_attachment(&mut self) {
        // A same-key player reset revokes leases even if a native book remains.
        self.attachment = allocate(&NEXT_ID).ok();
    }
    fn revise(&mut self, value: T) -> Result<(), Error> {
        let entry = self.entry.as_mut().ok_or(Error::NoIdentity)?;
        let Some(revision) = entry.revision.checked_add(1) else {
            entry.opening = None;
            return Err(Error::Exhausted);
        };
        entry.revision = revision;
        entry.value = value;
        Ok(())
    }
}

/// An explicitly selected legacy profile, not an added on-wire version byte.
/// Correlation is an opaque echoed u64; it is not authentication or authority.
struct LegacyU64PrefixV1<'a> {
    correlation: u64,
    body: &'a [u8],
}
impl<'a> LegacyU64PrefixV1<'a> {
    fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        const HEADER: usize = 8;
        let maximum = client_payload::MAX_BYTES.checked_add(HEADER).ok_or(Error::Envelope)?;
        if !(HEADER..=maximum).contains(&bytes.len()) {
            return Err(Error::Envelope);
        }
        let (header, body) = bytes.split_at(HEADER);
        Ok(Self {
            correlation: u64::from_be_bytes(header.try_into().map_err(|_| Error::Envelope)?),
            body,
        })
    }
}
#[derive(Clone, Copy)]
struct Pending {
    wire: u64,
    request: RequestId,
}
/// An excluded adapter model. Owner labels alone cannot construct a real native
/// lease. The proposed official owned-open operation must supply the identity.
struct Replies {
    identity: Identity,
    tracker: RequestTracker,
    pending: Option<Pending>,
}
struct Prepared<'a, T> {
    stamp: Stamp,
    pending: Pending,
    owner: &'a str,
    channel: &'a str,
    body: &'a [u8],
    replacement: T,
}
impl Replies {
    fn after_explicit_open<T>(slot: &RetainedSlot<T>, opening: OwnedOpening, channel: &str) -> Result<Self, Error> {
        let identity = slot.stamp()?.identity;
        if identity != opening.identity || slot.entry.as_ref().and_then(|e| e.owner.as_deref()) != Some(opening.owner.as_str()) {
            return Err(Error::Revoked);
        }
        let mut tracker = RequestTracker::new(&opening.owner, channel).map_err(|_| Error::InvalidContent)?;
        tracker.restart().map_err(|_| Error::Exhausted)?;
        Ok(Self { identity, tracker, pending: None })
    }
    fn invalidate(&mut self) {
        self.pending = None;
        self.tracker.invalidate();
    }
    fn current<T>(&mut self, slot: &RetainedSlot<T>) -> Result<Stamp, Error> {
        if !self.tracker.is_active() { return Err(Error::Revoked); }
        match slot.stamp() {
            Ok(stamp) if stamp.identity == self.identity
                && slot.entry.as_ref().and_then(|e| e.owner.as_deref()) == Some(self.tracker.owner()) => Ok(stamp),
            _ => {
                self.invalidate();
                Err(Error::Revoked)
            }
        }
    }
    fn request<T>(&mut self, slot: &RetainedSlot<T>) -> Result<u64, Error> {
        self.current(slot)?;
        let wire = match allocate(&NEXT_ID) {
            Ok(wire) => wire,
            Err(error) => { self.invalidate(); return Err(error); }
        };
        let request = self.tracker.issue().map_err(|_| {
            self.pending = None;
            Error::Exhausted
        })?;
        self.pending = Some(Pending { wire, request });
        Ok(wire)
    }
    fn prepare<'a, T>(
        &mut self,
        slot: &RetainedSlot<T>,
        owner: &'a str,
        channel: &'a str,
        bytes: &'a [u8],
        decode: impl FnOnce(&[u8], &T) -> Result<T, Error>,
    ) -> Result<Prepared<'a, T>, Error> {
        // This operation must be called after read-only callbacks. An overlay
        // is deliberately not an input gate for retained-content replacement.
        let stamp = self.current(slot)?;
        if owner != self.tracker.owner() { return Err(Error::WrongOwner); }
        if channel != self.tracker.channel() { return Err(Error::WrongChannel); }
        let frame = LegacyU64PrefixV1::parse(bytes)?;
        let pending = self.pending.filter(|p| p.wire == frame.correlation).ok_or(Error::NotPending)?;
        let replacement = decode(frame.body, slot.value().ok_or(Error::Revoked)?)?;
        Ok(Prepared { stamp, pending, owner, channel, body: frame.body, replacement })
    }
    fn commit<T>(&mut self, slot: &mut RetainedSlot<T>, prepared: Prepared<'_, T>) -> Result<(), Error> {
        let current = self.current(slot)?;
        if current != prepared.stamp { return Err(Error::CurrentChanged); }
        if !self.pending.is_some_and(|p| p.wire == prepared.pending.wire && p.request == prepared.pending.request) {
            return Err(Error::NotPending);
        }
        if current.revision == u64::MAX {
            slot.entry.as_mut().unwrap().opening = None;
            self.invalidate();
            return Err(Error::Exhausted);
        }
        self.tracker.consume(prepared.pending.request, prepared.owner, client_payload::Payload {
            channel: prepared.channel, bytes: prepared.body,
        }).map_err(|_| Error::NotPending)?;
        self.pending = None;
        // The exclusive borrow prevents another safe direct slot mutation
        // between these checks and assignment. However, dropping generic T can
        // run arbitrary code or panic after consumption: no rollback guarantee.
        slot.revise(prepared.replacement)
    }
}

#[cfg(test)]
mod retained_tests {
    use super::*;
    const OWNER: &str = "fixture:guide";
    const CHANNEL: &str = "fixture:reply";
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Pages { text: String, position: usize }
    fn fixture(position: usize) -> (RetainedSlot<Pages>, Replies) {
        let mut slot = RetainedSlot::new();
        let opening = slot.install_owned(Pages { text: "old".into(), position }, OWNER).unwrap();
        let replies = Replies::after_explicit_open(&slot, opening, CHANNEL).unwrap();
        (slot, replies)
    }
    fn frame(wire: u64, body: &[u8]) -> Vec<u8> {
        let mut result = wire.to_be_bytes().to_vec(); result.extend_from_slice(body); result
    }
    fn decode(body: &[u8], current: &Pages) -> Result<Pages, Error> {
        if body != b"new" { return Err(Error::InvalidContent); }
        // Test consumer policy, not Pluto/native cover semantics.
        Ok(Pages { text: "new".into(), position: if current.position == usize::MAX { usize::MAX } else { current.position.min(2) } })
    }
    #[test]
    fn overlay_refresh_retains_identity_and_consumer_cover_without_input_permission() {
        let (mut slot, mut replies) = fixture(usize::MAX);
        let stamp = slot.stamp().unwrap(); let wire = replies.request(&slot).unwrap();
        slot.overlay(true); assert_eq!(slot.stamp().unwrap(), stamp);
        let bytes = frame(wire, b"new");
        let proposal = replies.prepare(&slot, OWNER, CHANNEL, &bytes, decode).unwrap();
        replies.commit(&mut slot, proposal).unwrap();
        assert_eq!(slot.value().unwrap(), &Pages { text: "new".into(), position: usize::MAX });
        slot.overlay(false); assert_eq!(slot.stamp().unwrap().identity, stamp.identity);
    }
    #[test]
    fn identical_reopen_and_take_restore_revoke_old_reply() {
        for take in [false, true] {
            let (mut slot, mut replies) = fixture(0); let old = slot.stamp().unwrap();
            let bytes = frame(replies.request(&slot).unwrap(), b"new");
            let value = if take { slot.take().unwrap() } else { slot.value().unwrap().clone() };
            slot.install(value).unwrap(); assert_ne!(slot.stamp().unwrap().identity, old.identity);
            assert!(matches!(replies.prepare(&slot, OWNER, CHANNEL, &bytes, decode), Err(Error::Revoked)));
            assert_eq!(slot.value().unwrap().text, "old");
        }
    }
    #[test]
    fn same_key_actor_reset_revokes_even_when_retained_content_survives() {
        let (mut slot, mut replies) = fixture(0); let bytes = frame(replies.request(&slot).unwrap(), b"new");
        slot.reset_attachment();
        assert!(matches!(replies.prepare(&slot, OWNER, CHANNEL, &bytes, decode), Err(Error::Revoked)));
        assert_eq!(slot.value().unwrap().text, "old");
    }
    #[test]
    fn navigation_before_reply_is_preserved_then_consumer_clamps() {
        let (mut slot, mut replies) = fixture(0); let bytes = frame(replies.request(&slot).unwrap(), b"new");
        slot.revise(Pages { text: "old".into(), position: 8 }).unwrap();
        let proposal = replies.prepare(&slot, OWNER, CHANNEL, &bytes, decode).unwrap();
        replies.commit(&mut slot, proposal).unwrap(); assert_eq!(slot.value().unwrap().position, 2);
    }
    #[test]
    fn intervening_content_change_rejects_prepared_proposal_without_consuming_request() {
        let (mut slot, mut replies) = fixture(0); let bytes = frame(replies.request(&slot).unwrap(), b"new");
        let proposal = replies.prepare(&slot, OWNER, CHANNEL, &bytes, decode).unwrap();
        slot.revise(Pages { text: "changed".into(), position: 1 }).unwrap();
        assert_eq!(replies.commit(&mut slot, proposal), Err(Error::CurrentChanged));
        let retry = replies.prepare(&slot, OWNER, CHANNEL, &bytes, decode).unwrap();
        replies.commit(&mut slot, retry).unwrap();
    }
    #[test]
    fn newest_request_wins_and_duplicate_commit_cannot_apply_twice() {
        let (mut slot, mut replies) = fixture(0); let old = frame(replies.request(&slot).unwrap(), b"new");
        let bytes = frame(replies.request(&slot).unwrap(), b"new");
        assert!(matches!(replies.prepare(&slot, OWNER, CHANNEL, &old, decode), Err(Error::NotPending)));
        let first = replies.prepare(&slot, OWNER, CHANNEL, &bytes, decode).unwrap();
        let duplicate = replies.prepare(&slot, OWNER, CHANNEL, &bytes, decode).unwrap();
        replies.commit(&mut slot, first).unwrap();
        assert!(matches!(replies.commit(&mut slot, duplicate), Err(Error::CurrentChanged | Error::NotPending)));
        assert!(matches!(replies.prepare(&slot, OWNER, CHANNEL, &bytes, decode), Err(Error::NotPending)));
    }
    #[test]
    fn malformed_and_wrong_labels_do_not_consume_pending() {
        let (mut slot, mut replies) = fixture(0); let wire = replies.request(&slot).unwrap();
        for (owner, channel, bytes, expected) in [
            ("other:owner", CHANNEL, frame(wire, b"new"), Error::WrongOwner),
            (OWNER, "other:channel", frame(wire, b"new"), Error::WrongChannel),
            (OWNER, CHANNEL, vec![0; 7], Error::Envelope),
            (OWNER, CHANNEL, frame(wire, b"invalid"), Error::InvalidContent),
            (OWNER, CHANNEL, frame(wire, &vec![0; client_payload::MAX_BYTES + 1]), Error::Envelope),
        ] {
            assert!(matches!(replies.prepare(&slot, owner, channel, &bytes, decode), Err(e) if e == expected));
        }
        let bytes = frame(wire, b"new"); let valid = replies.prepare(&slot, OWNER, CHANNEL, &bytes, decode).unwrap();
        replies.commit(&mut slot, valid).unwrap();
    }
    #[test]
    fn full_legacy_body_fits_plus_header_without_raising_read_only_cap_or_copying() {
        let (mut slot, mut replies) = fixture(0); let wire = replies.request(&slot).unwrap();
        let bytes = frame(wire, &vec![7; client_payload::MAX_BYTES]);
        assert_eq!(bytes.len(), 262152);
        let parsed = LegacyU64PrefixV1::parse(&bytes).unwrap();
        assert_eq!(parsed.body.as_ptr(), bytes[8..].as_ptr());
        let proposal = replies.prepare(&slot, OWNER, CHANNEL, &bytes, |body, old| {
            assert_eq!(body.len(), client_payload::MAX_BYTES); Ok(old.clone())
        }).unwrap();
        replies.commit(&mut slot, proposal).unwrap();
        assert_eq!(client_payload::MAX_BYTES, 262144);
    }
    #[test]
    fn unrelated_client_and_replaced_slot_cannot_accept_an_old_request() {
        let (slot, mut replies) = fixture(0); let bytes = frame(replies.request(&slot).unwrap(), b"new");
        let (other, _) = fixture(0);
        assert!(matches!(replies.prepare(&other, OWNER, CHANNEL, &bytes, decode), Err(Error::Revoked)));
        assert!(matches!(replies.prepare(&slot, OWNER, CHANNEL, &bytes, decode), Err(Error::Revoked)));
    }
    #[test]
    fn identity_and_revision_exhaustion_preserve_local_presentation_but_revoke_capability() {
        let (mut slot, mut replies) = fixture(0); let bytes = frame(replies.request(&slot).unwrap(), b"new");
        slot.entry.as_mut().unwrap().revision = u64::MAX;
        let proposal = replies.prepare(&slot, OWNER, CHANNEL, &bytes, decode).unwrap();
        assert_eq!(replies.commit(&mut slot, proposal), Err(Error::Exhausted));
        assert_eq!(slot.value().unwrap().text, "old"); assert_eq!(slot.stamp(), Err(Error::NoIdentity));
        let exhausted = AtomicU64::new(u64::MAX);
        assert_eq!(slot.install_using(Pages { text: "still visible".into(), position: 0 }, &exhausted), Err(Error::NoIdentity));
        assert_eq!(slot.value().unwrap().text, "still visible");
        assert_eq!(allocate(&exhausted), Err(Error::Exhausted));
        assert_eq!(exhausted.load(Ordering::Relaxed), u64::MAX);
    }
    #[test]
    fn destructor_panic_after_commit_consumption_is_not_a_retry_or_rollback() {
        use std::sync::{Arc, atomic::AtomicBool};
        struct DropValue(Arc<AtomicBool>);
        impl Drop for DropValue {
            fn drop(&mut self) {
                if self.0.swap(false, Ordering::SeqCst) {
                    panic!("modeled destructor effect after commit starts");
                }
            }
        }
        let mut slot = RetainedSlot::new();
        let opening = slot.install_owned(DropValue(Arc::new(AtomicBool::new(true))), OWNER).unwrap();
        let mut replies = Replies::after_explicit_open(&slot, opening, CHANNEL).unwrap();
        let bytes = frame(replies.request(&slot).unwrap(), b"opaque");
        let proposal = replies.prepare(&slot, OWNER, CHANNEL, &bytes, |_, _| {
            Ok(DropValue(Arc::new(AtomicBool::new(false))))
        }).unwrap();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            replies.commit(&mut slot, proposal)
        }));
        assert!(outcome.is_err());
        assert!(replies.pending.is_none());
        assert!(!replies.tracker.has_pending());
        // Deliberately make no assertion about rollback or retained value.
        assert!(matches!(replies.prepare(&slot, OWNER, CHANNEL, &bytes, |_, _| {
            Ok(DropValue(Arc::new(AtomicBool::new(false))))
        }), Err(Error::NotPending)));
    }
    #[test]
    fn ordinary_observation_never_creates_owned_opening_and_stale_handle_is_rejected() {
        let mut slot = RetainedSlot::new();
        slot.install(Pages { text: "ordinary".into(), position: 0 }).unwrap();
        assert!(slot.stamp().is_ok());
        assert!(slot.entry.as_ref().unwrap().owner.is_none());
        let opening = slot.install_owned(Pages { text: "owned".into(), position: 0 }, OWNER).unwrap();
        let replacement = slot.install_owned(Pages { text: "owned".into(), position: 0 }, "other:owner").unwrap();
        assert!(matches!(Replies::after_explicit_open(&slot, opening, CHANNEL), Err(Error::Revoked)));
        let replies = Replies::after_explicit_open(&slot, replacement, CHANNEL).unwrap();
        assert_eq!(replies.tracker.owner(), "other:owner");
    }
}
