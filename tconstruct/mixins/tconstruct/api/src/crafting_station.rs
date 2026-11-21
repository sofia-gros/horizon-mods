//! The crafting station (CraftingStationBlockEntity, SlimeKnights, MIT; see LICENSE): a crafting
//! table whose 3x3 grid is the block's own inventory, so a half-arranged recipe survives closing
//! the window. The grid and its persistence are ported; the crafting itself is not, for a
//! foundation reason: the result is whatever the game's crafting recipe type matches for the
//! grid, and PlutoLib's public API has no crafting-recipe query (no match, no assemble, no
//! remaining-items) — the direct-host contracts keep the native crafting code unattached by
//! design. The result slot therefore stays empty until such a query exists. The neighbouring
//! chests the original's tabbed menu shows beside the grid are likewise out of reach (a menu
//! cannot read a neighbour's item slots), the same gap the station windows record.
use pluto_lib::{BlockEntityType, BlockEntityTypeRegistry, Identifier, ScreenHandlerRegistry, ScreenHandlerType, ScreenLayout};

/// Slots 0..=8 are the grid, slot 9 the result.
pub(crate) const RESULT_SLOT: usize = 9;
const SLOT_COUNT: usize = RESULT_SLOT + 1;
const NAME: &str = "crafting_station";

/// Registers the block entity type and the window. Called once from the product's hooks.
pub(crate) fn register() {
    let id = Identifier::new("tconstruct", NAME).expect("valid crafting station id");
    // The vanilla crafting window's geometry: the grid at (30, 17), the result at (124, 35).
    let mut layout = ScreenLayout::new(176.0, 166.0).inventory_at(8.0, 84.0);
    for row in 0..3 {
        for column in 0..3 {
            layout = layout.slot(30.0 + column as f32 * 18.0, 17.0 + row as f32 * 18.0);
        }
    }
    layout = layout.slot(124.0, 35.0);
    let window = ScreenHandlerType::new(id.clone(), layout)
        .on_slot_rule(|rule| { if rule.slot == RESULT_SLOT { rule.may_place = false; } })
        .exclude_from_pick_all(RESULT_SLOT);
    ScreenHandlerRegistry::register(window);
    BlockEntityTypeRegistry::register(
        BlockEntityType::new(id).item_slots(SLOT_COUNT as u32).menu("tconstruct:crafting_station"),
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_grid_and_result_fill_the_declared_slots() {
        assert_eq!(super::SLOT_COUNT, 10, "nine grid slots and the result, as the menu declaration says");
    }
}
