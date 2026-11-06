//! Fixture body requires exact official Book/Entry/Display/Ingredient declarations
//! and the actual horizon-items package. It must not run with semantic stand-ins.
pub use horizon_items::items;
#[path = "crafting.rs"]
pub mod crafting;
#[path = "recipe_book.rs"]
pub mod recipe_book;

#[cfg(test)]
mod tests {
    use crate::{crafting::Ingredient, items::ItemStack, recipe_book::{Book, Category, Display, Entry}};
    use horizon_world::nbt::Tag;
    use pluto_inventory_api::recipe_book::presentation::{Address, Error, Presentation, Replacement};
    use crate::recipe_book::pluto_presentation::stage;

    fn ingredient() -> Ingredient {
        Ingredient { items: ["example:input".into()].into(), order: vec!["example:input".into()],
            children: None, custom: None }
    }
    fn book() -> Book {
        let mut book = Book::default();
        for i in 0..2 {
            book.entries.push(Entry { id: format!("example:recipe_{i}"), display_id: i,
                category: Category::CraftingMisc, group: Some("unchanged".into()),
                display: Display::Shaped { width: 2, height: 2,
                    cells: vec![Some(ingredient()), None, Some(ingredient()), None] },
                result: ItemStack::new("example:output", 2),
                variants: vec![ItemStack::new("example:variant", 3)],
                stacks: vec![(0, vec![ItemStack::new("example:old", 1)])] });
            book.placements.insert(format!("example:recipe_{i}"), Default::default());
        }
        book.order.insert("example:input".into(), 17);
        book
    }
    fn address(p: &dyn Presentation<Stack = ItemStack>, i: usize) -> Address {
        let f = p.entry(i).unwrap();
        Address { index: f.index, recipe_id: f.recipe_id.into(), display_id: f.display_id,
            ingredient_count: f.ingredient_count, kind: f.kind }
    }
    fn component_stack() -> ItemStack {
        let mut s = ItemStack::new("example:shown", 4); s.damage = 7;
        s.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
            ("example:label", Tag::String("display only".into()))
        ])));
        s
    }
    #[test]
    fn full_native_payload_moves_without_clone_and_only_overrides_change() {
        let original = book(); let shown = component_stack();
        let pointer = shown.extra.as_deref().unwrap() as *const _ as usize;
        let (after, result) = stage(original, |p| {
            let entry = address(p, 0);
            assert_eq!(entry.ingredient_count, 2); // not four shaped grid cells
            p.replace(vec![Replacement { entry, alternatives: vec![(1, vec![shown]), (0, vec![])] }])
        });
        assert!(result.is_ok());
        let actual = &after.entries[0].stacks[0].1[0];
        assert_eq!(actual.extra.as_deref().unwrap() as *const _ as usize, pointer);
        assert_eq!(actual.count, 4); assert_eq!(actual.damage, 7);
        assert_eq!(after.entries[0].stacks[0].0, 1);
        assert!(after.entries[0].stacks[1].1.is_empty());
        assert_eq!(after.entries[0].result.item.as_ref(), "example:output");
        assert_eq!(after.entries[0].variants[0].item.as_ref(), "example:variant");
        assert_eq!(after.entries[0].group.as_deref(), Some("unchanged"));
        assert_eq!(after.entries[1].stacks[0].1[0].item.as_ref(), "example:old");
        assert_eq!(after.placements.len(), 2);
        assert_eq!(after.order.get("example:input"), Some(&17));
    }
    #[test]
    fn invalid_last_request_preserves_all_entries_and_returns_payloads() {
        let (after, result) = stage(book(), |p| {
            let first = address(p, 0); let second = address(p, 1);
            p.replace(vec![
                Replacement { entry: first, alternatives: vec![(0, vec![component_stack()])] },
                Replacement { entry: second, alternatives: vec![(2, vec![])] },
            ])
        });
        let Err(rejected) = result else { panic!("invalid batch accepted") };
        assert_eq!(rejected.error, Error::MissingIngredient { entry: 1, ingredient: 2 });
        assert_eq!(rejected.replacements.len(), 2);
        assert_eq!(rejected.replacements[0].alternatives[0].1[0].count, 4);
        for e in after.entries { assert_eq!(e.stacks[0].1[0].item.as_ref(), "example:old"); }
    }
    #[test]
    fn empty_batch_omitted_entry_clear_and_explicit_empty_are_distinct() {
        let (after, result) = stage(book(), |p| {
            assert!(p.replace(vec![]).is_ok());
            let entry = address(p, 0);
            p.replace(vec![Replacement { entry, alternatives: vec![] }])
        });
        assert!(result.is_ok()); assert!(after.entries[0].stacks.is_empty());
        assert!(!after.entries[1].stacks.is_empty());
    }
    #[test]
    fn furnace_override_is_rejected_without_altering_book() {
        let mut b = book(); b.entries[0].display = Display::Furnace(ingredient());
        let (after, result) = stage(b, |p| {
            let entry = address(p, 0);
            p.replace(vec![Replacement { entry, alternatives: vec![] }])
        });
        let Err(rejected) = result else { panic!("unsupported furnace accepted") };
        assert_eq!(rejected.error, Error::UnsupportedFurnace(0));
        assert!(!after.entries[0].stacks.is_empty());
    }
}
