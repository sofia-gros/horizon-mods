//! The crafting station (CraftingStationBlockEntity, SlimeKnights, MIT; see LICENSE): a crafting
//! table whose 3x3 grid is the block's own inventory, so a half-arranged recipe survives closing
//! the window. The crafting is the game's own: the result is whatever the crafting book matches
//! for the grid, asked through PlutoLib's crafting lookup, and taking it follows the original's
//! `takeResult` — one item spent from every occupied cell, the match's remaining items left in
//! their cells, a remainder that cannot stay in a shrunken cell handed to the player. The
//! original caches the last matched recipe and recomputes lazily when the matrix changes; the
//! port asks the lookup on every tick and every take instead, which stages the same result.
//! The original also awards the recipe and fires the crafting event on a take; no public
//! surface names that event for a menu take, so a take here does neither.
//!
//! The window is a tabbed table's: the neighbouring tables' tabs above it and the neighbouring
//! chest's side inventory beside the grid are carried (see `tabbed_tables`), the side
//! inventory through the menu's linked slots.
use crate::station_menu::take_with;
use crate::tabbed_tables;
use pluto_lib::recipe_api_v1;
use pluto_lib::{BlockEntityType, BlockEntityTypeRegistry, Identifier, ItemSlot, MenuAccess, ScreenHandlerRegistry, ScreenHandlerType, ScreenLayout, SlotAction};

/// Slots 0..=8 are the grid, slot 9 the result.
pub(crate) const RESULT_SLOT: usize = 9;
const SLOT_COUNT: usize = RESULT_SLOT + 1;
const NAME: &str = "crafting_station";
/// The icons sheet, as the station windows name it (the tab strip's texture).
const ICONS: &str = "tconstruct:gui/icons";
/// The vanilla crafting window's sheet, which the original's screen draws as its background
/// (`CraftingStationScreen` blits the vanilla crafting table texture).
const VANILLA_CRAFTING: &str = "minecraft:gui/container/crafting_table";

/// The window's own slots' places: the grid at (30, 17), the result at (124, 35) — the vanilla
/// crafting window's geometry.
fn own_positions() -> Vec<Option<[f32; 2]>> {
    let mut positions: Vec<Option<[f32; 2]>> = Vec::new();
    for row in 0..3 {
        for column in 0..3 {
            positions.push(Some([30.0 + column as f32 * 18.0, 17.0 + row as f32 * 18.0]));
        }
    }
    positions.push(Some([124.0, 35.0]));
    positions
}

/// Registers the block entity type and the window. Called once from the product's hooks.
pub(crate) fn register() {
    let id = Identifier::new("tconstruct", NAME).expect("valid crafting station id");
    let mut layout = ScreenLayout::new(176.0, 166.0).inventory_at(8.0, 84.0);
    for position in own_positions() {
        let [x, y] = position.expect("an own position");
        layout = layout.slot(x, y);
    }
    // The side inventory's linked range follows the window's own slots.
    for _ in 0..tabbed_tables::SIDE_SLOTS { layout = layout.linked_slot(); }
    let mut window = ScreenHandlerType::new(id.clone(), layout)
        .on_slot_rule(|rule| { if rule.slot == RESULT_SLOT { rule.may_place = false; } })
        .exclude_from_pick_all(RESULT_SLOT)
        .on_menu_links(|context| tabbed_tables::links_answer(context.pos))
        .on_dynamic_slots(|state| {
            let mut positions = own_positions();
            positions.extend(tabbed_tables::side_positions(state.data, 0));
            Some(positions)
        })
        .on_draw(|draw| {
            // The original's background is the vanilla crafting sheet itself. The draw is a
            // plain blit over the host's panel (no own background): a sheet the host cannot
            // resolve by this name draws nothing and the panel stays, exactly as before.
            draw.canvas.sprite(VANILLA_CRAFTING, [0.0, 0.0, 176.0, 166.0], [draw.origin[0], draw.origin[1], 176.0, 166.0]);
            tabbed_tables::draw_side_panel(draw.canvas, draw.origin, draw.data, 0);
            tabbed_tables::draw_tabs(draw.canvas, draw.origin, draw.data, 0, ICONS);
        })
        .on_click(|press| if press.right { None } else { tabbed_tables::tab_click(press.at, press.data, 0) })
        .on_tooltip(|tip| {
            if let Some(name) = tabbed_tables::tab_tooltip(tip.at, tip.data, 0) {
                tip.lines.push(name);
            }
        })
        .on_button(|press| { tabbed_tables::press_tab(press); });
    for action in [SlotAction::Pickup, SlotAction::QuickMove, SlotAction::Swap, SlotAction::Throw] {
        window = window.on_slot_click(RESULT_SLOT, action, |click| {
            take_with(click, RESULT_SLOT, &|menu| craft(menu));
        });
    }
    ScreenHandlerRegistry::register(window);
    // The registered slot count is the menu's whole slot space (PlutoLib's `menu` contract):
    // the block's own slots plus the linked range the menu declaration names, or the host
    // drops the menu for a size mismatch.
    BlockEntityTypeRegistry::register(
        BlockEntityType::new(id).item_slots((SLOT_COUNT + tabbed_tables::SIDE_SLOTS) as u32).menu("tconstruct:crafting_station").on_place(crate::retextured::keep_texture)
            .ticker(|tick| {
                tabbed_tables::publish(tick, 0, true);
                refresh_result(tick.items);
            }),
    );
}

