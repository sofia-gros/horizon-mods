//! Exact native declaration/layout fragments plus real ItemStack/storage code.
//! Game/menu selection and fill_ghost invocation are modeled, not UI rendering.
extern crate self as horizon_client;
pub use horizon_items::items;
pub use pluto_inventory_api as pluto_inventory;
pub mod crafting;
#[path = "stack-storage.rs"]
pub mod pluto_stack_storage;
pub mod recipe_book;
use recipe_book::{Book, BookType, Entry};
use std::marker::PhantomData;
struct RecipeUi {
    book: Option<Book>,
}
struct Game<'g> {
    remote: Option<()>,
    active: Option<BookType>,
    craft_size: usize,
    recipe_ui: RecipeUi,
    shown: Vec<(BookType, Entry)>,
    inventory: u32,
    packets: u32,
    knowledge: u32,
    lifetime: PhantomData<&'g ()>,
}
impl Game<'_> {
    fn active_book(&self) -> Option<BookType> {
        self.active
    }
    fn fill_ghost(&mut self, kind: BookType, entry: &Entry) {
        self.shown.push((kind, entry.clone()));
    }
}
#[path = "preview.rs"]
mod preview;
#[cfg(test)]
mod tests {
    use super::*;
    use crafting::{Ingredient, Shape};
    use horizon_world::nbt::Tag;
    use items::ItemStack;
    use pluto_inventory::recipe_book::preview::{
        BookKind, Error, OverlayError, Preview, Selection,
    };
    use recipe_book::{Category, Display};
    fn ingredient(id: &str) -> Ingredient {
        Ingredient {
            items: [id.to_owned()].into_iter().collect(),
            order: vec![id.into()],
            custom: None,
            children: None,
        }
    }
    fn entry() -> Entry {
        let mut input = ItemStack::new("example:input", 1);
        input
            .data_mut()
            .set_other("example:data", Some(Tag::String("variant".into())));
        Entry {
            id: "example:recipe".into(),
            display_id: 4,
            category: Category::CraftingMisc,
            group: Some("group".into()),
            display: Display::Shaped {
                width: 1,
                height: 1,
                cells: vec![Some(ingredient("example:input"))],
            },
            result: ItemStack::new("example:result", 1),
            variants: vec![],
            stacks: vec![(0, vec![input])],
        }
    }
    fn game() -> Game<'static> {
        let mut book = Book::default();
        book.entries.push(entry());
        Game {
            remote: None,
            active: Some(BookType::Crafting),
            craft_size: 3,
            recipe_ui: RecipeUi { book: Some(book) },
            shown: vec![],
            inventory: 17,
            packets: 0,
            knowledge: 6,
            lifetime: PhantomData,
        }
    }
    fn selected(game: &mut Game<'_>) -> preview::NativeSelection {
        preview::with_game(game, |p| p.select(BookKind::Crafting, 0).unwrap())
    }
    #[test]
    fn unchanged_selection_delegates_only_native_ghost_with_full_components() {
        let mut game = game();
        let selected = selected(&mut game);
        assert_eq!(selected.facts().recipe_id, "example:recipe");
        assert!(selected.facts().has_ingredient_previews);
        assert_eq!(preview::with_game(&mut game, |p| p.show(&selected)), Ok(()));
        assert_eq!(game.shown.len(), 1);
        assert_eq!(
            game.shown[0].1.stacks[0].1[0]
                .data()
                .unwrap()
                .other("example:data"),
            Some(&Tag::String("variant".into()))
        );
        assert_eq!((game.inventory, game.packets, game.knowledge), (17, 0, 6));
    }
    #[test]
    fn every_selected_entry_field_is_rechecked_before_ghost() {
        let mutations: Vec<Box<dyn Fn(&mut Entry)>> = vec![
            Box::new(|e| e.id = "example:other".into()),
            Box::new(|e| e.display_id += 1),
            Box::new(|e| e.group = None),
            Box::new(|e| e.result.count += 1),
            Box::new(|e| e.result.damage += 1),
            Box::new(|e| e.variants.push(ItemStack::new("example:other", 1))),
            Box::new(|e| e.stacks[0].1[0].count += 1),
            Box::new(|e| e.stacks[0].0 = 1),
            Box::new(|e| e.stacks.clear()),
            Box::new(|e| {
                if let Display::Shaped { cells, .. } = &mut e.display {
                    cells[0].as_mut().unwrap().custom =
                        Some(("example:condition".into(), Default::default()));
                }
            }),
            Box::new(|e| {
                if let Display::Shaped { cells, .. } = &mut e.display {
                    let i = cells[0].as_mut().unwrap();
                    i.children = Some((Shape::Difference, vec![ingredient("example:child")]));
                }
            }),
        ];
        for change in mutations {
            let mut game = game();
            let selected = selected(&mut game);
            change(&mut game.recipe_ui.book.as_mut().unwrap().entries[0]);
            assert_eq!(
                preview::with_game(&mut game, |p| p.show(&selected)),
                Err(Error::StaleSelection)
            );
            assert!(game.shown.is_empty());
            assert_eq!((game.inventory, game.packets, game.knowledge), (17, 0, 6));
        }
    }
    #[test]
    fn opaque_wire_and_runtime_bundle_changes_cannot_reuse_selection() {
        for wire in [false, true] {
            let mut game = game();
            let selected = selected(&mut game);
            let item = &mut game.recipe_ui.book.as_mut().unwrap().entries[0].stacks[0].1[0];
            if wire {
                let decoded = item.clone();
                item.data_mut().wire =
                    horizon_items::item_data::Wire::new(decoded, std::sync::Arc::from([1u8, 2]));
            } else {
                item.data_mut().bundle_selected.0 = 1;
            }
            assert_eq!(
                preview::with_game(&mut game, |p| p.show(&selected)),
                Err(Error::StaleSelection)
            );
            assert!(game.shown.is_empty());
        }
    }
    #[test]
    fn authority_menu_geometry_and_registration_fail_without_fallback() {
        let mut game = game();
        game.remote = Some(());
        assert!(matches!(
            preview::with_game(&mut game, |p| p.select(BookKind::Crafting, 0)),
            Err(Error::RemoteClient)
        ));
        game.remote = None;
        game.active = None;
        assert!(matches!(
            preview::with_game(&mut game, |p| p.select(BookKind::Crafting, 0)),
            Err(Error::InactiveBook)
        ));
        game.active = Some(BookType::Crafting);
        assert!(matches!(
            preview::with_game(&mut game, |p| p.select(BookKind::Crafting, 1)),
            Err(Error::MissingEntry)
        ));
        game.recipe_ui.book.as_mut().unwrap().entries[0].display_id = -1;
        assert!(matches!(
            preview::with_game(&mut game, |p| p.select(BookKind::Crafting, 0)),
            Err(Error::UnregisteredDisplay)
        ));
        game.recipe_ui.book.as_mut().unwrap().entries[0].display_id = 4;
        game.craft_size = 0;
        assert!(matches!(
            preview::with_game(&mut game, |p| p.select(BookKind::Crafting, 0)),
            Err(Error::DoesNotFit)
        ));
        game.craft_size = 3;
        game.recipe_ui.book.as_mut().unwrap().entries[0].category = Category::FurnaceMisc;
        assert!(matches!(
            preview::with_game(&mut game, |p| p.select(BookKind::Crafting, 0)),
            Err(Error::WrongBook)
        ));
        game.recipe_ui.book = None;
        assert!(matches!(
            preview::with_game(&mut game, |p| p.select(BookKind::Crafting, 0)),
            Err(Error::MissingBook)
        ));
        assert!(game.shown.is_empty());
    }
    #[test]
    fn changed_menu_after_selection_does_not_show_or_send() {
        let mut game = game();
        let selected = selected(&mut game);
        game.active = Some(BookType::Furnace);
        assert_eq!(
            preview::with_game(&mut game, |p| p.show(&selected)),
            Err(Error::InactiveBook)
        );
        assert!(game.shown.is_empty());
        assert_eq!(game.packets, 0);
    }
    #[test]
    fn overlays_use_native_layout_and_preserve_exact_preview_stacks() {
        let book = Book::default();
        let mut entry = entry();
        let cells = recipe_book::pluto_preview_cells::cells(&book, &entry).unwrap();
        assert_eq!((cells[0].0, cells[0].1), (1, 1));
        assert!(pluto_stack_storage::same_item_snapshot(
            &cells[0].2[0],
            &entry.stacks[0].1[0]
        ));
        entry.stacks.clear();
        let base = recipe_book::pluto_preview_cells::cells(&book, &entry).unwrap();
        assert_eq!(&*base[0].2[0].item, "example:input");
        assert!(base[0].2[0].data().is_none());
        entry.stacks = vec![(0, vec![])];
        assert!(recipe_book::pluto_preview_cells::cells(&book, &entry)
            .unwrap()
            .is_empty());
        entry.stacks.push((0, vec![]));
        assert!(matches!(
            recipe_book::pluto_preview_cells::cells(&book, &entry),
            Err(OverlayError::DuplicateIngredient(0))
        ));
        entry.stacks = vec![(1, vec![])];
        assert!(matches!(
            recipe_book::pluto_preview_cells::cells(&book, &entry),
            Err(OverlayError::MissingIngredient(1))
        ));
    }
}
