//! Opt-in preservation of selected saved recipe knowledge across native loading.
//! Capture, caller-declared readiness and one-shot restoration are separate.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::RwLock;

pub type Select = fn(&str) -> bool;
#[derive(Clone, Copy)]
struct Selector {
    select: Select,
    limit: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistrationError {
    InvalidId,
    InvalidLimit,
    DuplicateId,
    Poisoned,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaptureError {
    Poisoned,
    LimitExceeded(String),
    Overlap(String),
}

/// Only already-saved known IDs and their already-saved highlights are captured.
/// This DTO alone is not authority to grant recipes; the native pending owner
/// and current-world/index gates remain the restoration boundary.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Saved {
    pub known: BTreeSet<String>,
    pub highlighted: BTreeSet<String>,
}
impl Saved {
    /// Stage the accepted subset before any knowledge mutation. The callback
    /// runs once per still-indexed known ID, in lexical order. Highlights cannot
    /// introduce an ID absent from that accepted saved subset.
    pub fn plan(
        &self,
        contains: impl Fn(&str) -> bool,
        accepts: &mut dyn FnMut(&str) -> bool,
    ) -> Self {
        let known: BTreeSet<_> = self
            .known
            .iter()
            .filter(|id| contains(id) && accepts(id))
            .cloned()
            .collect();
        let highlighted = self.highlighted.intersection(&known).cloned().collect();
        Self { known, highlighted }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Readiness {
    Defer,
    Ready,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Restoration {
    Deferred,
    NoPending,
    Consumed {
        recipes_added: usize,
        highlights_added: usize,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestoreError {
    RemoteClient,
    ClientRole,
    GuestActor,
    TablesUnavailable,
    WorldMismatch,
    Poisoned,
}

/// Synchronous current-world capability. Declare Ready only after the consumer
/// has completed native world opening and validated its current recipe projection.
/// Do not call Ready from an earlier open/reload callback while Game fields are
/// still transitioning. Defer and adapter errors leave
/// pending data untouched. Ready consumes one owner before calling `accepts`;
/// rejection or panic cannot replay the same saved knowledge later.
/// No events, rewards, toasts or guest synchronization are implied.
pub trait Handoff {
    fn restore_saved(
        &mut self,
        owner: &str,
        readiness: Readiness,
        accepts: &mut dyn FnMut(&str) -> bool,
    ) -> Result<Restoration, RestoreError>;
}

#[derive(Default)]
struct Registry(RwLock<BTreeMap<String, Selector>>);
impl Registry {
    const fn new() -> Self {
        Self(RwLock::new(BTreeMap::new()))
    }
    fn register(&self, owner: &str, limit: usize, select: Select) -> Result<(), RegistrationError> {
        let Some((namespace, path)) = owner.split_once(':') else {
            return Err(RegistrationError::InvalidId);
        };
        if namespace.is_empty()
            || path.is_empty()
            || !namespace
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.-".contains(&c))
            || !path
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c))
            || path
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(RegistrationError::InvalidId);
        }
        if limit == 0 || limit == usize::MAX {
            return Err(RegistrationError::InvalidLimit);
        }
        let mut entries = self.0.write().map_err(|_| RegistrationError::Poisoned)?;
        if entries.contains_key(owner) {
            return Err(RegistrationError::DuplicateId);
        }
        entries.insert(owner.to_owned(), Selector { select, limit });
        Ok(())
    }
    fn remove(&self, owner: &str) -> Result<bool, RegistrationError> {
        Ok(self
            .0
            .write()
            .map_err(|_| RegistrationError::Poisoned)?
            .remove(owner)
            .is_some())
    }
    fn enabled(&self) -> bool {
        self.0.read().is_ok_and(|entries| !entries.is_empty())
    }
    fn capture(
        &self,
        known: &BTreeSet<String>,
        highlighted: &BTreeSet<String>,
    ) -> Result<BTreeMap<String, Saved>, CaptureError> {
        let entries = self.0.read().map_err(|_| CaptureError::Poisoned)?.clone();
        // No registry lock survives into selectors. State installation is later.
        let mut claimed = BTreeSet::new();
        let mut out = BTreeMap::new();
        for (owner, selector) in entries {
            let selected: BTreeSet<_> = known
                .iter()
                .filter(|id| (selector.select)(id))
                .take(selector.limit + 1)
                .cloned()
                .collect();
            if selected.len() > selector.limit {
                return Err(CaptureError::LimitExceeded(owner));
            }
            for id in &selected {
                if !claimed.insert(id.clone()) {
                    return Err(CaptureError::Overlap(id.clone()));
                }
            }
            if !selected.is_empty() {
                let highlighted = highlighted.intersection(&selected).cloned().collect();
                out.insert(
                    owner,
                    Saved {
                        known: selected,
                        highlighted,
                    },
                );
            }
        }
        Ok(out)
    }
}
static SELECTORS: Registry = Registry::new();
/// Register before opening a world. Selection and the bound are consumer policy.
/// Overlapping claims or an exceeded bound fail the complete capture closed.
/// No current world is retroactively captured by registration.
pub fn register(owner: &str, limit: usize, select: Select) -> Result<(), RegistrationError> {
    SELECTORS.register(owner, limit, select)
}
/// Affects future captures, not a snapshot already attached to a prepared world.
pub fn remove(owner: &str) -> Result<bool, RegistrationError> {
    SELECTORS.remove(owner)
}
pub fn enabled() -> bool {
    SELECTORS.enabled()
}
/// Native adapter entry, before filtering the actual prepared world's Settings.
pub fn capture(
    known: &BTreeSet<String>,
    highlighted: &BTreeSet<String>,
) -> Result<BTreeMap<String, Saved>, CaptureError> {
    SELECTORS.capture(known, highlighted)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn set(ids: &[&str]) -> BTreeSet<String> {
        ids.iter().map(|id| (*id).to_owned()).collect()
    }
    fn a(id: &str) -> bool {
        id.starts_with("a:")
    }
    fn b(id: &str) -> bool {
        id.starts_with("b:")
    }
    #[test]
    fn captures_only_selected_saved_members_and_intersected_highlights() {
        let r = Registry::new();
        r.register("test:a", 4, a).unwrap();
        let got = r
            .capture(
                &set(&["a:one", "b:two"]),
                &set(&["a:one", "a:unsaved", "b:two"]),
            )
            .unwrap();
        assert_eq!(
            got["test:a"],
            Saved {
                known: set(&["a:one"]),
                highlighted: set(&["a:one"])
            }
        );
        assert_eq!(
            r.capture(&BTreeSet::new(), &set(&["a:one"])),
            Ok(BTreeMap::new())
        );
    }
    #[test]
    fn registration_bounds_and_duplicates_fail_without_replacement() {
        let r = Registry::new();
        assert_eq!(r.register("bad", 1, a), Err(RegistrationError::InvalidId));
        assert_eq!(
            r.register("test:a", 0, a),
            Err(RegistrationError::InvalidLimit)
        );
        assert_eq!(
            r.register("test:a", usize::MAX, a),
            Err(RegistrationError::InvalidLimit)
        );
        r.register("test:a", 1, a).unwrap();
        assert_eq!(
            r.register("test:a", 2, b),
            Err(RegistrationError::DuplicateId)
        );
        assert_eq!(
            r.capture(&set(&["a:1", "a:2"]), &BTreeSet::new()),
            Err(CaptureError::LimitExceeded("test:a".into()))
        );
        assert_eq!(r.remove("test:a"), Ok(true));
        assert!(!r.enabled());
    }
    #[test]
    fn overlapping_owners_fail_without_a_partial_capture() {
        let r = Registry::new();
        r.register("test:a", 3, a).unwrap();
        r.register("test:b", 3, a).unwrap();
        assert_eq!(
            r.capture(&set(&["a:1"]), &BTreeSet::new()),
            Err(CaptureError::Overlap("a:1".into()))
        );
    }
    #[test]
    fn disjoint_owners_keep_independent_snapshots() {
        let r = Registry::new();
        r.register("test:b", 3, b).unwrap();
        r.register("test:a", 3, a).unwrap();
        let mut captured = r.capture(&set(&["b:1", "a:1"]), &BTreeSet::new()).unwrap();
        assert_eq!(captured.remove("test:a").unwrap().known, set(&["a:1"]));
        assert_eq!(captured["test:b"].known, set(&["b:1"]));
    }
    #[test]
    fn plan_intersects_saved_current_index_and_consumer_eligibility() {
        let saved = Saved {
            known: set(&["a:gone", "a:kept", "a:reject"]),
            highlighted: set(&["a:kept", "a:reject", "a:never_saved"]),
        };
        let mut seen = Vec::new();
        let plan = saved.plan(|id| id != "a:gone", &mut |id| {
            seen.push(id.to_owned());
            id != "a:reject"
        });
        assert_eq!(seen, ["a:kept", "a:reject"]);
        assert_eq!(
            plan,
            Saved {
                known: set(&["a:kept"]),
                highlighted: set(&["a:kept"])
            }
        );
        assert_eq!(saved.known.len(), 3);
    }
    static CHANGING: Registry = Registry::new();
    fn removing(id: &str) -> bool {
        CHANGING.remove("test:b").unwrap();
        a(id)
    }
    #[test]
    fn selectors_run_unlocked_and_registration_changes_affect_next_capture() {
        CHANGING.register("test:a", 3, removing).unwrap();
        CHANGING.register("test:b", 3, b).unwrap();
        let known = set(&["a:1", "b:1"]);
        assert_eq!(CHANGING.capture(&known, &BTreeSet::new()).unwrap().len(), 2);
        assert_eq!(CHANGING.capture(&known, &BTreeSet::new()).unwrap().len(), 1);
        CHANGING.remove("test:a").unwrap();
    }
}
