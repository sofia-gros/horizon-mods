// Preparation-time derived terrain bindings. These are facts, not draw commands
// or an authoritative content/physics registry.
use std::collections::BTreeMap;
use std::sync::{Arc, Weak};

pub const MAX_BINDINGS: usize = 16_384;
pub const MAX_TEXT_BYTES: usize = 1024 * 1024;
pub const MAX_LIVE_OWNERS: usize = 64;

/// The upper content owner supplies explicit selectors and an opaque variant.
/// This layer never interprets level names, flow, tint or a fluid family policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub group: String,
    pub variant: u32,
    pub block: String,
    pub properties: BTreeMap<String, String>,
    pub texture: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    RegistryUnavailable,
    StaleRegistry,
    LimitExceeded,
    InvalidRequest,
    DuplicateVariant { group: String, variant: u32 },
    MissingState { group: String, variant: u32 },
    ConflictingState(u32),
    MissingTexture(String),
    InvalidLayer(String),
    MissingFacts(u32),
    NativeFluid(u32),
    NotModel(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    pub group: String,
    pub variant: u32,
    pub state: u32,
    /// Actual atlas layer in the exact matching resource owners.
    pub sprite: u16,
    pub animated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Catalog {
    /// Diagnostic only; this counter is not an ownership key.
    pub resource_generation: u64,
    pub by_state: BTreeMap<u32, Binding>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Ready(Catalog),
    Unavailable(Error),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacheError {
    Poisoned,
    Unbound,
    StaleRegistry,
    ConflictingSnapshot,
    LiveOwnerBudget,
}

/// Check all declarations before any native lookup or publication.
pub fn validate(requests: &[Request]) -> Result<(), Error> {
    if requests.len() > MAX_BINDINGS {
        return Err(Error::LimitExceeded);
    }
    let mut bytes = 0usize;
    let mut variants = std::collections::BTreeSet::new();
    for request in requests {
        if request.group.is_empty()
            || request.block.is_empty()
            || request.texture.is_empty()
            || request
                .properties
                .iter()
                .any(|(k, v)| k.is_empty() || v.is_empty())
        {
            return Err(Error::InvalidRequest);
        }
        for value in [&request.group, &request.block, &request.texture]
            .into_iter()
            .chain(request.properties.iter().flat_map(|(k, v)| [k, v]))
        {
            bytes = bytes.checked_add(value.len()).ok_or(Error::LimitExceeded)?;
            if bytes > MAX_TEXT_BYTES {
                return Err(Error::LimitExceeded);
            }
        }
        if !variants.insert((&request.group, request.variant)) {
            return Err(Error::DuplicateVariant {
                group: request.group.clone(),
                variant: request.variant,
            });
        }
    }
    Ok(())
}

// Weak references retain the allocation identity without retaining host values.
// A lookup must upgrade all three and match the exact currently borrowed values.
// No bare pointer/hash/generation key or inferred record equality is used.
struct Entry<A, R, T, V> {
    assets: Weak<A>,
    render: Weak<R>,
    tables: Weak<T>,
    value: Arc<V>,
}
pub(super) struct OwnerCache<A, R, T, V> {
    entries: Vec<Entry<A, R, T, V>>,
}
impl<A, R, T, V: PartialEq> OwnerCache<A, R, T, V> {
    pub(super) const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
    fn live(entry: &Entry<A, R, T, V>) -> bool {
        entry.assets.strong_count() != 0
            && entry.render.strong_count() != 0
            && entry.tables.strong_count() != 0
    }
    fn matches(entry: &Entry<A, R, T, V>, assets: &A, render: &R, tables: &T) -> bool {
        let (Some(a), Some(r), Some(t)) = (
            entry.assets.upgrade(),
            entry.render.upgrade(),
            entry.tables.upgrade(),
        ) else {
            return false;
        };
        std::ptr::eq(a.as_ref(), assets)
            && std::ptr::eq(r.as_ref(), render)
            && std::ptr::eq(t.as_ref(), tables)
    }
    pub(super) fn publish(
        &mut self,
        assets: &Arc<A>,
        render: &Arc<R>,
        tables: &Arc<T>,
        value: V,
    ) -> Result<(), CacheError> {
        self.entries.retain(Self::live);
        if let Some(entry) = self
            .entries
            .iter()
            .find(|e| Self::matches(e, assets, render, tables))
        {
            return if *entry.value == value {
                Ok(())
            } else {
                Err(CacheError::ConflictingSnapshot)
            };
        }
        if self.entries.len() >= MAX_LIVE_OWNERS {
            return Err(CacheError::LiveOwnerBudget);
        }
        self.entries.push(Entry {
            assets: Arc::downgrade(assets),
            render: Arc::downgrade(render),
            tables: Arc::downgrade(tables),
            value: Arc::new(value),
        });
        Ok(())
    }
    pub(super) fn get(&self, assets: &A, render: &R, tables: &T) -> Option<Arc<V>> {
        self.entries
            .iter()
            .find(|entry| Self::matches(entry, assets, render, tables))
            .map(|entry| Arc::clone(&entry.value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn owners() -> (Arc<u32>, Arc<u32>, Arc<u32>) {
        (Arc::new(1), Arc::new(2), Arc::new(3))
    }
    #[test]
    fn exact_owner_triple_survives_arc_clones_not_identical_reload_values() {
        let (a, r, t) = owners();
        let mut cache = OwnerCache::new();
        cache.publish(&a, &r, &t, 7).unwrap();
        assert_eq!(*cache.get(&a.clone(), &r.clone(), &t.clone()).unwrap(), 7);
        let (aa, rr, tt) = owners();
        assert!(cache.get(&aa, &r, &t).is_none());
        assert!(cache.get(&a, &rr, &t).is_none());
        assert!(cache.get(&a, &r, &tt).is_none());
        cache.publish(&aa, &rr, &tt, 8).unwrap();
        assert_eq!(*cache.get(&aa, &rr, &tt).unwrap(), 8);
        assert_eq!(*cache.get(&a, &r, &t).unwrap(), 7);
    }
    #[test]
    fn same_owner_snapshot_cannot_be_changed_by_late_registration() {
        let (a, r, t) = owners();
        let mut cache = OwnerCache::new();
        cache.publish(&a, &r, &t, Vec::<u32>::new()).unwrap();
        assert_eq!(
            cache.publish(&a, &r, &t, vec![1]),
            Err(CacheError::ConflictingSnapshot)
        );
        assert!(cache.get(&a, &r, &t).unwrap().is_empty());
        assert_eq!(cache.publish(&a, &r, &t, vec![]), Ok(()));
    }
    #[test]
    fn weak_cache_does_not_retain_values_and_expired_entries_are_pruned() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        struct Owner(Arc<AtomicUsize>);
        impl Drop for Owner {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::Relaxed);
            }
        }
        let drops = Arc::new(AtomicUsize::new(0));
        let a = Arc::new(Owner(drops.clone()));
        let r = Arc::new(2);
        let t = Arc::new(3);
        let mut cache = OwnerCache::new();
        cache.publish(&a, &r, &t, 7).unwrap();
        drop(a);
        assert_eq!(drops.load(Ordering::Relaxed), 1);
        let a = Arc::new(Owner(drops.clone()));
        assert!(cache.get(&a, &r, &t).is_none());
        cache.publish(&a, &r, &t, 8).unwrap();
        assert_eq!(cache.entries.len(), 1);
    }
    #[test]
    fn live_owner_budget_never_evicts_an_existing_resource_set() {
        let mut cache = OwnerCache::new();
        let mut retained = Vec::new();
        for n in 0..MAX_LIVE_OWNERS {
            let (a, r, t) = owners();
            cache.publish(&a, &r, &t, n).unwrap();
            retained.push((a, r, t));
        }
        let (a, r, t) = owners();
        assert_eq!(
            cache.publish(&a, &r, &t, 99),
            Err(CacheError::LiveOwnerBudget)
        );
        for (n, (a, r, t)) in retained.iter().enumerate() {
            assert_eq!(*cache.get(a, r, t).unwrap(), n);
        }
    }
    #[test]
    fn snapshot_failure_is_not_an_empty_catalog() {
        let (a, r, t) = owners();
        let mut cache = OwnerCache::new();
        cache
            .publish(&a, &r, &t, Status::Unavailable(Error::RegistryUnavailable))
            .unwrap();
        assert_eq!(
            *cache.get(&a, &r, &t).unwrap(),
            Status::Unavailable(Error::RegistryUnavailable)
        );
    }
    #[test]
    fn request_validation_precedes_native_resolution() {
        let request = Request {
            group: "example:fluid".into(),
            variant: 0,
            block: "example:fluid_block".into(),
            properties: BTreeMap::new(),
            texture: "example:fluid_texture".into(),
        };
        assert_eq!(validate(&[]), Ok(()));
        assert_eq!(validate(&[request.clone()]), Ok(()));
        assert!(matches!(
            validate(&[request.clone(), request.clone()]),
            Err(Error::DuplicateVariant { .. })
        ));
        let mut invalid = request;
        invalid.properties.insert("".into(), "0".into());
        assert_eq!(validate(&[invalid]), Err(Error::InvalidRequest));
    }
}

/// Neutral upper-owner validity check. It must be cheap, monotonic and read-only.
/// The renderer does not import the authoritative registry's crate.
pub type CurrentRevision = Arc<dyn Fn() -> bool + Send + Sync>;

// Run only after releasing the native cache lock. A changed revision during
// read-only inspection discards the callback result; no rollback is implied.
pub(super) fn inspect<R>(
    status: &Status,
    revision: Option<&CurrentRevision>,
    callback: impl FnOnce(&Status) -> R,
) -> Result<R, CacheError> {
    if revision.is_some_and(|current| !current()) {
        return Ok(callback(&Status::Unavailable(Error::StaleRegistry)));
    }
    let result = callback(status);
    if revision.is_some_and(|current| !current()) {
        return Err(CacheError::StaleRegistry);
    }
    Ok(result)
}
#[cfg(test)]
mod revision_view_tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    #[test]
    fn stale_before_inspection_is_explicitly_unavailable() {
        let current: CurrentRevision = Arc::new(|| false);
        let ready = Status::Ready(Catalog {
            resource_generation: 1,
            by_state: BTreeMap::new(),
        });
        assert_eq!(
            inspect(&ready, Some(&current), Clone::clone),
            Ok(Status::Unavailable(Error::StaleRegistry))
        );
    }
    #[test]
    fn revision_changed_during_inspection_discards_return_value() {
        let live = Arc::new(AtomicBool::new(true));
        let read = live.clone();
        let current: CurrentRevision = Arc::new(move || read.load(Ordering::Acquire));
        let ready = Status::Ready(Catalog {
            resource_generation: 1,
            by_state: BTreeMap::new(),
        });
        assert_eq!(
            inspect(&ready, Some(&current), |_| {
                live.store(false, Ordering::Release);
                42
            }),
            Err(CacheError::StaleRegistry)
        );
    }
}
