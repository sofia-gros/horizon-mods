//! Cheap-interest, deny-only admission before native recipe matching/assembly.
//! No output replacement, inventory write or recipe identity is provided.
use crate::{
    advancements::predicates::ItemSnapshot,
    recipe_book::placement::{Form, Ingredient},
};
use std::{collections::BTreeMap, sync::RwLock};

#[derive(Clone, Copy, Debug)]
pub struct TemplateFacts<'a> {
    pub item: Option<&'a str>,
    pub count: u32,
    pub damage: u32,
    pub has_components: bool,
}
pub struct RecipeFacts<'a> {
    pub form: Form,
    /// Cell order and holes are retained. None is unavailable (special recipes).
    pub ingredients: Option<&'a [Option<&'a dyn Ingredient>]>,
    /// Fixed result templates only; special assembly results are unavailable.
    pub template: Option<TemplateFacts<'a>>,
}
pub struct InputFacts<'a> {
    pub width: usize,
    pub height: usize,
    pub left: usize,
    pub top: usize,
    /// Cropped native cells, in row-major order, including empty positions.
    pub cells: &'a [Option<ItemSnapshot<'a>>],
}
pub struct Request<'a> {
    pub recipe: &'a RecipeFacts<'a>,
    pub input: InputFacts<'a>,
    /// The fixed template's native created stack, not a proposed output.
    /// Its component availability follows the existing shared read-only view.
    pub result: Option<ItemSnapshot<'a>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Pass,
    Deny,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Native,
    Denied { first: &'static str },
}
pub type Interest = for<'a> fn(&RecipeFacts<'a>) -> bool;
pub type Callback = for<'a> fn(&Request<'a>) -> Decision;
#[derive(Clone, Copy)]
pub struct Registration {
    pub id: &'static str,
    pub interest: Interest,
    pub callback: Callback,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistrationError {
    EmptyId,
    DuplicateId,
}
struct Registry(RwLock<BTreeMap<&'static str, (Interest, Callback)>>);
impl Registry {
    const fn new() -> Self {
        Self(RwLock::new(BTreeMap::new()))
    }
    fn register(&self, entry: Registration) -> Result<(), RegistrationError> {
        if entry.id.is_empty() {
            return Err(RegistrationError::EmptyId);
        }
        let mut callbacks = self.0.write().unwrap_or_else(|e| e.into_inner());
        if callbacks.contains_key(entry.id) {
            return Err(RegistrationError::DuplicateId);
        }
        callbacks.insert(entry.id, (entry.interest, entry.callback));
        Ok(())
    }
    fn has_providers(&self) -> bool {
        !self.0.read().unwrap_or_else(|e| e.into_inner()).is_empty()
    }
    fn prepare(&self, recipe: &RecipeFacts<'_>) -> Option<Evaluation> {
        let callbacks: Vec<_> = self
            .0
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|(&id, &(interest, callback))| (id, interest, callback))
            .collect();
        let selected: Vec<_> = callbacks
            .into_iter()
            .filter_map(|(id, interest, callback)| interest(recipe).then_some((id, callback)))
            .collect();
        (!selected.is_empty()).then_some(Evaluation(selected))
    }
}
/// Ephemeral callback snapshot only, not a recipe/world identity or authority
/// token. The adapter evaluates it synchronously with the same recipe facts.
pub struct Evaluation(Vec<(&'static str, Callback)>);
impl Evaluation {
    /// Any denial wins. Pass cannot grant matching or provide an output.
    /// Callbacks run unlocked in lexical registration order, including after a
    /// prior denial; the first denying ID is reported deterministically.
    pub fn evaluate(&self, request: &Request<'_>) -> Outcome {
        let mut first = None;
        for &(id, callback) in &self.0 {
            if callback(request) == Decision::Deny && first.is_none() {
                first = Some(id);
            }
        }
        first.map_or(Outcome::Native, |first| Outcome::Denied { first })
    }
}
static POLICIES: Registry = Registry::new();
/// Register once during startup. Both functions must be read-only/deterministic
/// for their arguments. They must not infer a persistent identity, capture an
/// actor or perform inventory mutations. There is no unregister/reset path.
pub fn register(entry: Registration) -> Result<(), RegistrationError> {
    POLICIES.register(entry)
}
pub fn has_providers() -> bool {
    POLICIES.has_providers()
}
/// Interest runs on cheap borrowed recipe facts. None avoids all input/result
/// component capture in the native adapter, even with unrelated registrations.
pub fn prepare(recipe: &RecipeFacts<'_>) -> Option<Evaluation> {
    POLICIES.prepare(recipe)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn recipe() -> RecipeFacts<'static> {
        RecipeFacts {
            form: Form::Special,
            ingredients: None,
            template: None,
        }
    }
    fn inspect(_: &RecipeFacts<'_>) -> bool {
        true
    }
    fn ignore(_: &RecipeFacts<'_>) -> bool {
        false
    }
    fn pass(_: &Request<'_>) -> Decision {
        Decision::Pass
    }
    fn deny(_: &Request<'_>) -> Decision {
        Decision::Deny
    }
    fn request<'a>(recipe: &'a RecipeFacts<'a>) -> Request<'a> {
        Request {
            recipe,
            input: InputFacts {
                width: 0,
                height: 0,
                left: 0,
                top: 0,
                cells: &[],
            },
            result: None,
        }
    }
    #[test]
    fn empty_and_uninterested_registries_require_no_component_request() {
        let r = Registry::new();
        assert!(!r.has_providers());
        assert!(r.prepare(&recipe()).is_none());
        r.register(Registration {
            id: "unrelated",
            interest: ignore,
            callback: |_| panic!("ignored callback"),
        })
        .unwrap();
        assert!(r.prepare(&recipe()).is_none());
    }
    #[test]
    fn native_pass_and_lexical_deny_are_independent_of_registration_order() {
        for order in [["z", "a"], ["a", "z"]] {
            let r = Registry::new();
            r.register(Registration {
                id: "0",
                interest: inspect,
                callback: pass,
            })
            .unwrap();
            let facts = recipe();
            assert_eq!(
                r.prepare(&facts).unwrap().evaluate(&request(&facts)),
                Outcome::Native
            );
            for id in order {
                r.register(Registration {
                    id,
                    interest: inspect,
                    callback: deny,
                })
                .unwrap();
            }
            assert_eq!(
                r.prepare(&facts).unwrap().evaluate(&request(&facts)),
                Outcome::Denied { first: "a" }
            );
        }
    }
    #[test]
    fn duplicates_and_empty_ids_never_replace() {
        let r = Registry::new();
        assert_eq!(
            r.register(Registration {
                id: "",
                interest: inspect,
                callback: pass
            }),
            Err(RegistrationError::EmptyId)
        );
        r.register(Registration {
            id: "one",
            interest: inspect,
            callback: deny,
        })
        .unwrap();
        assert_eq!(
            r.register(Registration {
                id: "one",
                interest: ignore,
                callback: pass
            }),
            Err(RegistrationError::DuplicateId)
        );
    }
    #[test]
    fn interest_and_evaluation_are_unlocked_with_a_stable_snapshot() {
        static R: Registry = Registry::new();
        fn interest(_: &RecipeFacts<'_>) -> bool {
            let _ = R.register(Registration {
                id: "later",
                interest: inspect,
                callback: deny,
            });
            true
        }
        fn callback(_: &Request<'_>) -> Decision {
            let _ = R.register(Registration {
                id: "last",
                interest: inspect,
                callback: deny,
            });
            Decision::Pass
        }
        R.register(Registration {
            id: "first",
            interest,
            callback,
        })
        .unwrap();
        let facts = recipe();
        let prepared = R.prepare(&facts).unwrap();
        assert_eq!(prepared.evaluate(&request(&facts)), Outcome::Native);
        assert_eq!(
            R.prepare(&facts).unwrap().evaluate(&request(&facts)),
            Outcome::Denied { first: "last" }
        );
    }
    #[test]
    fn unavailable_result_and_input_holes_remain_explicit() {
        fn check(r: &Request<'_>) -> Decision {
            assert!(r.recipe.ingredients.is_none());
            assert!(r.result.is_none());
            assert_eq!(
                (r.input.width, r.input.height, r.input.left, r.input.top),
                (2, 1, 1, 2)
            );
            assert_eq!(r.input.cells.len(), 2);
            assert!(r.input.cells[0].is_none());
            assert!(r.input.cells[1].unwrap().components.is_none());
            Decision::Deny
        }
        let r = Registry::new();
        r.register(Registration {
            id: "inspect",
            interest: inspect,
            callback: check,
        })
        .unwrap();
        let facts = recipe();
        let cells = [
            None,
            Some(ItemSnapshot {
                id: "example:item",
                count: 1,
                damage: 0,
                enchantments: &[],
                potion: None,
                custom_name: false,
                components: None,
            }),
        ];
        assert_eq!(
            r.prepare(&facts).unwrap().evaluate(&Request {
                recipe: &facts,
                input: InputFacts {
                    width: 2,
                    height: 1,
                    left: 1,
                    top: 2,
                    cells: &cells
                },
                result: None
            }),
            Outcome::Denied { first: "inspect" }
        );
    }
}
