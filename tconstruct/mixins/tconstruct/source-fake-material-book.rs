//! Index/display support for the exact fake-metal projections, not generic
//! metadata-aware recipe placement or source advancement discovery.
use super::{Book, Known, RecipeIndex};
use crate::{crafting::Recipe, items::ItemStack};
use horizon_world::nbt::Tag;

const PREFIX: &str = "tconstruct:fake_metal_native/";
const MARKER: &str = "tconstruct:fake_metal_conversion";

pub fn marked(recipe: &Recipe) -> bool {
    matches!(recipe, Recipe::Shaped { cells, .. } if cells.iter().flatten().any(|i|
        i.custom.as_ref().is_some_and(|(kind, _)| kind == MARKER)))
}

// Only the generator's complete, self-consistent records may reserve displays.
fn display_stacks(id: &str, recipe: &Recipe) -> Option<(ItemStack, ItemStack)> {
    let Recipe::Shaped { width, height, cells, result } = recipe else { return None; };
    let result = result.create();
    let (direction, input, side, count) = match &*result.item {
        "tconstruct:fake_storage_block" => ("block", "tconstruct:fake_ingot", 3, 1),
        "tconstruct:fake_ingot" => ("ingots", "tconstruct:fake_storage_block", 1, 9),
        _ => return None,
    };
    let Tag::String(material) = result.data()?.other("minecraft:custom_data")?.get("Material")? else { return None; };
    let material = material.to_string_lossy();
    let hex: String = material.bytes().map(|b| format!("{b:02x}")).collect();
    if material.is_empty() || material.len() > 256 || material.contains('#')
        || id != format!("{PREFIX}{direction}/{hex}") || result.count != count
        || *width != side || *height != side || cells.len() != side * side
        || cells.iter().any(|cell| !cell.as_ref().is_some_and(|i|
            i.items.len() == 1 && i.items.contains(input) && i.children.is_none()
            && i.custom.as_ref().is_some_and(|(kind, data)| kind == MARKER && data.len() == 1
                && data.get("material").and_then(serde_json::Value::as_str) == Some(material.as_str())))) {
        return None;
    }
    let mut ingredient = ItemStack::new(input, 1);
    ingredient.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
        ("Material", Tag::String(material.as_str().into()))
    ])));
    Some((ingredient, result))
}

/// Called only after authoritative world-local projection. Preserve every
/// unrelated display number; assign the sorted synthetic records after them.
/// The reserved namespace is rebuilt, so invalid snapshots remove stale IDs.
pub fn replace(index: &mut RecipeIndex, crafting: &[(String, Recipe)]) {
    index.recipes.retain(|id, _| !id.starts_with(PREFIX));
    let Some(mut next) = index.recipes.values().try_fold(0i32, |end, known| {
        i32::try_from(known.displays).ok()
            .and_then(|n| known.first_display.checked_add(n)).map(|n| end.max(n))
    }) else { return; };
    let generated: std::collections::BTreeMap<_, _> = crafting.iter()
        .filter_map(|(id, recipe)| display_stacks(id, recipe).map(|(_, output)| (id, output)))
        .collect();
    for (id, output) in generated {
        let Some(after) = next.checked_add(1) else { break; };
        index.recipes.insert(id.clone(), Known {
            special: false, displays: 1, first_display: next,
            station: "minecraft:crafting_table", notify: true,
            result: Some(output.item.to_string()),
        });
        next = after;
    }
}

