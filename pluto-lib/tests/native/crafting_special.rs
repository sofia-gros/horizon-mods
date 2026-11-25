//! Exact native grid lookup with the special-recipe HEAD fill: the
//! helper replays the native scan, a native match continues into the real
//! body, and only a native miss breaks with a provider's answer.
//! External ingredient hooks and special endpoints are modeled.
extern crate self as horizon_client;
pub use horizon_items::{item_data, items};
pub use pluto_inventory_api as pluto_inventory;
pub mod crafting;
pub mod recipe_book;
pub mod advancements {
    include!("item-view.rs");
}
#[path = "component-view.rs"]
pub mod pluto_component_view;
#[path = "stack-storage.rs"]
pub mod pluto_stack_storage;
#[cfg(test)]
mod tests {
    use super::*;
    use crafting::{Ingredient, Recipe, Recipes, Template};
    use horizon_world::nbt::Tag;
    use items::ItemStack;
    use pluto_inventory::crafting::special::{self, Registration, StackSpec};
    use pluto_inventory::{
        advancements::predicates::{ComponentText, ComponentValue},
        crafting::matching::InputFacts,
    };
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static PAIR_CALLS: AtomicUsize = AtomicUsize::new(0);
    static ANY_CALLS: AtomicUsize = AtomicUsize::new(0);

    fn text(value: &str) -> ComponentText {
        ComponentText(value.encode_utf16().collect())
    }

    /// A repair-shaped provider: exactly one kit and one tool in the grid.
    fn pair(input: &InputFacts<'_>) -> Option<(&'static str, StackSpec)> {
        PAIR_CALLS.fetch_add(1, Ordering::SeqCst);
        let ids: Vec<&str> = input.cells.iter().flatten().map(|cell| cell.id).collect();
        if ids.len() != 2 || !ids.contains(&"example:kit") || !ids.contains(&"example:tool") {
            return None;
        }
        let mut data = BTreeMap::new();
        data.insert(text("repaired"), ComponentValue::Byte(1));
        Some((
            "example:tables/repair",
            StackSpec {
                item: "example:tool".into(),
                count: 1,
                damage: 0,
                custom_data: Some(ComponentValue::Compound(data)),
            },
        ))
    }

    /// A provider that answers every grid; registered under a lexically
    /// later id than the pair provider, so the pair wins where it answers.
    fn any(_: &InputFacts<'_>) -> Option<(&'static str, StackSpec)> {
        ANY_CALLS.fetch_add(1, Ordering::SeqCst);
        Some((
            "z:any",
            StackSpec { item: "example:any".into(), count: 1, damage: 0, custom_data: None },
        ))
    }

    fn stack(id: &str) -> ItemStack {
        ItemStack::new(id, 1)
    }
    fn template(stack: &ItemStack) -> Template {
        Template {
            item: Some(stack.item.clone()),
            count: stack.count,
            damage: stack.damage,
            extra: stack.extra.clone(),
        }
    }
    fn ingredient(id: &str) -> Ingredient {
        Ingredient {
            items: [id.into()].into_iter().collect(),
            order: vec![id.into()],
            custom: None,
            children: None,
        }
    }
    fn no_scale(_: i32) -> Option<u8> {
        None
    }

    #[test]
    fn special_fill_only_answers_native_misses_in_lexical_order() {
        let recipes = Recipes { crafting: vec![], cooking: vec![] };
        let pair_grid = vec![Some(stack("example:kit")), Some(stack("example:tool"))];
        // No providers: the lookup stays native and finds nothing.
        assert!(!special::has_providers());
        assert!(recipes.find_with(&pair_grid, 2, &no_scale).is_none());

        special::register(Registration { id: "example:tables/repair", callback: pair }).unwrap();
        special::register(Registration { id: "z:any", callback: any }).unwrap();
        assert!(special::has_providers());

        // The pair grid is answered by the pair provider, with its result
        // stack built by the adapter (custom data component included).
        let (name, result) = recipes.find_with(&pair_grid, 2, &no_scale).expect("the pair answers");
        assert_eq!(name, "example:tables/repair");
        assert_eq!((result.item.as_ref(), result.count, result.damage), ("example:tool", 1, 0));
        let components = item_data::encode_components(&result).expect("the result has components");
        let Tag::Compound(map) = components else { panic!("expected a component compound") };
        let Some(Tag::Compound(data)) = map.get("minecraft:custom_data") else {
            panic!("expected the provider's custom data on the result");
        };
        assert_eq!(data.get("repaired"), Some(&Tag::Byte(1)));
        assert_eq!(ANY_CALLS.load(Ordering::SeqCst), 0, "the lexical-first answer wins outright");

        // A grid the pair passes on falls to the later provider.
        let other_grid = vec![Some(stack("example:stone"))];
        let (name, result) = recipes.find_with(&other_grid, 2, &no_scale).expect("the fallback answers");
        assert_eq!((name, result.item.as_ref()), ("z:any", "example:any"));

        // A native match stands: providers are never consulted.
        let native = Recipe::Shaped {
            width: 1,
            height: 1,
            cells: vec![Some(ingredient("example:kit"))],
            result: template(&stack("example:tool")),
        };
        let recipes = Recipes { crafting: vec![("example:native".into(), native)], cooking: vec![] };
        let kit_grid = vec![Some(stack("example:kit"))];
        let pair_before = PAIR_CALLS.load(Ordering::SeqCst);
        let any_before = ANY_CALLS.load(Ordering::SeqCst);
        let (name, result) = recipes.find_with(&kit_grid, 2, &no_scale).expect("the native recipe matches");
        assert_eq!(name, "example:native");
        assert_eq!(result.item.as_ref(), "example:tool");
        assert_eq!(PAIR_CALLS.load(Ordering::SeqCst), pair_before, "a native match skips the providers");
        assert_eq!(ANY_CALLS.load(Ordering::SeqCst), any_before, "a native match skips the providers");

        // An empty grid is a native miss no provider can answer (the
        // adapter's crop is empty), and stays unmatched.
        assert!(recipes.find_with(&[None, None], 2, &no_scale).is_none());
    }
}
