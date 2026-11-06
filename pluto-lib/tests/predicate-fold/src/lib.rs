//! Differential oracle only: original fold bodies with already-resolved leaf values.
use std::{cell::RefCell, rc::Rc};
struct Leaf { value: Option<bool>, index: usize, visits: Rc<RefCell<Vec<usize>>> }
impl Leaf {
    fn evaluate(&self) -> Option<bool> { self.visits.borrow_mut().push(self.index); self.value }
    fn matches(&self, _: (), _: ()) -> Option<bool> { self.evaluate() }
    fn matches_with_container(&self, _: (), _: (), _: ()) -> Option<bool> { self.evaluate() }
}
fn conditional_test(leaf: &Leaf, _: (), _: (), _: (), _: ()) -> Option<bool> { leaf.evaluate() }
fn conditions(values: &[Leaf], kind: &str) -> Option<bool> {
 let (registries, config, items, mods) = ((), (), (), ());
            let mut unknown = false;
            for child in values {
                match conditional_test(child, registries, config, items, mods) {
                    Some(result) if (kind == "forge:and" && !result) || (kind == "forge:or" && result) => return Some(result),
                    Some(_) => {}, None => unknown = true,
                }
            }
            if unknown { None } else { Some(kind == "forge:and") }
}
fn transaction_or(values: &[Leaf]) -> Option<bool> {
 let (registries, config) = ((), ());
                let mut unknown = false;
                for value in values {
                    match value.matches(registries, config) {
                        Some(true) => return Some(true),
                        Some(false) => {},
                        None => unknown = true,
                    }
                }
                if unknown { None } else { Some(false) }
}
fn transaction_and(values: &[Leaf]) -> Option<bool> {
 let (registries, config) = ((), ());
                let mut unknown = false;
                for value in values {
                    match value.matches(registries, config) {
                        Some(false) => return Some(false),
                        Some(true) => {},
                        None => unknown = true,
                    }
                }
                if unknown { None } else { Some(true) }
}
fn melting_any(choices: &[Leaf]) -> Option<bool> {
 let (item, in_tag, no_container) = ((), (), ());
let mut unknown = false; for choice in choices { match choice.matches_with_container(item, in_tag, no_container) { Some(true) => return Some(true), Some(false) => {}, None => unknown = true } } if unknown { None } else { Some(false) }
}
fn melting_all(choices: &[Leaf]) -> Option<bool> {
 let (item, in_tag, no_container) = ((), (), ());
let mut unknown = false; for choice in choices { match choice.matches_with_container(item, in_tag, no_container) { Some(false) => return Some(false), Some(true) => {}, None => unknown = true } } if unknown { None } else { Some(true) }
}

fn observe(values: &[Option<bool>], run: impl FnOnce(&[Leaf]) -> Option<bool>) -> (Option<bool>, Vec<usize>) {
    let visits = Rc::new(RefCell::new(Vec::new()));
    let leaves: Vec<_> = values.iter().enumerate().map(|(index, value)| Leaf {
        value: *value, index, visits: visits.clone(),
    }).collect();
    let result = run(&leaves);
    let trace = visits.borrow().clone();
    (result, trace)
}
#[test]
fn all_three_legacy_callers_match_result_and_visitation_exhaustively() {
    let mut checked = 0;
    for size in 0..=6 {
        for mut code in 0..3usize.pow(size) {
            let values: Vec<_> = (0..size).map(|_| {
                let value = [None, Some(false), Some(true)][code % 3]; code /= 3; value
            }).collect();
            let any = observe(&values, |v| pluto_inventory_api::predicate_fold::any(v.iter().map(Leaf::evaluate)));
            let all = observe(&values, |v| pluto_inventory_api::predicate_fold::all(v.iter().map(Leaf::evaluate)));
            for (old, expected) in [
                (observe(&values, |v| conditions(v, "forge:or")), &any),
                (observe(&values, |v| conditions(v, "forge:and")), &all),
                (observe(&values, transaction_or), &any),
                (observe(&values, transaction_and), &all),
                (observe(&values, melting_any), &any),
                (observe(&values, melting_all), &all),
            ] { assert_eq!(&old, expected, "input {values:?}"); checked += 1; }
        }
    }
    assert_eq!(checked, 6558);
}

#[test]
fn public_empty_and_unknown_contract() {
    use pluto_inventory_api::predicate_fold::{all, any};
    assert_eq!(any([]), Some(false));
    assert_eq!(all([]), Some(true));
    assert_eq!(any([None, Some(false)]), None);
    assert_eq!(all([None, Some(true)]), None);
}

#[test]
fn public_decisive_values_preserve_unknown_and_order() {
    let values = [None, Some(false), Some(true), None];
    assert_eq!(observe(&values, |v| pluto_inventory_api::predicate_fold::any(v.iter().map(Leaf::evaluate))),
               (Some(true), vec![0, 1, 2]));
    assert_eq!(observe(&values, |v| pluto_inventory_api::predicate_fold::all(v.iter().map(Leaf::evaluate))),
               (Some(false), vec![0, 1]));
}

#[test]
fn public_decisive_value_never_evaluates_tail() {
    use pluto_inventory_api::predicate_fold::{all, any};
    assert_eq!(any(std::iter::once(Some(true)).chain(std::iter::once_with(|| panic!("unvisited")))), Some(true));
    assert_eq!(all(std::iter::once(Some(false)).chain(std::iter::once_with(|| panic!("unvisited")))), Some(false));
}
