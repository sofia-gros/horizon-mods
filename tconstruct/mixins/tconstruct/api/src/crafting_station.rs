//! The crafting station (CraftingStationBlockEntity, SlimeKnights, MIT; see LICENSE): a crafting
//! table whose 3x3 grid is the block's own inventory, so a half-arranged recipe survives closing
//! the window. The grid and its persistence are ported; the crafting itself is not, for a
//! foundation reason: the result is whatever the game's crafting recipe type matches for the
//! grid, and PlutoLib's public API has no crafting-recipe query (no match, no assemble, no
//! remaining-items) — the direct-host contracts keep the native crafting code unattached by
//! design. The result slot therefore stays empty until such a query exists.
//!
//! The window is a tabbed table's: the neighbouring tables' tabs above it and the neighbouring
//! chest's side inventory beside the grid are carried (see `tabbed_tables`), the side
//! inventory through the menu's linked slots.
use crate::tabbed_tables;
use pluto_lib::{BlockEntityType, BlockEntityTypeRegistry, Identifier, ScreenHandlerRegistry, ScreenHandlerType, ScreenLayout};

/// Slots 0..=8 are the grid, slot 9 the result.
pub(crate) const RESULT_SLOT: usize = 9;
const SLOT_COUNT: usize = RESULT_SLOT + 1;
const NAME: &str = "crafting_station";
/// The icons sheet, as the station windows name it (the tab strip's texture).
const ICONS: &str = "tconstruct:gui/icons";

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
    let window = ScreenHandlerType::new(id.clone(), layout)
        .on_slot_rule(|rule| { if rule.slot == RESULT_SLOT { rule.may_place = false; } })
        .exclude_from_pick_all(RESULT_SLOT)
        .on_menu_links(|context| tabbed_tables::links_answer(context.pos))
        .on_dynamic_slots(|state| {
            let mut positions = own_positions();
            positions.extend(tabbed_tables::side_positions(state.data, 0));
            Some(positions)
        })
        .on_draw(|draw| {
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
    ScreenHandlerRegistry::register(window);
    // The registered slot count is the menu's whole slot space (PlutoLib's `menu` contract):
    // the block's own slots plus the linked range the menu declaration names, or the host
    // drops the menu for a size mismatch.
    BlockEntityTypeRegistry::register(
        BlockEntityType::new(id).item_slots((SLOT_COUNT + tabbed_tables::SIDE_SLOTS) as u32).menu("tconstruct:crafting_station").on_place(crate::retextured::keep_texture)
            .ticker(|tick| tabbed_tables::publish(tick, 0, true)),
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_grid_and_result_fill_the_declared_slots() {
        assert_eq!(super::SLOT_COUNT, 10, "nine grid slots and the result, as the menu declaration says");
        assert_eq!(super::own_positions().len(), super::SLOT_COUNT);
        assert_eq!(super::own_positions()[super::RESULT_SLOT], Some([124.0, 35.0]), "the result at the vanilla crafting place");
    }
}