/// Stages the grid's crafting result in the result slot: the crafting book's match for the
/// nine grid slots, recomputed every tick like the other stations' previews. True when the
/// staged result changed.
pub(crate) fn refresh_result(items: &mut Vec<Option<ItemSlot>>) -> bool {
    if items.len() <= RESULT_SLOT { return false; }
    let next = recipe_api_v1::crafting_match(&items[..RESULT_SLOT], 3).map(|matched| matched.result);
    if items[RESULT_SLOT] == next { return false; }
    items[RESULT_SLOT] = next;
    true
}

/// One craft at the station, the original's `takeResult` over the menu's staged state: a
/// fresh match for the grid as it stands, one item spent from every occupied cell, the
/// match's remaining items applied per cell — a cell of one becomes its remainder, a
/// remainder matching the cell's stack rejoins it (the craft spends nothing there), any
/// other remainder is handed to the player as the cell shrinks — and the next match staged
/// as the new result. Nothing happens (and `None` returns) without a match.
fn craft(menu: &mut dyn MenuAccess) -> Option<ItemSlot> {
    let grid: Vec<Option<ItemSlot>> = (0..RESULT_SLOT).map(|slot| menu.stack(slot)).collect();
    let matched = recipe_api_v1::crafting_match(&grid, 3)?;
    let result = matched.result.clone();
    for (cell, original) in grid.iter().enumerate() {
        let Some(original) = original else { continue; };
        let remainder = matched.remainders.get(cell).cloned().flatten();
        if original.count <= 1 {
            menu.set_stack(cell, remainder);
        } else if let Some(remainder) = remainder {
            if menu.same_item_and_components(original, &remainder) {
                let mut stayed = remainder;
                stayed.count = original.count - 1 + stayed.count;
                menu.set_stack(cell, Some(stayed));
            } else {
                let mut shrunk = original.clone();
                shrunk.count -= 1;
                menu.set_stack(cell, Some(shrunk));
                menu.give_stack(remainder);
            }
        } else {
            let mut shrunk = original.clone();
            shrunk.count -= 1;
            menu.set_stack(cell, Some(shrunk));
        }
    }
    let next: Vec<Option<ItemSlot>> = (0..RESULT_SLOT).map(|slot| menu.stack(slot)).collect();
    menu.set_stack(RESULT_SLOT, recipe_api_v1::crafting_match(&next, 3).map(|matched| matched.result));
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::recipe_api_v1::CraftingMatch;

    #[test]
    fn the_grid_and_result_fill_the_declared_slots() {
        assert_eq!(super::SLOT_COUNT, 10, "nine grid slots and the result, as the menu declaration says");
        assert_eq!(super::own_positions().len(), super::SLOT_COUNT);
        assert_eq!(super::own_positions()[super::RESULT_SLOT], Some([124.0, 35.0]), "the result at the vanilla crafting place");
    }

    fn slot(item: &str, count: u32) -> ItemSlot {
        ItemSlot::new(Identifier::parse(item).unwrap(), count)
    }

    /// A three-recipe book for the station tests: four planks in a square craft a table,
    /// a lone milk bucket crafts a cake and leaves the bucket, and a lone honey bottle
    /// crafts sugar while the bottle itself stays (the stay a special recipe declares).
    fn book(grid: &[Option<ItemSlot>], size: usize) -> Option<CraftingMatch> {
        let occupied: Vec<(usize, &ItemSlot)> = grid.iter().enumerate().filter_map(|(index, cell)| cell.as_ref().map(|stack| (index, stack))).collect();
        let answer = |recipe: &str, result: ItemSlot, remainders: Vec<Option<ItemSlot>>| CraftingMatch { recipe: recipe.to_owned(), result, remainders };
        if occupied.len() == 4 && [0, 1, size, size + 1].iter().all(|index| occupied.iter().any(|(at, stack)| at == index && stack.item.to_string() == "minecraft:oak_planks")) {
            return Some(answer("minecraft:crafting_table", slot("minecraft:crafting_table", 1), vec![None; size * size]));
        }
        if occupied.len() == 1 {
            let (index, stack) = occupied[0];
            let mut remainders = vec![None; size * size];
            match stack.item.to_string().as_str() {
                "minecraft:milk_bucket" => {
                    remainders[index] = Some(slot("minecraft:bucket", 1));
                    return Some(answer("minecraft:cake", slot("minecraft:cake", 1), remainders));
                }
                "minecraft:honey_bottle" => {
                    remainders[index] = Some(slot("minecraft:honey_bottle", 1));
                    return Some(answer("test:sugar_from_bottle", slot("minecraft:sugar", 1), remainders));
                }
                _ => {}
            }
        }
        None
    }

    /// A staged menu over plain slots: the station's ten and a player range behind them.
    struct TestMenu {
        slots: Vec<Option<ItemSlot>>,
        carried: Option<ItemSlot>,
        given: Vec<ItemSlot>,
        dropped: Vec<ItemSlot>,
    }
    impl TestMenu {
        fn new(grid: Vec<Option<ItemSlot>>) -> Self {
            let mut slots = grid;
            slots.resize(RESULT_SLOT + 1 + 36, None);
            Self { slots, carried: None, given: Vec::new(), dropped: Vec::new() }
        }
    }
    impl MenuAccess for TestMenu {
        fn slot_count(&self) -> usize { self.slots.len() }
        fn stack(&self, slot: usize) -> Option<ItemSlot> { self.slots.get(slot).cloned().flatten() }
        fn set_stack(&mut self, slot: usize, stack: Option<ItemSlot>) -> bool {
            match self.slots.get_mut(slot) { Some(at) => { *at = stack; true } None => false }
        }
        fn swap_stack(&self, _: i32) -> Option<ItemSlot> { None }
        fn set_swap_stack(&mut self, _: i32, _: Option<ItemSlot>) -> bool { false }
        fn carried(&self) -> Option<ItemSlot> { self.carried.clone() }
        fn set_carried(&mut self, stack: Option<ItemSlot>) { self.carried = stack; }
        fn same_item_and_components(&self, left: &ItemSlot, right: &ItemSlot) -> bool {
            left.item == right.item && left.damage == right.damage && left.custom_data == right.custom_data
        }
        fn max_stack_count(&self, _: &ItemSlot) -> u32 { 64 }
        fn move_stack(&mut self, stack: &mut ItemSlot, start: usize, end: usize, _: bool) -> bool {
            let mut moved = false;
            for index in start..end.min(self.slots.len()) {
                if stack.count == 0 { break; }
                match &mut self.slots[index] {
                    Some(at) if at.item == stack.item && at.count < 64 => {
                        let take = (64 - at.count).min(stack.count);
                        at.count += take;
                        stack.count -= take;
                        moved = true;
                    }
                    None => {
                        self.slots[index] = Some(stack.clone());
                        stack.count = 0;
                        moved = true;
                    }
                    _ => {}
                }
            }
            moved
        }
        fn crafting_remainder(&self, _: &ItemSlot) -> Option<ItemSlot> { None }
        fn give_stack(&mut self, stack: ItemSlot) { self.given.push(stack); }
        fn drop_stack(&mut self, stack: &ItemSlot) { self.dropped.push(stack.clone()); }
    }

    fn grid_with(cells: &[(usize, &str, u32)]) -> Vec<Option<ItemSlot>> {
        let mut grid = vec![None; RESULT_SLOT + 1];
        for &(index, item, count) in cells {
            grid[index] = Some(slot(item, count));
        }
        grid
    }

    #[test]
    fn a_vanilla_recipe_crafts_at_the_station() {
        recipe_api_v1::set_crafting_lookup(book);
        let mut items = grid_with(&[(0, "minecraft:oak_planks", 2), (1, "minecraft:oak_planks", 2), (3, "minecraft:oak_planks", 2), (4, "minecraft:oak_planks", 2)]);
        assert!(refresh_result(&mut items), "the ticker stages the matched result");
        assert_eq!(items[RESULT_SLOT], Some(slot("minecraft:crafting_table", 1)));
        let mut menu = TestMenu::new(items[..RESULT_SLOT].to_vec());
        menu.set_stack(RESULT_SLOT, Some(slot("minecraft:crafting_table", 1)));
        let crafted = craft(&mut menu).expect("the staged grid crafts");
        assert_eq!(crafted, slot("minecraft:crafting_table", 1));
        for index in [0, 1, 3, 4] {
            assert_eq!(menu.stack(index), Some(slot("minecraft:oak_planks", 1)), "one plank spent from cell {index}");
        }
        assert_eq!(menu.stack(RESULT_SLOT), Some(slot("minecraft:crafting_table", 1)), "the remaining planks match again");
        let crafted = craft(&mut menu).expect("the second craft runs");
        assert_eq!(crafted, slot("minecraft:crafting_table", 1));
        for index in [0, 1, 3, 4] {
            assert_eq!(menu.stack(index), None, "cell {index} empties");
        }
        assert_eq!(menu.stack(RESULT_SLOT), None, "nothing is staged once the grid is spent");
    }

    #[test]
    fn a_non_matching_grid_yields_nothing() {
        recipe_api_v1::set_crafting_lookup(book);
        let mut items = grid_with(&[(0, "minecraft:oak_planks", 4)]);
        assert!(!refresh_result(&mut items), "no result is staged for a lone plank");
        assert_eq!(items[RESULT_SLOT], None);
        let mut menu = TestMenu::new(items[..RESULT_SLOT].to_vec());
        assert_eq!(craft(&mut menu), None, "no craft runs without a match");
        assert_eq!(menu.stack(0), Some(slot("minecraft:oak_planks", 4)), "the grid is untouched");
    }

    #[test]
    fn a_remainder_replaces_a_spent_single() {
        recipe_api_v1::set_crafting_lookup(book);
        let mut menu = TestMenu::new(grid_with(&[(4, "minecraft:milk_bucket", 1)])[..RESULT_SLOT].to_vec());
        let crafted = craft(&mut menu).expect("the milk bucket crafts");
        assert_eq!(crafted, slot("minecraft:cake", 1));
        assert_eq!(menu.stack(4), Some(slot("minecraft:bucket", 1)), "the empty bucket stays in the cell");
        assert!(menu.given.is_empty(), "nothing is handed over");
    }

    #[test]
    fn a_remainder_is_handed_over_when_the_cell_shrinks() {
        recipe_api_v1::set_crafting_lookup(book);
        let mut menu = TestMenu::new(grid_with(&[(4, "minecraft:milk_bucket", 3)])[..RESULT_SLOT].to_vec());
        let crafted = craft(&mut menu).expect("the milk bucket crafts");
        assert_eq!(crafted, slot("minecraft:cake", 1));
        assert_eq!(menu.stack(4), Some(slot("minecraft:milk_bucket", 2)), "one milk bucket is spent");
        assert_eq!(menu.given, vec![slot("minecraft:bucket", 1)], "the empty bucket goes to the player");
    }

    #[test]
    fn a_stay_remainder_keeps_the_cells_count() {
        recipe_api_v1::set_crafting_lookup(book);
        let mut menu = TestMenu::new(grid_with(&[(0, "minecraft:honey_bottle", 3)])[..RESULT_SLOT].to_vec());
        let crafted = craft(&mut menu).expect("the bottle crafts");
        assert_eq!(crafted, slot("minecraft:sugar", 1));
        assert_eq!(menu.stack(0), Some(slot("minecraft:honey_bottle", 3)), "the declared stay spends nothing from the cell");
        assert!(menu.given.is_empty(), "nothing is handed over");
    }
}
