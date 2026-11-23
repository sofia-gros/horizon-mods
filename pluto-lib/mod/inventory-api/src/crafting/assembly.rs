//! Assembly providers consulted before native recipe assembly: a provider
//! may pass, deny the match, or replace the result template stack's
//! `minecraft:custom_data` component with its own compound. No inventory
//! write or recipe identity is provided; the adapter builds the stack.
use crate::advancements::predicates::ComponentValue;
use std::{collections::BTreeMap, sync::RwLock};

use super::matching::{RecipeFacts, Request};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    Pass,
    Deny,
    Assemble { custom_data: ComponentValue },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Native,
    Denied { first: &'static str },
    Assembled { owner: &'static str, custom_data: ComponentValue },
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
    /// Any denial wins. Otherwise the first assembling provider in lexical
    /// registration order supplies the result's custom data. Callbacks run
    /// unlocked, including after a prior denial or assembly; the reported
    /// ids are deterministic.
    pub fn evaluate(&self, request: &Request<'_>) -> Outcome {
        let mut denied = None;
        let mut assembled = None;
        for &(id, callback) in &self.0 {
            match callback(request) {
                Decision::Pass => {}
                Decision::Deny => {
                    if denied.is_none() {
                        denied = Some(id);
                    }
                }
                Decision::Assemble { custom_data } => {
                    if assembled.is_none() {
                        assembled = Some((id, custom_data));
                    }
                }
            }
        }
        if let Some(first) = denied {
            return Outcome::Denied { first };
        }
        assembled.map_or(Outcome::Native, |(owner, custom_data)| Outcome::Assembled {
            owner,
            custom_data,
        })
    }
}
static ASSEMBLERS: Registry = Registry::new();
/// Register once during startup. Both functions must be read-only/deterministic
/// for their arguments. They must not infer a persistent identity, capture an
/// actor or perform inventory mutations. There is no unregister/reset path.
pub fn register(entry: Registration) -> Result<(), RegistrationError> {
    ASSEMBLERS.register(entry)
}
pub fn has_providers() -> bool {
    ASSEMBLERS.has_providers()
}
/// Interest runs on cheap borrowed recipe facts. None avoids all input/result
/// component capture in the native adapter, even with unrelated registrations.
pub fn prepare(recipe: &RecipeFacts<'_>) -> Option<Evaluation> {
    ASSEMBLERS.prepare(recipe)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipe_book::placement::Form;
    fn recipe() -> RecipeFacts<'static> {
        RecipeFacts { form: Form::Special, id: None, ingredients: None, template: None }
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
    fn assemble(_: &Request<'_>) -> Decision {
        Decision::Assemble { custom_data: ComponentValue::Compound(Default::default()) }
    }
    fn request<'a>(recipe: &'a RecipeFacts<'a>) -> Request<'a> {
        Request {
            recipe,
            input: super::super::matching::InputFacts {
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
        r.register(Registration { id: "unrelated", interest: ignore, callback: assemble }).unwrap();
        assert!(r.prepare(&recipe()).is_none());
    }
    #[test]
    fn denial_wins_and_the_first_assembly_is_lexical() {
        for order in [["z", "a"], ["a", "z"]] {
            let r = Registry::new();
            r.register(Registration { id: "0", interest: inspect, callback: pass }).unwrap();
            let facts = recipe();
            assert_eq!(r.prepare(&facts).unwrap().evaluate(&request(&facts)), Outcome::Native);
            for id in order {
                r.register(Registration { id, interest: inspect, callback: assemble }).unwrap();
            }
            let Outcome::Assembled { owner, .. } = r.prepare(&facts).unwrap().evaluate(&request(&facts))
            else {
                panic!("expected an assembly");
            };
            assert_eq!(owner, "a");
            r.register(Registration { id: "m", interest: inspect, callback: deny }).unwrap();
            assert_eq!(
                r.prepare(&facts).unwrap().evaluate(&request(&facts)),
                Outcome::Denied { first: "m" }
            );
        }
    }
    #[test]
    fn duplicates_and_empty_ids_never_replace() {
        let r = Registry::new();
        assert_eq!(
            r.register(Registration { id: "", interest: inspect, callback: pass }),
            Err(RegistrationError::EmptyId)
        );
        r.register(Registration { id: "one", interest: inspect, callback: deny }).unwrap();
        assert_eq!(
            r.register(Registration { id: "one", interest: ignore, callback: pass }),
            Err(RegistrationError::DuplicateId)
        );
    }
}
