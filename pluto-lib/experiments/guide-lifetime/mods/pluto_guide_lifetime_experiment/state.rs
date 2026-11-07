// Isolated lifetime experiment, not an installed/public guide API.
// These identities correlate a local presentation and request, not authority.
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_OPEN: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Opening(u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    opening: Opening,
    sequence: u64,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    InvalidName,
    Exhausted,
    Inactive,
}
struct Session {
    opening: Opening,
    epoch: u64,
    owner: String,
    channel: String,
    sequence: u64,
    pending: Option<Request>,
}
#[derive(Default)]
pub struct State {
    session: Option<Session>,
}
fn valid_name(name: &str) -> bool {
    let Some((namespace, path)) = name.split_once(':') else {
        return false;
    };
    !namespace.is_empty()
        && !path.is_empty()
        && name.len() <= 256
        && namespace
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.-".contains(&c))
        && path
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c))
}
impl State {
    /// Called only by an explicitly owned opening path in a future adapter.
    /// The native experiment intentionally has no such activation path.
    pub fn open(&mut self, epoch: u64, owner: &str, channel: &str) -> Result<Opening, Error> {
        self.open_using(&NEXT_OPEN, epoch, owner, channel)
    }
    fn open_using(
        &mut self,
        counter: &AtomicU64,
        epoch: u64,
        owner: &str,
        channel: &str,
    ) -> Result<Opening, Error> {
        self.invalidate();
        if !valid_name(owner) || !valid_name(channel) {
            return Err(Error::InvalidName);
        }
        // MAX is a permanently exhausted sentinel, never returned or reset.
        let opening = Opening(
            counter
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
                .map_err(|_| Error::Exhausted)?,
        );
        self.session = Some(Session {
            opening,
            epoch,
            owner: owner.into(),
            channel: channel.into(),
            sequence: 0,
            pending: None,
        });
        Ok(opening)
    }
    pub fn invalidate(&mut self) {
        self.session = None;
    }
    pub fn active(&self, opening: Opening) -> bool {
        self.session.as_ref().is_some_and(|s| s.opening == opening)
    }
    pub fn request(&mut self, opening: Opening) -> Result<Request, Error> {
        let session = self
            .session
            .as_mut()
            .filter(|s| s.opening == opening)
            .ok_or(Error::Inactive)?;
        let Some(sequence) = session.sequence.checked_add(1) else {
            self.invalidate();
            return Err(Error::Exhausted);
        };
        session.sequence = sequence;
        let request = Request { opening, sequence };
        session.pending = Some(request);
        Ok(request)
    }
    /// Consume a matching pending reply before any later caller effect.
    /// Hidden replies are dropped; an absent/changed level revokes the session.
    /// No document mutation or network operation happens in this method.
    pub fn accept(
        &mut self,
        request: Request,
        epoch: Option<u64>,
        admitted: bool,
        owner: &str,
        channel: &str,
    ) -> bool {
        let Some(session) = self.session.as_mut() else {
            return false;
        };
        if epoch != Some(session.epoch) {
            self.invalidate();
            return false;
        }
        if session.pending != Some(request) || session.owner != owner || session.channel != channel
        {
            return false;
        }
        session.pending = None;
        admitted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const OWNER: &str = "example:guide";
    const CHANNEL: &str = "example:reply";
    fn open(state: &mut State, counter: &AtomicU64) -> Opening {
        state.open_using(counter, 7, OWNER, CHANNEL).unwrap()
    }
    #[test]
    fn identical_content_reopen_never_reuses_identity() {
        let counter = AtomicU64::new(1);
        let mut state = State::default();
        let first = open(&mut state, &counter);
        let late = state.request(first).unwrap();
        state.invalidate();
        let second = open(&mut state, &counter);
        assert_ne!(first, second);
        assert!(!state.active(first));
        assert!(state.active(second));
        assert!(!state.accept(late, Some(7), true, OWNER, CHANNEL));
    }
    #[test]
    fn same_key_player_reset_requires_explicit_invalidation() {
        let counter = AtomicU64::new(1);
        let mut state = State::default();
        let opening = open(&mut state, &counter);
        let reply = state.request(opening).unwrap();
        state.invalidate();
        assert!(!state.accept(reply, Some(7), true, OWNER, CHANNEL));
    }
    #[test]
    fn close_restore_reconfiguration_and_disconnect_revoke() {
        let counter = AtomicU64::new(1);
        for _event in [
            "close",
            "direct_restore",
            "reconfigure",
            "disconnect",
            "save_take",
            "clear_empty",
        ] {
            let mut state = State::default();
            let opening = open(&mut state, &counter);
            let reply = state.request(opening).unwrap();
            state.invalidate();
            assert!(!state.accept(reply, Some(7), true, OWNER, CHANNEL));
        }
    }
    #[test]
    fn latest_pending_request_wins_and_duplicate_is_consumed() {
        let counter = AtomicU64::new(1);
        let mut state = State::default();
        let opening = open(&mut state, &counter);
        let old = state.request(opening).unwrap();
        let current = state.request(opening).unwrap();
        assert!(!state.accept(old, Some(7), true, OWNER, CHANNEL));
        assert!(state.accept(current, Some(7), true, OWNER, CHANNEL));
        assert!(!state.accept(current, Some(7), true, OWNER, CHANNEL));
    }
    #[test]
    fn hidden_reply_is_not_replayed_after_overlay_dismissal() {
        let counter = AtomicU64::new(1);
        let mut state = State::default();
        let opening = open(&mut state, &counter);
        let reply = state.request(opening).unwrap();
        assert!(!state.accept(reply, Some(7), false, OWNER, CHANNEL));
        assert!(!state.accept(reply, Some(7), true, OWNER, CHANNEL));
        assert!(state.active(opening));
    }
    #[test]
    fn absent_or_changed_epoch_revokes_without_cross_world_revival() {
        let counter = AtomicU64::new(1);
        for epoch in [None, Some(8)] {
            let mut state = State::default();
            let opening = open(&mut state, &counter);
            let reply = state.request(opening).unwrap();
            assert!(!state.accept(reply, epoch, true, OWNER, CHANNEL));
            assert!(!state.active(opening));
            assert!(!state.accept(reply, Some(7), true, OWNER, CHANNEL));
        }
    }
    #[test]
    fn owner_and_channel_mismatch_cannot_consume_current_request() {
        let counter = AtomicU64::new(1);
        let mut state = State::default();
        let opening = open(&mut state, &counter);
        let reply = state.request(opening).unwrap();
        assert!(!state.accept(reply, Some(7), true, "other:guide", CHANNEL));
        assert!(!state.accept(reply, Some(7), true, OWNER, "other:reply"));
        assert!(state.accept(reply, Some(7), true, OWNER, CHANNEL));
    }
    #[test]
    fn map_replacement_and_two_clients_cannot_reuse_open_nonce() {
        let counter = AtomicU64::new(1);
        let mut a = State::default();
        let mut b = State::default();
        let first = open(&mut a, &counter);
        let reply = a.request(first).unwrap();
        let second = open(&mut b, &counter);
        assert_ne!(first, second);
        assert!(!b.accept(reply, Some(7), true, OWNER, CHANNEL));
        a = State::default();
        let third = open(&mut a, &counter);
        assert_ne!(first, third);
        assert!(!a.accept(reply, Some(7), true, OWNER, CHANNEL));
    }
    #[test]
    fn open_nonce_exhaustion_never_wraps() {
        let counter = AtomicU64::new(u64::MAX - 1);
        let mut state = State::default();
        let last = open(&mut state, &counter);
        assert_eq!(
            state.open_using(&counter, 7, OWNER, CHANNEL),
            Err(Error::Exhausted)
        );
        assert!(!state.active(last));
        assert_eq!(counter.load(Ordering::Relaxed), u64::MAX);
        assert_eq!(
            state.open_using(&counter, 7, OWNER, CHANNEL),
            Err(Error::Exhausted)
        );
    }
    #[test]
    fn request_exhaustion_revokes_instead_of_wrapping() {
        let counter = AtomicU64::new(1);
        let mut state = State::default();
        let opening = open(&mut state, &counter);
        state.session.as_mut().unwrap().sequence = u64::MAX;
        assert_eq!(state.request(opening), Err(Error::Exhausted));
        assert!(!state.active(opening));
    }
    #[test]
    fn malformed_owner_names_do_not_activate() {
        let counter = AtomicU64::new(1);
        let mut state = State::default();
        for name in ["", ":empty", "empty:", "UPPER:id", "double:colon:value"] {
            assert_eq!(
                state.open_using(&counter, 7, name, CHANNEL),
                Err(Error::InvalidName)
            );
        }
        assert_eq!(counter.load(Ordering::Relaxed), 1);
    }
}
