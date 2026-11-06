//! Exact native shaped/shapeless matching and shared component facts.
//! HEAD wrappers, external ingredient hooks and special endpoints are modeled.
extern crate self as horizon_client;
pub use horizon_items::{item_data, items};
pub use pluto_inventory_api as pluto_inventory;
pub mod crafting;
pub mod recipe_book;
pub mod advancements {
    include!("item-view.rs");
}
#[path = "matching-callback.rs"]
mod consumer;
#[path = "component-view.rs"]
pub mod pluto_component_view;
#[path = "stack-storage.rs"]
pub mod pluto_stack_storage;
#[path = "matching-reference-gate.rs"]
mod reference_gate;
#[path = "matching-reference-plan.rs"]
mod reference_plan;
#[cfg(test)]
mod tests {
    use super::*;
    use crafting::{Ingredient, Input, Recipe, Recipes, Template};
    use horizon_world::nbt::Tag;
    use items::ItemStack;
    use pluto_inventory::{
        crafting::matching::{self, Decision, Outcome, Registration},
        recipe_book::placement::Form,
    };
    const MARKER: &str = "tconstruct:fake_metal_conversion";
    fn stack(id: &str, material: &str, count: u32) -> ItemStack {
        let mut stack = ItemStack::new(id, count);
        stack.data_mut().set_other(
            "minecraft:custom_data",
            Some(Tag::compound([("Material", Tag::String(material.into()))])),
        );
        stack
    }
    fn ingredient(id: &str, material: Option<&str>) -> Ingredient {
        Ingredient {
            items: [id.into()].into_iter().collect(),
            order: vec![id.into()],
            custom: material.map(|m| {
                (
                    MARKER.into(),
                    serde_json::Map::from_iter([(
                        "material".into(),
                        serde_json::Value::String(m.into()),
                    )]),
                )
            }),
            children: None,
        }
    }
    fn template(stack: ItemStack) -> Template {
        Template {
            item: Some(stack.item),
            count: stack.count,
            damage: stack.damage,
            extra: stack.extra,
        }
    }
    fn recipe() -> Recipe {
        Recipe::Shaped {
            width: 3,
            height: 3,
            cells: vec![Some(ingredient("tconstruct:fake_ingot", Some("test:iron"))); 9],
            result: template(stack("tconstruct:fake_storage_block", "test:iron", 1)),
        }
    }
    fn grid() -> Vec<Option<ItemStack>> {
        vec![Some(stack("tconstruct:fake_ingot", "test:iron", 2)); 9]
    }
    fn find(recipe: Recipe, grid: &[Option<ItemStack>], size: usize) -> Option<ItemStack> {
        let recipes = Recipes {
            crafting: vec![("example:recipe".into(), recipe)],
            cooking: vec![],
        };
        recipes
            .find_with(grid, size, &|_| None)
            .map(|(_, result)| result)
    }
    fn check_reference(recipe: Recipe, grid: Vec<Option<ItemStack>>) {
        let original = grid.clone();
        let input = Input::of(&grid, 3).unwrap();
        let expected = reference_gate::denies(&input, &recipe);
        assert_eq!(
            matches!(
                crafting::pluto_matching::evaluate(&input, &recipe),
                Outcome::Denied { .. }
            ),
            expected
        );
        let result = find(recipe, &grid, 3);
        assert_eq!(result.is_none(), expected);
        for (before, after) in original.iter().zip(&grid) {
            match (before, after) {
                (Some(b), Some(a)) => {
                    assert_eq!(b.count, a.count);
                    assert!(b.same_item_same_components(a));
                }
                (None, None) => {}
                _ => panic!("input changed"),
            }
        }
    }
    #[test]
    fn exact_native_matching_preserves_fallback_and_applies_only_consumer_vetoes() {
        // Without a registered policy the native custom ingredient endpoint is
        // unchanged. The fixture models no other native ingredient provider.
        assert!(!matching::has_providers());
        let mut mixed = grid();
        mixed[8] = Some(stack("tconstruct:fake_ingot", "test:copper", 1));
        assert!(find(recipe(), &mixed, 3).is_some());
        matching::register(Registration {
            id: "unrelated",
            interest: |_| false,
            callback: |_| panic!("uninterested callback"),
        })
        .unwrap();
        assert!(find(recipe(), &mixed, 3).is_some());
        matching::register(Registration {
            id: "tconstruct:material-test",
            interest: consumer::interest,
            callback: consumer::policy,
        })
        .unwrap();
        check_reference(recipe(), grid());
        check_reference(recipe(), mixed);
        let mut missing = grid();
        missing[5] = Some(ItemStack::new("tconstruct:fake_ingot", 1));
        check_reference(recipe(), missing);
        let mut zero = grid();
        zero[5].as_mut().unwrap().count = 0;
        check_reference(recipe(), zero);
        let mut wrong_item = grid();
        wrong_item[5] = Some(stack("example:wrong", "test:iron", 1));
        check_reference(recipe(), wrong_item);
        let mut extra_input = grid();
        extra_input[0]
            .as_mut()
            .unwrap()
            .data_mut()
            .set_other("example:note", Some(Tag::Int(7)));
        check_reference(recipe(), extra_input);
        for mode in 0..6 {
            let mut r = recipe();
            if let Recipe::Shaped {
                width,
                cells,
                result,
                ..
            } = &mut r
            {
                match mode {
                    0 => *width = 2,
                    1 => cells[0]
                        .as_mut()
                        .unwrap()
                        .custom
                        .as_mut()
                        .unwrap()
                        .1
                        .insert("material".into(), "test:wrong".into())
                        .map(|_| ())
                        .unwrap_or(()),
                    2 => {
                        cells[0].as_mut().unwrap().children = Some((
                            crafting::Shape::Compound,
                            vec![ingredient("tconstruct:fake_ingot", None)],
                        ))
                    }
                    3 => result.count = 9,
                    4 => result.damage = 1,
                    _ => result
                        .extra
                        .as_mut()
                        .unwrap()
                        .set_other("example:extra", Some(Tag::Int(1))),
                }
            }
            check_reference(r, grid());
        }
        // Source 1x1 reverse conversion in the center of a larger grid keeps
        // native cropping, position and result construction.
        let mut reverse = vec![None; 9];
        reverse[4] = Some(stack("tconstruct:fake_storage_block", "test:iron", 1));
        let r = Recipe::Shaped {
            width: 1,
            height: 1,
            cells: vec![Some(ingredient(
                "tconstruct:fake_storage_block",
                Some("test:iron"),
            ))],
            result: template(stack("tconstruct:fake_ingot", "test:iron", 9)),
        };
        check_reference(r, reverse);
        // Unavailable semantic components fail closed; no private identity is inferred.
        let mut opaque = grid();
        let s = opaque[0].as_mut().unwrap();
        let decoded = s.clone();
        s.data_mut().wire =
            horizon_items::item_data::Wire::new(decoded, std::sync::Arc::from([1u8]));
        assert!(find(recipe(), &opaque, 3).is_none());
        // Native find proceeds to a later recipe when the marked candidate is denied.
        let recipes = Recipes {
            cooking: vec![],
            crafting: vec![
                ("denied".into(), recipe()),
                (
                    "fallback".into(),
                    Recipe::Shapeless {
                        ingredients: vec![ingredient("tconstruct:fake_ingot", None); 9],
                        result: template(ItemStack::new("example:fallback", 1)),
                    },
                ),
            ],
        };
        let mut mismatched = grid();
        mismatched[0] = Some(stack("tconstruct:fake_ingot", "test:other", 1));
        assert_eq!(
            recipes.find_with(&mismatched, 3, &|_| None).unwrap().0,
            "fallback"
        );
        assert!(!recipes.recipe_matches("denied", &mismatched, 3));
        // Read-only native facts retain holes/order/offset. Passing cannot replace result.
        fn geometry(f: &matching::RecipeFacts<'_>) -> bool {
            f.template
                .is_some_and(|t| t.item == Some("example:geometry"))
        }
        fn inspect(r: &matching::Request<'_>) -> Decision {
            assert_eq!(
                (r.input.width, r.input.height, r.input.left, r.input.top),
                (2, 2, 1, 1)
            );
            assert_eq!(r.input.cells.len(), 4);
            assert_eq!(r.input.cells[0].unwrap().id, "example:a");
            assert!(r.input.cells[1].is_none() && r.input.cells[2].is_none());
            assert_eq!(r.input.cells[3].unwrap().id, "example:b");
            Decision::Pass
        }
        matching::register(Registration {
            id: "example:geometry",
            interest: geometry,
            callback: inspect,
        })
        .unwrap();
        let mut positioned = vec![None; 16];
        positioned[5] = Some(ItemStack::new("example:a", 1));
        positioned[10] = Some(ItemStack::new("example:b", 1));
        let r = Recipe::Shaped {
            width: 2,
            height: 2,
            cells: vec![
                Some(ingredient("example:a", None)),
                None,
                None,
                Some(ingredient("example:b", None)),
            ],
            result: template(ItemStack::new("example:geometry", 3)),
        };
        let result = find(r, &positioned, 4).unwrap();
        assert_eq!((&*result.item, result.count), ("example:geometry", 3));
        // Special matching itself is modeled: assert only delegation/no-delegation.
        let ordinary = [Some(ItemStack::new("example:a", 1))];
        let special = || Recipe::Special(crafting::Special::RepairItem);
        assert!(find(special(), &ordinary, 1).is_none());
        let before = crafting::SPECIAL_CALLS.load(std::sync::atomic::Ordering::SeqCst);
        assert!(before > 0);
        matching::register(Registration {
            id: "example:special",
            interest: |f| f.form == Form::Special,
            callback: |r| {
                assert!(r.result.is_none() && r.recipe.ingredients.is_none());
                Decision::Deny
            },
        })
        .unwrap();
        assert!(find(special(), &ordinary, 1).is_none());
        assert_eq!(
            crafting::SPECIAL_CALLS.load(std::sync::atomic::Ordering::SeqCst),
            before
        );
    }
}
