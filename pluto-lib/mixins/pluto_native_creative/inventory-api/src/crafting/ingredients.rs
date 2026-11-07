//! Explicit, bounded station-ingredient parsing without consumer JSON policy.
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashSet},
    sync::RwLock,
};

/// Borrowed from the Items supplied to this load, never a cached/global view.
pub trait RegistryView {
    fn contains_item(&self, id: &str) -> bool;
    /// None is a missing tag; Some(empty) is a known empty tag. Native order
    /// and duplicates are visible, with no namespace or membership inference.
    fn ordered_tag(&self, id: &str) -> Option<&[String]>;
}
pub struct Request<'a> {
    pub value: &'a Value,
    pub registry: &'a dyn RegistryView,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    Unsupported,
    /// Ordered, unique current-registry IDs. Empty is an explicit no-match,
    /// not a request to try the native parser. Components are not represented.
    Handled(Vec<String>),
}
pub type Callback = for<'a> fn(&Request<'a>) -> Decision;
#[derive(Clone, Copy)]
pub struct Registration {
    pub id: &'static str,
    pub callback: Callback,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistrationError {
    EmptyId,
    DuplicateId,
}
/// Validation bounds apply to returned proposals, not allocations inside a
/// consumer callback. Consumers remain responsible for bounded JSON parsing.
pub const MAX_ITEMS: usize = 65_536;
pub const MAX_ID_BYTES: usize = 1_024;
pub const MAX_TOTAL_ID_BYTES: usize = 4 * 1_024 * 1_024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Invalid {
    TooManyItems,
    IdTooLong,
    TooManyBytes,
    UnknownItem,
    DuplicateItem,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Native,
    Resolved {
        owner: &'static str,
        items: Vec<String>,
    },
    Conflict {
        first: &'static str,
        second: &'static str,
    },
    Invalid {
        owner: &'static str,
        reason: Invalid,
    },
}
fn validate(items: &[String], registry: &dyn RegistryView) -> Result<(), Invalid> {
    if items.len() > MAX_ITEMS {
        return Err(Invalid::TooManyItems);
    }
    let mut bytes = 0usize;
    let mut seen = HashSet::with_capacity(items.len());
    for id in items {
        if id.len() > MAX_ID_BYTES {
            return Err(Invalid::IdTooLong);
        }
        bytes = bytes.checked_add(id.len()).ok_or(Invalid::TooManyBytes)?;
        if bytes > MAX_TOTAL_ID_BYTES {
            return Err(Invalid::TooManyBytes);
        }
        if !registry.contains_item(id) {
            return Err(Invalid::UnknownItem);
        }
        if !seen.insert(id.as_str()) {
            return Err(Invalid::DuplicateItem);
        }
    }
    Ok(())
}
struct Registry(RwLock<BTreeMap<&'static str, Callback>>);
impl Registry {
    const fn new() -> Self {
        Self(RwLock::new(BTreeMap::new()))
    }
    fn register(&self, registration: Registration) -> Result<(), RegistrationError> {
        if registration.id.is_empty() {
            return Err(RegistrationError::EmptyId);
        }
        let mut all = self.0.write().unwrap_or_else(|e| e.into_inner());
        if all.contains_key(registration.id) {
            return Err(RegistrationError::DuplicateId);
        }
        all.insert(registration.id, registration.callback);
        Ok(())
    }
    fn has_parsers(&self) -> bool {
        !self.0.read().unwrap_or_else(|e| e.into_inner()).is_empty()
    }
    fn evaluate(&self, request: &Request<'_>) -> Outcome {
        let callbacks: Vec<_> = self
            .0
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|(&id, &callback)| (id, callback))
            .collect();
        let mut first = None;
        let mut conflict = None;
        for (id, callback) in callbacks {
            if let Decision::Handled(items) = callback(request) {
                if let Some((owner, _)) = &first {
                    if conflict.is_none() {
                        conflict = Some((*owner, id));
                    }
                } else {
                    first = Some((id, items));
                }
            }
        }
        if let Some((first, second)) = conflict {
            return Outcome::Conflict { first, second };
        }
        let Some((owner, items)) = first else {
            return Outcome::Native;
        };
        match validate(&items, request.registry) {
            Ok(()) => Outcome::Resolved { owner, items },
            Err(reason) => Outcome::Invalid { owner, reason },
        }
    }
}
static PARSERS: Registry = Registry::new();
/// Register once before station tables load. Parsing is synchronous, read-only
/// and deterministic for the supplied request. Late registration neither
/// reparses an existing table nor grants persistent ownership. No unregister.
pub fn register(registration: Registration) -> Result<(), RegistrationError> {
    PARSERS.register(registration)
}
pub fn has_parsers() -> bool {
    PARSERS.has_parsers()
}
/// All callbacks run unlocked in lexical ID order. Even identical multiple
/// claims conflict; no arbitrary winning policy is selected. Conflicts/invalid
/// proposals map to the native empty ingredient, never native parser fallback.
pub fn evaluate(request: &Request<'_>) -> Outcome {
    PARSERS.evaluate(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Items;
    impl RegistryView for Items {
        fn contains_item(&self, id: &str) -> bool {
            id != "missing"
        }
        fn ordered_tag(&self, id: &str) -> Option<&[String]> {
            if id == "empty" { Some(&[]) } else { None }
        }
    }
    fn request() -> Request<'static> {
        static VALUE: Value = Value::Null;
        static ITEMS: Items = Items;
        Request {
            value: &VALUE,
            registry: &ITEMS,
        }
    }
    fn parser(_: &Request<'_>) -> Decision {
        Decision::Handled(vec!["b".into(), "a".into()])
    }
    #[test]
    fn native_and_handled_empty_differ() {
        let registry = Registry::new();
        assert!(!registry.has_parsers());
        assert_eq!(registry.evaluate(&request()), Outcome::Native);
        registry
            .register(Registration {
                id: "unrelated",
                callback: |_| Decision::Unsupported,
            })
            .unwrap();
        assert_eq!(registry.evaluate(&request()), Outcome::Native);
        registry
            .register(Registration {
                id: "empty",
                callback: |_| Decision::Handled(vec![]),
            })
            .unwrap();
        assert_eq!(
            registry.evaluate(&request()),
            Outcome::Resolved {
                owner: "empty",
                items: vec![]
            }
        );
    }
    #[test]
    fn order_is_preserved_and_missing_tags_are_not_empty() {
        let registry = Registry::new();
        registry
            .register(Registration {
                id: "parser",
                callback: |r| {
                    assert_eq!(r.registry.ordered_tag("missing"), None);
                    assert_eq!(r.registry.ordered_tag("empty"), Some([].as_slice()));
                    parser(r)
                },
            })
            .unwrap();
        assert_eq!(
            registry.evaluate(&request()),
            Outcome::Resolved {
                owner: "parser",
                items: vec!["b".into(), "a".into()]
            }
        );
    }
    #[test]
    fn duplicate_and_empty_registration_preserve_existing_policy() {
        let registry = Registry::new();
        assert_eq!(
            registry.register(Registration {
                id: "",
                callback: parser
            }),
            Err(RegistrationError::EmptyId)
        );
        registry
            .register(Registration {
                id: "parser",
                callback: parser,
            })
            .unwrap();
        assert_eq!(
            registry.register(Registration {
                id: "parser",
                callback: |_| panic!()
            }),
            Err(RegistrationError::DuplicateId)
        );
        assert!(matches!(
            registry.evaluate(&request()),
            Outcome::Resolved { .. }
        ));
    }
    #[test]
    fn claims_conflict_in_lexical_order_even_if_equal_or_empty() {
        for reverse in [false, true] {
            let registry = Registry::new();
            for id in if reverse { ["z", "a"] } else { ["a", "z"] } {
                registry
                    .register(Registration {
                        id,
                        callback: |_| Decision::Handled(vec![]),
                    })
                    .unwrap();
            }
            assert_eq!(
                registry.evaluate(&request()),
                Outcome::Conflict {
                    first: "a",
                    second: "z"
                }
            );
        }
    }
    #[test]
    fn callbacks_are_unlocked_and_the_snapshot_is_stable() {
        static REGISTRY: Registry = Registry::new();
        REGISTRY
            .register(Registration {
                id: "a",
                callback: |_| {
                    let _ = REGISTRY.register(Registration {
                        id: "b",
                        callback: parser,
                    });
                    Decision::Handled(vec![])
                },
            })
            .unwrap();
        assert_eq!(
            REGISTRY.evaluate(&request()),
            Outcome::Resolved {
                owner: "a",
                items: vec![]
            }
        );
        assert_eq!(
            REGISTRY.evaluate(&request()),
            Outcome::Conflict {
                first: "a",
                second: "b"
            }
        );
    }
    #[test]
    fn all_callbacks_run_after_conflict() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static CALLS: AtomicUsize = AtomicUsize::new(0);
        let registry = Registry::new();
        for id in ["a", "b", "c"] {
            registry
                .register(Registration {
                    id,
                    callback: |_| {
                        CALLS.fetch_add(1, Ordering::SeqCst);
                        Decision::Handled(vec![])
                    },
                })
                .unwrap();
        }
        assert!(matches!(
            registry.evaluate(&request()),
            Outcome::Conflict { .. }
        ));
        assert_eq!(CALLS.load(Ordering::SeqCst), 3);
    }
    #[test]
    fn returned_list_validation_is_bounded_and_does_not_normalize() {
        assert_eq!(
            validate(&["missing".into()], &Items),
            Err(Invalid::UnknownItem)
        );
        assert_eq!(
            validate(&["a".into(), "a".into()], &Items),
            Err(Invalid::DuplicateItem)
        );
        assert_eq!(
            validate(&vec![String::new(); MAX_ITEMS + 1], &Items),
            Err(Invalid::TooManyItems)
        );
        assert_eq!(
            validate(&["a".repeat(MAX_ID_BYTES + 1)], &Items),
            Err(Invalid::IdTooLong)
        );
        let ids: Vec<_> = (0..4097).map(|n| format!("{n:01024}")).collect();
        assert_eq!(validate(&ids, &Items), Err(Invalid::TooManyBytes));
    }
    #[test]
    fn invalid_claim_never_falls_back() {
        let registry = Registry::new();
        registry
            .register(Registration {
                id: "bad",
                callback: |_| Decision::Handled(vec!["missing".into()]),
            })
            .unwrap();
        assert_eq!(
            registry.evaluate(&request()),
            Outcome::Invalid {
                owner: "bad",
                reason: Invalid::UnknownItem
            }
        );
    }
}