pub fn decorate(book: &mut Book, crafting: &[(String, Recipe)]) {
    for entry in &mut book.entries {
        if !entry.id.starts_with(PREFIX) { continue; }
        let Some((_, recipe)) = crafting.iter().find(|(id, _)| id == &entry.id) else { continue; };
        let Some((ingredient, _)) = display_stacks(&entry.id, recipe) else { continue; };
        // A naked fake item cannot communicate the required material. Native
        // ghost/overlay ingredient rendering reads these exact stacks.
        entry.stacks = (0..entry.requirements().len())
            .map(|i| (i, vec![ingredient.clone()])).collect();
        // Item-ID-only stock counting would claim mixed materials craft this.
        // Keep it visible in the unfiltered book, but not the craftable filter.
        book.placements.remove(&entry.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crafting::{Ingredient, Template};

    fn conversion(material: &str, block: bool) -> (String, Recipe) {
        let (direction, input, output, side, count) = if block {
            ("block", "tconstruct:fake_ingot", "tconstruct:fake_storage_block", 3, 1)
        } else { ("ingots", "tconstruct:fake_storage_block", "tconstruct:fake_ingot", 1, 9) };
        let mut stack = ItemStack::new(output, count);
        stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
            ("Material", Tag::String(material.into()))
        ])));
        let cells = (0..side * side).map(|_| {
            let mut i = Ingredient::of(vec![input.into()]);
            i.custom = Some((MARKER.into(), serde_json::Map::from_iter([
                ("material".into(), serde_json::Value::String(material.into()))
            ])));
            Some(i)
        }).collect();
        let hex: String = material.bytes().map(|b| format!("{b:02x}")).collect();
        (format!("{PREFIX}{direction}/{hex}"), Recipe::Shaped {
            width: side, height: side, cells,
            result: Template { item: Some(stack.item), count: stack.count, damage: stack.damage, extra: stack.extra },
        })
    }

    #[test]
    fn exact_records_keep_material_previews_and_reject_spoofed_ids() {
        for block in [false, true] {
            let (id, recipe) = conversion("tconstruct:bronze", block);
            let (input, output) = display_stacks(&id, &recipe).unwrap();
            assert!(marked(&recipe));
            assert_eq!(input.count, 1);
            assert_eq!(output.count, if block { 1 } else { 9 });
            assert_eq!(input.data().unwrap().other("minecraft:custom_data"),
                output.data().unwrap().other("minecraft:custom_data"));
            assert!(display_stacks("tconstruct:other", &recipe).is_none());
            let (wrong_id, _) = conversion("tconstruct:invar", block);
            assert!(display_stacks(&wrong_id, &recipe).is_none());
        }
    }

    #[test]
    fn refresh_is_world_local_and_removes_stale_displays() {
        let mut index = RecipeIndex::default();
        index.recipes.insert("other:recipe".into(), Known {
            special: false, displays: 2, first_display: 8,
            station: "minecraft:furnace", notify: false, result: None,
        });
        let mut recipes = Vec::new();
        let (id, recipe) = conversion("tconstruct:bronze", true);
        recipes.push((id.clone(), recipe));
        replace(&mut index, &recipes);
        assert_eq!(index.display_ids("other:recipe"), 8..10);
        assert_eq!(index.display_ids(&id), 10..11);
        assert_eq!(index.recipe_of_display(10), Some(id.as_str()));
        assert!(!RecipeIndex::default().contains(&id));
        replace(&mut index, &recipes);
        assert_eq!(index.display_ids(&id), 10..11);
        recipes.clear();
        replace(&mut index, &recipes);
        assert!(!index.contains(&id));
        assert_eq!(index.display_ids("other:recipe"), 8..10);
    }

    #[test]
    fn malformed_marker_or_geometry_does_not_enter_index() {
        let (id, mut recipe) = conversion("tconstruct:bronze", true);
        if let Recipe::Shaped { cells, .. } = &mut recipe { cells.pop(); }
        assert!(display_stacks(&id, &recipe).is_none());
        let (id, mut recipe) = conversion("tconstruct:bronze", false);
        if let Recipe::Shaped { cells, .. } = &mut recipe {
            cells[0].as_mut().unwrap().custom = None;
        }
        assert!(!marked(&recipe));
        assert!(display_stacks(&id, &recipe).is_none());
    }

    #[test]
    fn book_ingredients_keep_material_and_do_not_offer_id_only_placement() {
        let (id, recipe) = conversion("tconstruct:bronze", true);
        let Recipe::Shaped { width, height, cells, result } = &recipe else { unreachable!(); };
        let mut book = Book::default();
        book.entries.push(super::super::Entry {
            id: id.clone(), display_id: -1,
            category: super::super::Category::CraftingMisc, group: None,
            display: super::super::Display::Shaped {
                width: *width, height: *height, cells: cells.clone(),
            },
            result: result.create(), variants: Vec::new(), stacks: Vec::new(),
        });
        book.placements.insert(id.clone(), crate::crafting::Placement::default());
        book.placements.insert("other:recipe".into(), crate::crafting::Placement::default());
        decorate(&mut book, &[(id.clone(), recipe)]);
        assert!(!book.placements.contains_key(&id));
        assert!(book.placements.contains_key("other:recipe"));
        assert_eq!(book.entries[0].stacks.len(), 9);
        for (i, shown) in &book.entries[0].stacks {
            assert!(*i < 9);
            assert_eq!(shown.len(), 1);
            assert_eq!(&*shown[0].item, "tconstruct:fake_ingot");
            assert_eq!(shown[0].data().unwrap().other("minecraft:custom_data"),
                book.entries[0].result.data().unwrap().other("minecraft:custom_data"));
        }
    }

}
