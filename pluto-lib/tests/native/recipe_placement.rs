//! Exact native placement bodies, modeled outer state and mutation endpoints.
//! Test consumer policy lives in TConstruct and is never installed in PlutoLib.
extern crate self as horizon_client;
pub use horizon_items::items;
pub use pluto_inventory_api as pluto_inventory;
#[path = "callback.rs"]
mod consumer;
pub mod crafting;
#[path = "stack-storage.rs"]
pub mod pluto_stack_storage;
pub mod recipe_book;
#[path = "reference_marked.rs"]
mod reference;
mod menu {
    pub struct Env<'a> {
        pub recipes: &'a crate::crafting::Recipes,
    }
    #[derive(Default)]
    pub struct Menu {
        pub probes: std::cell::Cell<usize>,
        pub writes: usize,
        pub recomputes: usize,
        pub available: bool,
        pub used_max: Option<bool>,
    }
    include!("native_menu.rs");
    impl Menu {
        fn place_grid(&self) -> Option<Grid> {
            Some(Grid {
                width: 3,
                height: 3,
                inputs: vec![0],
                clear: vec![0],
            })
        }
        fn test_clear_grid(&self, _: &Env<'_>, _: &Grid) -> bool {
            self.probes.set(self.probes.get() + 1);
            true
        }
        fn stacked_contents(&self, _: &Env<'_>) -> Stacked {
            let mut stacked = Stacked::default();
            if self.available {
                stacked.account(&std::sync::Arc::from("example:input"), 64);
            }
            stacked
        }
        fn place(
            &mut self,
            _: &Env<'_>,
            _: &str,
            _: &Grid,
            _: &crate::crafting::Placement,
            _: &mut Stacked,
            max: bool,
        ) {
            self.writes += 1;
            self.used_max = Some(max);
        }
        fn clear_grid(&mut self, _: &Env<'_>, _: &Grid) {
            self.writes += 1;
        }
        fn recompute(&mut self, _: &Env<'_>) {
            self.recomputes += 1;
        }
    }
}
mod game {
    pub struct RecipeUi {
        pub book: Option<crate::recipe_book::Book>,
    }
    pub struct Game<'g> {
        pub remote: Option<()>,
        pub active: Option<crate::recipe_book::BookType>,
        pub recipe_ui: RecipeUi,
        pub craft_size: usize,
        pub creative: bool,
        pub recipes: std::sync::Arc<crate::crafting::Recipes>,
        pub menu: crate::menu::Menu,
        pub ghosts: usize,
        pub packets: usize,
        pub changed: usize,
        pub lifetime: std::marker::PhantomData<&'g ()>,
    }
    impl Game<'_> {
        fn active_book(&self) -> Option<crate::recipe_book::BookType> {
            self.active
        }
        fn fill_ghost(&mut self, _: crate::recipe_book::BookType, _: &crate::recipe_book::Entry) {
            self.ghosts += 1;
        }
        fn remote_menu_id(&self) -> i32 {
            0
        }
        fn send(&mut self, _: horizon_protocol::packets::play::Serverbound) {
            self.packets += 1;
        }
        fn menu_session(
            &mut self,
            run: impl FnOnce(&mut crate::menu::Menu, &crate::menu::Env<'_>, &mut ()) -> Option<bool>,
        ) {
            let recipes = std::sync::Arc::clone(&self.recipes);
            let env = crate::menu::Env { recipes: &recipes };
            let _ = run(&mut self.menu, &env, &mut ());
        }
        fn recipe_book_changed(&mut self) {
            self.changed += 1;
        }
    }
    pub mod recipe_book {
        use super::Game;
        use crate::{menu::PostPlace, recipe_book::BookType};
        pub mod pluto_preview {
            include!(concat!(env!("CARGO_MANIFEST_DIR"), "/preview.rs"));
        }
        pub mod pluto_placement {
            include!(concat!(env!("CARGO_MANIFEST_DIR"), "/attempt.rs"));
        }
        include!("click.rs");
        include!("native_game.rs");
        pub fn dispatch(game: &mut Game<'_>, book: BookType, index: usize, max: bool) {
            game.place_recipe(book, index, max);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crafting::{Ingredient, Recipe, Recipes, Template};
    use pluto_inventory::recipe_book::placement::{
        self, Decision, Form, Ingredient as _, Outcome, PreviewMode, Registration,
    };
    use recipe_book::{Book, BookType, Category, Display, Entry};
    const MATERIAL: &str = "tconstruct:fake_metal_conversion";
    const RECIPE: &str = "tconstruct:fake_metal_native/block/example";
    fn ingredient(marker: Option<&str>) -> Ingredient {
        Ingredient {
            items: ["example:input".into()].into_iter().collect(),
            order: vec!["example:input".into()],
            custom: marker.map(|m| {
                (
                    m.into(),
                    serde_json::from_str(r#"{"material":"iron"}"#).unwrap(),
                )
            }),
            children: None,
        }
    }
    fn recipe(ingredient: Ingredient, shaped: bool) -> Recipe {
        let result = Template {
            item: Some("example:output".into()),
            count: 1,
            damage: 0,
            extra: None,
        };
        if shaped {
            Recipe::Shaped {
                width: 1,
                height: 1,
                cells: vec![Some(ingredient)],
                result,
            }
        } else {
            Recipe::Shapeless {
                ingredients: vec![ingredient],
                result,
            }
        }
    }
    fn recipes(r: Recipe) -> Recipes {
        Recipes {
            cooking: vec![],
            crafting: vec![(RECIPE.into(), r)],
        }
    }
    fn entry(i: Ingredient) -> Entry {
        Entry {
            id: RECIPE.into(),
            display_id: 4,
            category: Category::CraftingMisc,
            group: None,
            display: Display::Shaped {
                width: 1,
                height: 1,
                cells: vec![Some(i)],
            },
            result: items::ItemStack::new("example:output", 1),
            variants: vec![],
            stacks: vec![(0, vec![items::ItemStack::new("example:input", 1)])],
        }
    }
    fn game(r: Recipes, e: Entry) -> game::Game<'static> {
        let mut book = Book::default();
        book.entries.push(e);
        game::Game {
            remote: None,
            active: Some(BookType::Crafting),
            recipe_ui: game::RecipeUi { book: Some(book) },
            craft_size: 3,
            creative: false,
            recipes: std::sync::Arc::new(r),
            menu: menu::Menu {
                available: true,
                ..Default::default()
            },
            ghosts: 0,
            packets: 0,
            changed: 0,
            lifetime: Default::default(),
        }
    }
    fn assert_untouched(g: &game::Game<'_>) {
        assert_eq!(
            (
                g.menu.writes,
                g.menu.recomputes,
                g.menu.probes.get(),
                g.packets,
                g.changed
            ),
            (0, 0, 0, 0, 0)
        );
    }
    #[test]
    fn official_bodies_and_test_consumer_close_id_only_paths_without_preview_permission() {
        // Registry starts empty; both local placement and remote send retain native behavior.
        assert!(!placement::has_providers());
        let mut vanilla = game(
            recipes(recipe(ingredient(None), true)),
            entry(ingredient(None)),
        );
        game::recipe_book::dispatch(&mut vanilla, BookType::Crafting, 0, true);
        assert_eq!(
            (
                vanilla.menu.writes,
                vanilla.menu.recomputes,
                vanilla.menu.used_max
            ),
            (1, 1, Some(true))
        );
        vanilla.remote = Some(());
        game::recipe_book::dispatch(&mut vanilla, BookType::Crafting, 0, false);
        assert_eq!(vanilla.packets, 1);
        placement::register(Registration {
            id: "tconstruct:material-test",
            callback: consumer::policy,
        })
        .unwrap();
        // Exact old marked(Recipe) reference agrees for ordinary, material, nested and shapeless cases.
        let mut nested = ingredient(None);
        nested.children = Some((crafting::Shape::Compound, vec![ingredient(Some(MATERIAL))]));
        for r in [
            recipe(ingredient(None), true),
            recipe(ingredient(Some(MATERIAL)), true),
            recipe(ingredient(Some(MATERIAL)), false),
            recipe(nested, true),
        ] {
            let marked = reference::marked(&r);
            let recipes = recipes(r);
            assert_eq!(
                matches!(
                    crafting::pluto_placement::definition(&recipes, RECIPE),
                    Outcome::Denied { .. }
                ),
                marked
            );
            assert_eq!(recipes.placement(RECIPE).is_none(), marked);
        }
        let r = recipes(recipe(ingredient(Some(MATERIAL)), true));
        let e = entry(ingredient(Some(MATERIAL)));
        assert!(recipe_book::native_book_placements(std::slice::from_ref(&e), &r).is_empty());
        // Native menu method rejects before its clear-grid probe and every mutation endpoint.
        for max in [false, true] {
            for creative in [false, true] {
                let mut menu = menu::Menu {
                    available: true,
                    ..Default::default()
                };
                assert_eq!(
                    menu.place_recipe(&menu::Env { recipes: &r }, RECIPE, max, creative, None),
                    menu::PostPlace::Nothing
                );
                assert_eq!((menu.probes.get(), menu.writes, menu.recomputes), (0, 0, 0));
            }
        }
        // Local display requests a ghost but cannot enter the native menu path.
        let mut local = game(r, e);
        game::recipe_book::dispatch(&mut local, BookType::Crafting, 0, true);
        assert_eq!(local.ghosts, 1);
        assert_untouched(&local);
        // Every presentation failure leaves the authoritative veto intact.
        for mode in 0..5 {
            let mut g = game(
                recipes(recipe(ingredient(Some(MATERIAL)), true)),
                entry(ingredient(Some(MATERIAL))),
            );
            match mode {
                0 => g.active = None,
                1 => g.craft_size = 0,
                2 => g.recipe_ui.book.as_mut().unwrap().entries[0].display_id = -1,
                3 => g.recipe_ui.book.as_mut().unwrap().entries[0].category = Category::FurnaceMisc,
                _ => g.recipe_ui.book.as_mut().unwrap().entries[0].stacks.clear(),
            }
            game::recipe_book::dispatch(&mut g, BookType::Crafting, 0, false);
            assert_eq!(g.ghosts, 0);
            assert_untouched(&g);
        }
        let mut remote = game(
            recipes(recipe(ingredient(Some(MATERIAL)), true)),
            entry(ingredient(Some(MATERIAL))),
        );
        remote.remote = Some(());
        game::recipe_book::dispatch(&mut remote, BookType::Crafting, 0, true);
        assert_eq!(remote.ghosts, 0);
        assert_untouched(&remote);
        // A remote display can omit the source marker. It remains native here;
        // the server's own canonical policy is required to reject allocation.
        let mut opaque_remote = game(
            recipes(recipe(ingredient(Some(MATERIAL)), true)),
            entry(ingredient(None)),
        );
        opaque_remote.remote = Some(());
        game::recipe_book::dispatch(&mut opaque_remote, BookType::Crafting, 0, false);
        assert_eq!(opaque_remote.packets, 1);
        assert_eq!(opaque_remote.menu.writes, 0);
        // Canonical cooking precedence and repeated minecraft prefix trimming
        // are native lookup rules, not inferred consumer namespace behavior.
        let mut collision = recipes(recipe(ingredient(Some(MATERIAL)), true));
        collision.crafting[0].0 = "collision".into();
        collision.cooking.push(crafting::Cooking {
            id: "collision".into(),
            category: "misc".into(),
            group: String::new(),
            kind: "minecraft:smelting".into(),
            ingredient: ingredient(None),
            result: items::ItemStack::new("example:output", 1),
            experience: 0.0,
            time: 200,
        });
        assert!(collision
            .placement("minecraft:minecraft:collision")
            .is_some());
        // An unknown recipe remains unknown, and unrelated normal recipes remain placeable.
        assert!(remote.recipes.placement("example:missing").is_none());
        let mut ordinary = game(
            recipes(recipe(ingredient(None), true)),
            entry(ingredient(None)),
        );
        game::recipe_book::dispatch(&mut ordinary, BookType::Crafting, 0, false);
        assert_eq!(ordinary.menu.writes, 1);
        fn silent(_: &placement::Request<'_>) -> Decision {
            Decision::Deny(PreviewMode::Silent)
        }
        placement::register(Registration {
            id: "other:silent",
            callback: silent,
        })
        .unwrap();
        let mut g = game(
            recipes(recipe(ingredient(Some(MATERIAL)), true)),
            entry(ingredient(Some(MATERIAL))),
        );
        game::recipe_book::dispatch(&mut g, BookType::Crafting, 0, false);
        assert_eq!(g.ghosts, 0);
        assert_untouched(&g);
    }
    #[test]
    fn native_ingredient_views_preserve_custom_json_and_nested_shape_without_interpretation() {
        let mut i = ingredient(Some("example:unknown"));
        i.children = Some((
            crafting::Shape::Difference,
            vec![ingredient(None), ingredient(Some("other:rule"))],
        ));
        assert_eq!(i.custom().unwrap().0, "example:unknown");
        assert_eq!(i.custom().unwrap().1["material"], "iron");
        assert_eq!(
            i.children(),
            Some((placement::IngredientShape::Difference, 2))
        );
        assert_eq!(i.child(1).unwrap().custom().unwrap().0, "other:rule");
        assert!(i.child(2).is_none());
        assert!(i.accepted_ids().contains("example:input"));
        assert_eq!(i.display_order(), &["example:input"]);
        assert!(matches!(Form::Special, Form::Special));
    }
}
