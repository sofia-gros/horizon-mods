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
/// A finite borrowed view of an already stored value, not an effective patch.
/// Compound/List preserve type and presence; no unbounded clone or iteration is
/// provided. To inspect a nested compound key, query the reader with that path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoredValue<'a> {
    End, Byte(i8), Short(i16), Int(i32), Long(i64), Float(u32), Double(u64),
    ByteArray(&'a [u8]), String(&'a [u16]), List, Compound,
    IntArray(&'a [i32]), LongArray(&'a [i64]),
}
impl<'a> StoredValue<'a> {
    pub fn string_units(self) -> Option<&'a [u16]> {
        if let Self::String(units) = self { Some(units) } else { None }
    }
}
/// Reader over the same stacks used for this synchronous matching invocation.
/// An input index refers to the request's cropped cells, retaining holes.
/// Each path step is the native compound get operation. None is absent at that
/// step (including a non-compound parent), never an opaque/unavailable capture.
/// Query a shorter path to distinguish a malformed parent from a missing one.
/// Stored values remain queryable when semantic components are unavailable.
pub trait StoredComponents {
    fn input_other(&self, cell: usize, id: &str, path: &[&str]) -> Option<StoredValue<'_>>;
    fn result_other(&self, id: &str, path: &[&str]) -> Option<StoredValue<'_>>;
}
pub struct StoredRequest<'a> {
    pub matching: &'a Request<'a>,
    pub stored: &'a dyn StoredComponents,
}
pub type StoredCallback = for<'a> fn(&StoredRequest<'a>) -> Decision;
#[derive(Clone, Copy)]
pub struct StoredRegistration {
    pub id: &'static str,
    pub interest: Interest,
    pub callback: StoredCallback,
}
#[derive(Clone, Copy)]
enum Entry {
    Legacy(Callback),
    Stored(StoredCallback),
}
struct Registry(RwLock<BTreeMap<&'static str, (Interest, Entry)>>);
impl Registry {
    const fn new() -> Self {
        Self(RwLock::new(BTreeMap::new()))
    }
    fn register(&self, entry: Registration) -> Result<(), RegistrationError> {
        self.insert(entry.id, entry.interest, Entry::Legacy(entry.callback))
    }
    fn register_stored(&self, entry: StoredRegistration) -> Result<(), RegistrationError> {
        self.insert(entry.id, entry.interest, Entry::Stored(entry.callback))
    }
    fn insert(&self, id: &'static str, interest: Interest, entry: Entry) -> Result<(), RegistrationError> {
        if id.is_empty() {
            return Err(RegistrationError::EmptyId);
        }
        let mut callbacks = self.0.write().unwrap_or_else(|e| e.into_inner());
        if callbacks.contains_key(id) {
            return Err(RegistrationError::DuplicateId);
        }
        callbacks.insert(id, (interest, entry));
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
pub struct Evaluation(Vec<(&'static str, Entry)>);
impl Evaluation {
    /// Any denial wins. Pass cannot grant matching or provide an output.
    /// Callbacks run unlocked in lexical registration order, including after a
    /// prior denial; the first denying ID is reported deterministically.
    /// Legacy registrations behave unchanged. A selected stored-value
    /// registration is denied without calling it when its capability is absent.
    pub fn evaluate(&self, request: &Request<'_>) -> Outcome {
        self.evaluate_inner(request, None)
    }
    /// Supply an exact live reader, never a reader reconstructed from the
    /// semantic ItemSnapshot or an unrelated grid/result.
    pub fn evaluate_with_stored<'a>(&self, request: &'a Request<'a>, stored: &'a dyn StoredComponents) -> Outcome {
        self.evaluate_inner(request, Some(stored))
    }
    fn evaluate_inner<'a>(&self, request: &'a Request<'a>, stored: Option<&'a dyn StoredComponents>) -> Outcome {
        let mut first = None;
        for &(id, entry) in &self.0 {
            let decision = match entry {
                Entry::Legacy(callback) => callback(request),
                Entry::Stored(callback) => stored.map_or(Decision::Deny, |stored| {
                    callback(&StoredRequest { matching: request, stored })
                }),
            };
            if decision == Decision::Deny && first.is_none() {
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
/// Opt in to live stored-value inspection. Shares IDs, order, snapshots and the
/// one registry with legacy registrations. Startup/read-only rules are identical.
pub fn register_stored(entry: StoredRegistration) -> Result<(), RegistrationError> {
    POLICIES.register_stored(entry)
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

#[cfg(test)]
mod stored_tests {
    use super::*;
    fn facts() -> RecipeFacts<'static> { RecipeFacts { form: Form::Special, ingredients: None, template: None } }
    fn request<'a>(recipe: &'a RecipeFacts<'a>) -> Request<'a> {
        Request { recipe, input: InputFacts { width: 0, height: 0, left: 0, top: 0, cells: &[] }, result: None }
    }
    fn yes(_: &RecipeFacts<'_>) -> bool { true }
    fn no(_: &RecipeFacts<'_>) -> bool { false }
    fn pass(_: &Request<'_>) -> Decision { Decision::Pass }
    fn stored_pass(_: &StoredRequest<'_>) -> Decision { Decision::Pass }
    struct Reader;
    impl StoredComponents for Reader {
        fn input_other(&self, cell: usize, id: &str, path: &[&str]) -> Option<StoredValue<'_>> {
            match (cell, id, path) {
                (1, "example:stored", []) => Some(StoredValue::Compound),
                (1, "example:stored", ["value"]) => Some(StoredValue::String(&[0xd800, 0x61])),
                _ => None,
            }
        }
        fn result_other(&self, id: &str, path: &[&str]) -> Option<StoredValue<'_>> {
            (id == "example:stored" && path.is_empty()).then_some(StoredValue::Int(0))
        }
    }
    #[test]
    fn missing_stored_capability_denies_without_invoking_callback() {
        let r = Registry::new();
        r.register_stored(StoredRegistration { id: "needs-values", interest: yes,
            callback: |_| panic!("missing capability must not invoke callback") }).unwrap();
        let facts = facts();
        assert_eq!(r.prepare(&facts).unwrap().evaluate(&request(&facts)), Outcome::Denied { first: "needs-values" });
    }
    #[test]
    fn both_variants_share_ids_and_do_not_replace() {
        let r = Registry::new();
        r.register(Registration { id: "same", interest: yes, callback: pass }).unwrap();
        assert_eq!(r.register_stored(StoredRegistration { id: "same", interest: yes, callback: stored_pass }), Err(RegistrationError::DuplicateId));
        assert_eq!(r.register_stored(StoredRegistration { id: "", interest: yes, callback: stored_pass }), Err(RegistrationError::EmptyId));
        let r = Registry::new();
        r.register_stored(StoredRegistration { id: "same", interest: yes, callback: stored_pass }).unwrap();
        assert_eq!(r.register(Registration { id: "same", interest: yes, callback: pass }), Err(RegistrationError::DuplicateId));
    }
    #[test]
    fn mixed_variants_use_one_lexical_order_and_deny_wins() {
        use std::sync::Mutex;
        static VISITS: Mutex<Vec<&str>> = Mutex::new(Vec::new());
        let r = Registry::new();
        r.register_stored(StoredRegistration { id: "z", interest: yes, callback: |_| { VISITS.lock().unwrap().push("z"); Decision::Pass } }).unwrap();
        r.register(Registration { id: "a", interest: yes, callback: |_| { VISITS.lock().unwrap().push("a"); Decision::Deny } }).unwrap();
        r.register_stored(StoredRegistration { id: "b", interest: yes, callback: |_| { VISITS.lock().unwrap().push("b"); Decision::Deny } }).unwrap();
        let facts = facts();
        assert_eq!(r.prepare(&facts).unwrap().evaluate_with_stored(&request(&facts), &Reader), Outcome::Denied { first: "a" });
        assert_eq!(*VISITS.lock().unwrap(), ["a", "b", "z"]);
    }
    #[test]
    fn uninterested_stored_registration_requires_no_reader() {
        let r = Registry::new();
        r.register_stored(StoredRegistration { id: "unused", interest: no, callback: |_| panic!("uninterested") }).unwrap();
        assert!(r.prepare(&facts()).is_none());
        r.register(Registration { id: "legacy", interest: yes, callback: pass }).unwrap();
        let facts = facts();
        assert_eq!(r.prepare(&facts).unwrap().evaluate(&request(&facts)), Outcome::Native);
    }
    #[test]
    fn stored_queries_do_not_confuse_missing_type_zero_or_utf16() {
        fn check(r: &StoredRequest<'_>) -> Decision {
            assert_eq!(r.stored.input_other(0, "example:stored", &[]), None);
            assert_eq!(r.stored.input_other(1, "example:stored", &[]), Some(StoredValue::Compound));
            assert_eq!(r.stored.input_other(1, "example:stored", &["value"]).and_then(StoredValue::string_units), Some(&[0xd800, 0x61][..]));
            assert_eq!(r.stored.result_other("example:stored", &[]), Some(StoredValue::Int(0)));
            assert_eq!(r.stored.result_other("example:stored", &["not-a-compound"]), None);
            Decision::Pass
        }
        let r = Registry::new();
        r.register_stored(StoredRegistration { id: "read", interest: yes, callback: check }).unwrap();
        let facts = facts();
        assert_eq!(r.prepare(&facts).unwrap().evaluate_with_stored(&request(&facts), &Reader), Outcome::Native);
    }
    #[test]
    fn stored_interest_and_callback_are_unlocked_and_snapshot_is_stable() {
        static R: Registry = Registry::new();
        fn interest(_: &RecipeFacts<'_>) -> bool {
            let _ = R.register(Registration { id: "later", interest: yes, callback: |_| Decision::Deny });
            true
        }
        fn callback(_: &StoredRequest<'_>) -> Decision {
            let _ = R.register_stored(StoredRegistration { id: "last", interest: yes, callback: |_| Decision::Deny });
            Decision::Pass
        }
        R.register_stored(StoredRegistration { id: "first", interest, callback }).unwrap();
        let facts = facts();
        assert_eq!(R.prepare(&facts).unwrap().evaluate_with_stored(&request(&facts), &Reader), Outcome::Native);
        assert_eq!(R.prepare(&facts).unwrap().evaluate_with_stored(&request(&facts), &Reader), Outcome::Denied { first: "last" });
    }
}
