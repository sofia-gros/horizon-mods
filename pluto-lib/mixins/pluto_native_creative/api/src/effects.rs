//! Immutable curative-item policy values for an explicitly owned effect instance.
//! This does not attach metadata to Horizon instances or remove/apply effects.
use std::fmt;

pub const MAX_CURATIVES: usize = 256;
pub const MAX_ITEM_ID_BYTES: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error { TooManyItems, InvalidItemId { index: usize } }
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "invalid curative policy: {self:?}") }
}
impl std::error::Error for Error {}

/// None means the caller has not resolved the effect's default items yet.
/// Some(empty) is an explicit empty cure list, never an unresolved default.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Curatives { items: Option<Vec<String>> }

/// Explicit source operation, not inferred from duration/amplifier changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transfer {
    /// Updating the current active instance retains its own curative list.
    OrdinaryUpdate,
    /// Copying details, including hidden-effect restoration, copies its list.
    DetailsCopy,
}

fn item_id(id: &str) -> bool {
    if id.len() > MAX_ITEM_ID_BYTES { return false; }
    let Some((namespace, path)) = id.split_once(':') else { return false; };
    let part = |s: &str, slash: bool| !s.is_empty() && s.bytes().all(|c|
        c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, b'_'|b'.'|b'-') || (slash && c == b'/'));
    part(namespace, false) && part(path, true)
}
impl Curatives {
    /// No implicit milk or other item default. The effect-owning caller supplies it.
    pub fn unresolved() -> Self { Self { items: None } }
    /// Preserve caller order and duplicates; matching uses only canonical item ID.
    /// Tags, counts and other item payload do not participate in this boundary.
    pub fn explicit(items: Vec<String>) -> Result<Self, Error> {
        if items.len() > MAX_CURATIVES { return Err(Error::TooManyItems); }
        if let Some(index) = items.iter().position(|id| !item_id(id)) {
            return Err(Error::InvalidItemId { index });
        }
        Ok(Self { items: Some(items) })
    }
    pub fn items(&self) -> Option<&[String]> { self.items.as_deref() }
    /// Resolve exactly once into a new owned value. Already-explicit values,
    /// including empty, never call the default supplier or change their items.
    pub fn resolve_with(self, default: impl FnOnce() -> Vec<String>) -> Result<Self, Error> {
        if self.items.is_some() { Ok(self) } else { Self::explicit(default()) }
    }
    /// None is unresolved, not allowed or denied. Invalid/noncanonical candidates
    /// cannot match a validated canonical identity and return Some(false).
    pub fn allows(&self, item: &str) -> Option<bool> {
        self.items.as_ref().map(|items| items.iter().any(|known| known == item))
    }
    /// Caller owns the actual instance/hidden-chain lifetime and selects the
    /// operation. This does not observe or mirror arbitrary native updates.
    pub fn transferred(&self, incoming: &Self, operation: Transfer) -> Self {
        match operation { Transfer::OrdinaryUpdate => self.clone(), Transfer::DetailsCopy => incoming.clone() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn list(ids: &[&str]) -> Curatives { Curatives::explicit(ids.iter().map(|s| (*s).into()).collect()).unwrap() }
    #[test]
    fn unresolved_and_explicit_empty_are_distinct_and_never_assume_milk() {
        let unresolved = Curatives::unresolved();
        assert_eq!(unresolved.allows("example:cure"), None);
        let empty = list(&[]);
        assert_eq!(empty.allows("example:cure"), Some(false));
        assert_eq!(empty.clone().resolve_with(|| panic!("empty list is already explicit")).unwrap(), empty);
        assert_eq!(unresolved.resolve_with(|| vec!["example:cure".into()]).unwrap().allows("example:cure"), Some(true));
    }
    #[test]
    fn lazy_resolution_runs_once_and_preserves_order_and_duplicates() {
        let first = Curatives::unresolved().resolve_with(|| vec!["example:b".into(), "example:a".into(), "example:b".into()]).unwrap();
        let second = first.clone().resolve_with(|| panic!("defaults consulted twice")).unwrap();
        assert_eq!(second, first);
        assert_eq!(second.items().unwrap(), ["example:b", "example:a", "example:b"]);
    }
    #[test]
    fn ordinary_refresh_does_not_copy_incoming_policy_but_details_copy_does() {
        let current = list(&["example:a"]); let incoming = list(&[]);
        assert_eq!(current.transferred(&incoming, Transfer::OrdinaryUpdate), current);
        assert_eq!(current.transferred(&incoming, Transfer::DetailsCopy), incoming);
        assert_eq!(incoming.transferred(&current, Transfer::OrdinaryUpdate), incoming);
        assert_eq!(incoming.transferred(&current, Transfer::DetailsCopy), current);
    }
    #[test]
    fn explicit_hidden_instance_copy_and_restore_keep_their_own_policy() {
        let active = list(&["example:active"]);
        let hidden = active.clone();
        let lower_incoming = list(&["example:hidden"]);
        let stored_lower = lower_incoming.clone();
        assert_eq!(active.transferred(&lower_incoming, Transfer::OrdinaryUpdate), active);
        assert_eq!(active.transferred(&stored_lower, Transfer::DetailsCopy), lower_incoming);
        assert_eq!(hidden.allows("example:active"), Some(true));
    }
    #[test]
    fn clone_has_independent_list_storage_and_matching_uses_exact_item_identity() {
        let first = list(&["example:cure"]); let second = first.clone();
        assert_ne!(first.items().unwrap().as_ptr(), second.items().unwrap().as_ptr());
        for id in ["cure", "other:cure", "example:Cure", "example:cure{tag:1}"] {
            assert_eq!(first.allows(id), Some(false));
        }
    }
    #[test]
    fn malformed_and_oversized_lists_reject_before_any_policy_is_produced() {
        for id in ["", "cure", ":cure", "example:", "Example:cure", "example:a:b", "example:bad space"] {
            assert_eq!(Curatives::explicit(vec![id.into()]), Err(Error::InvalidItemId { index: 0 }));
        }
        assert_eq!(Curatives::explicit(vec!["example:cure".into(); MAX_CURATIVES + 1]), Err(Error::TooManyItems));
        assert!(Curatives::explicit(vec![format!("example:{}", "a".repeat(MAX_ITEM_ID_BYTES))]).is_err());
        assert!(list(&["example:folder/item_1"]).allows("example:folder/item_1").unwrap());
    }
}
