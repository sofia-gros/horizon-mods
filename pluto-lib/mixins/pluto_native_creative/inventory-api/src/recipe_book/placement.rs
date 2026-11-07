//! Read-only, veto-only policy for native item-ID recipe placement.
//!
//! Register at startup before books are built or placement is attempted. Pass is
//! not permission: native checks and every other veto still apply. No recipe
//! identity, ownership, component schema or eligibility policy is inferred here.
use std::collections::{BTreeMap, HashSet};
use std::sync::RwLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IngredientShape {
    Compound,
    Intersection,
    Difference,
}
/// Borrowed native facts; no material key or custom ingredient type is interpreted.
pub trait Ingredient {
    fn accepted_ids(&self) -> &HashSet<String>;
    fn display_order(&self) -> &[String];
    fn custom(&self) -> Option<(&str, &serde_json::Map<String, serde_json::Value>)>;
    fn children(&self) -> Option<(IngredientShape, usize)>;
    fn child(&self, index: usize) -> Option<&dyn Ingredient>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    Shaped { width: usize, height: usize },
    Shapeless,
    Cooking,
    Special,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Definition,
    LocalAttempt,
    RemoteAttempt,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplayFacts {
    pub book: super::preview::BookKind,
    pub display_id: i32,
    pub has_ingredient_previews: bool,
}
pub struct Request<'a> {
    /// Exact caller spelling and exact matched native record spelling.
    pub requested_id: &'a str,
    pub resolved_id: &'a str,
    pub phase: Phase,
    pub form: Form,
    /// Native cell order, preserving holes. None means unavailable, not empty.
    /// Special recipe definitions are currently unavailable; no flattening of
    /// their custom matching semantics is attempted.
    pub ingredients: Option<&'a [Option<&'a dyn Ingredient>]>,
    pub display: Option<DisplayFacts>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewMode {
    Silent,
    IfLocal,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Pass,
    Deny(PreviewMode),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Native,
    /// First denying registration in lexical order. Silent wins if any denying
    /// provider requests it. Preview failure never reverses this denial.
    Denied {
        first: &'static str,
        preview: PreviewMode,
    },
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
struct Registry(RwLock<BTreeMap<&'static str, Callback>>);
impl Registry {
    const fn new() -> Self {
        Self(RwLock::new(BTreeMap::new()))
    }
    fn register(&self, registration: Registration) -> Result<(), RegistrationError> {
        if registration.id.is_empty() {
            return Err(RegistrationError::EmptyId);
        }
        let mut callbacks = self.0.write().unwrap_or_else(|e| e.into_inner());
        if callbacks.contains_key(registration.id) {
            return Err(RegistrationError::DuplicateId);
        }
        callbacks.insert(registration.id, registration.callback);
        Ok(())
    }
    fn has_providers(&self) -> bool {
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
        let mut result = Outcome::Native;
        for (id, callback) in callbacks {
            if let Decision::Deny(preview) = callback(request) {
                result = match result {
                    Outcome::Native => Outcome::Denied { first: id, preview },
                    Outcome::Denied {
                        first,
                        preview: old,
                    } => Outcome::Denied {
                        first,
                        preview: if old == PreviewMode::Silent || preview == PreviewMode::Silent {
                            PreviewMode::Silent
                        } else {
                            PreviewMode::IfLocal
                        },
                    },
                };
            }
        }
        result
    }
}
static POLICIES: Registry = Registry::new();
/// One process-lifetime registry. Duplicate IDs never replace a provider.
/// Callbacks must be deterministic/read-only and may not depend on the selected
/// actor, previous callback calls, cached world identity or mutable inventory.
/// Registration must precede book construction; it does not invalidate existing
/// UI caches. No unregister or runtime activation contract is provided.
pub fn register(registration: Registration) -> Result<(), RegistrationError> {
    POLICIES.register(registration)
}
pub fn has_providers() -> bool {
    POLICIES.has_providers()
}
/// Callbacks run unlocked against one snapshot, in lexical registration order.
pub fn evaluate(request: &Request<'_>) -> Outcome {
    POLICIES.evaluate(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> Request<'static> {
        Request {
            requested_id: "example:recipe",
            resolved_id: "example:recipe",
            phase: Phase::Definition,
            form: Form::Special,
            ingredients: None,
            display: None,
        }
    }
    fn pass(_: &Request<'_>) -> Decision {
        Decision::Pass
    }
    fn preview(_: &Request<'_>) -> Decision {
        Decision::Deny(PreviewMode::IfLocal)
    }
    fn silent(_: &Request<'_>) -> Decision {
        Decision::Deny(PreviewMode::Silent)
    }
    #[test]
    fn no_provider_or_pass_preserves_native_and_never_grants() {
        let registry = Registry::new();
        assert!(!registry.has_providers());
        assert_eq!(registry.evaluate(&request()), Outcome::Native);
        registry
            .register(Registration {
                id: "one",
                callback: pass,
            })
            .unwrap();
        assert_eq!(registry.evaluate(&request()), Outcome::Native);
    }
    #[test]
    fn deny_wins_in_lexical_order_and_silent_dominates() {
        for order in [["z", "a"], ["a", "z"]] {
            let registry = Registry::new();
            for id in order {
                registry
                    .register(Registration {
                        id,
                        callback: if id == "a" { preview } else { silent },
                    })
                    .unwrap();
            }
            registry
                .register(Registration {
                    id: "0",
                    callback: pass,
                })
                .unwrap();
            assert_eq!(
                registry.evaluate(&request()),
                Outcome::Denied {
                    first: "a",
                    preview: PreviewMode::Silent
                }
            );
        }
    }
    #[test]
    fn duplicates_and_empty_ids_do_not_replace() {
        let registry = Registry::new();
        assert_eq!(
            registry.register(Registration {
                id: "",
                callback: pass
            }),
            Err(RegistrationError::EmptyId)
        );
        registry
            .register(Registration {
                id: "one",
                callback: silent,
            })
            .unwrap();
        assert_eq!(
            registry.register(Registration {
                id: "one",
                callback: pass
            }),
            Err(RegistrationError::DuplicateId)
        );
        assert_eq!(
            registry.evaluate(&request()),
            Outcome::Denied {
                first: "one",
                preview: PreviewMode::Silent
            }
        );
    }
    #[test]
    fn unavailable_ingredients_and_origin_are_not_silently_normalized() {
        fn inspect(r: &Request<'_>) -> Decision {
            assert!(r.ingredients.is_none());
            assert_eq!(r.phase, Phase::Definition);
            assert_eq!(r.requested_id, "example:recipe");
            assert_eq!(r.resolved_id, "example:recipe");
            Decision::Pass
        }
        let registry = Registry::new();
        registry
            .register(Registration {
                id: "inspect",
                callback: inspect,
            })
            .unwrap();
        assert_eq!(registry.evaluate(&request()), Outcome::Native);
    }
    #[test]
    fn callbacks_run_without_registry_lock() {
        static REGISTRY: Registry = Registry::new();
        fn reenter(_: &Request<'_>) -> Decision {
            let _ = REGISTRY.register(Registration {
                id: "later",
                callback: silent,
            });
            Decision::Pass
        }
        REGISTRY
            .register(Registration {
                id: "first",
                callback: reenter,
            })
            .unwrap();
        assert_eq!(REGISTRY.evaluate(&request()), Outcome::Native);
        assert_eq!(
            REGISTRY.evaluate(&request()),
            Outcome::Denied {
                first: "later",
                preview: PreviewMode::Silent
            }
        );
    }
}
