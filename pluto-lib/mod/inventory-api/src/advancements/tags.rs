//! Resolved item-tag overlays for advancement matching and candidate indexing.
//! Resolution, namespaces, owned tags and registry completeness are consumer policy.
use std::collections::BTreeMap;

/// Present Some(empty) is a known empty tag; None is owned but unavailable.
pub type TagSnapshot = BTreeMap<String, Option<Vec<String>>>;
/// One store per world, owned by the native adapter, never a process-global cache.
pub type OwnedSnapshots = BTreeMap<String, TagSnapshot>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Unchanged,
    Changed,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplaceError {
    EmptyOwner,
    Conflict { tag: String, owner: String },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lookup<'a> {
    Unmanaged,
    Unavailable,
    Members(&'a [String]),
    /// Defensive failure for externally supplied invalid stores. Normal replace
    /// rejects this state atomically; never choose a lexical winner for matching.
    Conflict,
}

/// Replace exactly one owner's snapshot atomically. An empty map releases its
/// ownership. Membership order/duplicates are normalized without parsing IDs.
/// A conflict leaves every owner unchanged; callers must handle the error.
pub fn replace_owner(
    store: &mut OwnedSnapshots,
    owner: &str,
    mut next: TagSnapshot,
) -> Result<Change, ReplaceError> {
    if owner.is_empty() {
        return Err(ReplaceError::EmptyOwner);
    }
    for (other, tags) in store.iter() {
        if other != owner {
            if let Some(tag) = next.keys().find(|tag| tags.contains_key(*tag)) {
                return Err(ReplaceError::Conflict {
                    tag: tag.clone(),
                    owner: other.clone(),
                });
            }
        }
    }
    for members in next.values_mut().flatten() {
        members.sort();
        members.dedup();
    }
    if next.is_empty() {
        return Ok(if store.remove(owner).is_some() {
            Change::Changed
        } else {
            Change::Unchanged
        });
    }
    if store.get(owner) == Some(&next) {
        return Ok(Change::Unchanged);
    }
    store.insert(owner.to_owned(), next);
    Ok(Change::Changed)
}

/// Unmanaged permits native fallback. Unavailable/Conflict must fail closed;
/// neither is interchangeable with a known empty tag in the public contract.
pub fn lookup<'a>(store: &'a OwnedSnapshots, tag: &str) -> Lookup<'a> {
    let mut found = None;
    for tags in store.values() {
        if let Some(members) = tags.get(tag) {
            if found.is_some() {
                return Lookup::Conflict;
            }
            found = Some(members);
        }
    }
    match found {
        None => Lookup::Unmanaged,
        Some(None) => Lookup::Unavailable,
        Some(Some(members)) => Lookup::Members(members),
    }
}

/// A synchronous current-world capability. Successful changes invalidate only
/// the shared custom candidate cache, never the native cache, inventory-event
/// hash or progress. Explicit invalidation has this same custom-only scope.
pub trait TagSink {
    fn replace_owner(&mut self, owner: &str, tags: TagSnapshot) -> Result<Change, ReplaceError>;
    fn invalidate_candidates(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tags(values: Option<Vec<&str>>) -> TagSnapshot {
        [(
            "example:tag".into(),
            values.map(|v| v.into_iter().map(str::to_owned).collect()),
        )]
        .into_iter()
        .collect()
    }
    #[test]
    fn absent_unknown_and_explicit_empty_are_distinct() {
        let mut store = OwnedSnapshots::new();
        assert_eq!(lookup(&store, "example:tag"), Lookup::Unmanaged);
        replace_owner(&mut store, "a", tags(None)).unwrap();
        assert_eq!(lookup(&store, "example:tag"), Lookup::Unavailable);
        replace_owner(&mut store, "a", tags(Some(vec![]))).unwrap();
        assert_eq!(lookup(&store, "example:tag"), Lookup::Members(&[]));
    }
    #[test]
    fn replacement_is_normalized_idempotent_and_removal_releases_ownership() {
        let mut store = OwnedSnapshots::new();
        assert_eq!(
            replace_owner(&mut store, "a", tags(Some(vec!["z", "a", "z"]))),
            Ok(Change::Changed)
        );
        assert_eq!(
            replace_owner(&mut store, "a", tags(Some(vec!["a", "z"]))),
            Ok(Change::Unchanged)
        );
        assert_eq!(
            lookup(&store, "example:tag"),
            Lookup::Members(&["a".into(), "z".into()])
        );
        assert_eq!(
            replace_owner(&mut store, "a", TagSnapshot::new()),
            Ok(Change::Changed)
        );
        assert_eq!(lookup(&store, "example:tag"), Lookup::Unmanaged);
        assert_eq!(
            replace_owner(&mut store, "a", TagSnapshot::new()),
            Ok(Change::Unchanged)
        );
    }
    #[test]
    fn conflicting_owner_update_is_atomic_and_does_not_change_other_tags() {
        let mut store = OwnedSnapshots::new();
        replace_owner(&mut store, "a", tags(None)).unwrap();
        let before = store.clone();
        let mut next = tags(Some(vec!["x"]));
        next.insert("other".into(), Some(vec!["y".into()]));
        assert_eq!(
            replace_owner(&mut store, "b", next),
            Err(ReplaceError::Conflict {
                tag: "example:tag".into(),
                owner: "a".into()
            })
        );
        assert_eq!(store, before);
        assert_eq!(
            replace_owner(&mut store, "", TagSnapshot::new()),
            Err(ReplaceError::EmptyOwner)
        );
    }
    #[test]
    fn independent_world_stores_and_owners_do_not_leak() {
        let mut first = OwnedSnapshots::new();
        let second = OwnedSnapshots::new();
        replace_owner(&mut first, "a", tags(Some(vec!["x"]))).unwrap();
        replace_owner(
            &mut first,
            "b",
            [("other".into(), None)].into_iter().collect(),
        )
        .unwrap();
        assert_eq!(lookup(&second, "example:tag"), Lookup::Unmanaged);
        assert_eq!(lookup(&first, "other"), Lookup::Unavailable);
    }
    #[test]
    fn malformed_external_store_fails_closed_instead_of_choosing_an_owner() {
        let store = [
            ("a".into(), tags(None)),
            ("b".into(), tags(Some(vec!["x"]))),
        ]
        .into_iter()
        .collect();
        assert_eq!(lookup(&store, "example:tag"), Lookup::Conflict);
    }
}
