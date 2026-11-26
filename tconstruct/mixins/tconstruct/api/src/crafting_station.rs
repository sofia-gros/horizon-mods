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
use pluto_lib::{BlockEntityType, BlockEntityTypeRegistry, Identifier, ScreenCanvas, ScreenHandlerRegistry, ScreenHandlerType, ScreenLayout};

/// Slots 0..=8 are the grid, slot 9 the result.
pub(crate) const RESULT_SLOT: usize = 9;
const SLOT_COUNT: usize = RESULT_SLOT + 1;
const NAME: &str = "crafting_station";
/// The icons sheet, as the station windows name it (the tab strip's texture).
const ICONS: &str = "tconstruct:gui/icons";
/// The vanilla crafting window's sheet, which the original's screen draws as its background
/// (`CraftingStationScreen` blits the vanilla crafting table texture). The host's GUI atlas
/// keys vanilla sheets without a namespace (`gui/...`, where a mod's own sheets are keyed
/// `<mod>:gui/...`): `native_sprite` (pinned host `native_hooks.rs`, API 19) looks the name up
/// verbatim, and the host's own crafting background is drawn under exactly this key
/// (`game.rs`). The `minecraft:`-prefixed form therefore never resolved, and the window fell
/// back to the host's plain panel (step-2 B8, confirmed in-game).
const VANILLA_CRAFTING: &str = "gui/container/crafting_table";

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

/// The window's background: the vanilla crafting sheet itself, blitted over the host's panel
/// (no own background). A sheet the host cannot resolve by this name draws nothing and the
/// panel stays.
fn draw_background(canvas: &mut dyn ScreenCanvas, origin: [f32; 2]) {
    canvas.sprite(VANILLA_CRAFTING, [0.0, 0.0, 176.0, 166.0], [origin[0], origin[1], 176.0, 166.0]);
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
            draw_background(draw.canvas, draw.origin);
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
    use super::*;

    #[derive(Default)]
    struct Recorder { sprites: Vec<(String, [f32; 4], [f32; 4])> }
    impl ScreenCanvas for Recorder {
        fn fill(&mut self, _: [f32; 4], _: u32) {}
        fn sprite(&mut self, name: &str, src: [f32; 4], dst: [f32; 4]) -> bool { self.sprites.push((name.to_owned(), src, dst)); true }
        fn text(&mut self, _: &str, _: [f32; 2], _: u32, _: bool) {}
        fn text_width(&self, _: &str) -> f32 { 0.0 }
    }

    #[test]
    fn the_grid_and_result_fill_the_declared_slots() {
        assert_eq!(super::SLOT_COUNT, 10, "nine grid slots and the result, as the menu declaration says");
        assert_eq!(super::own_positions().len(), super::SLOT_COUNT);
        assert_eq!(super::own_positions()[super::RESULT_SLOT], Some([124.0, 35.0]), "the result at the vanilla crafting place");
    }

    #[test]
    fn the_background_blits_the_vanilla_crafting_sheet_under_its_atlas_key() {
        // The host's GUI atlas keys vanilla sheets without a namespace (the host's own
        // crafting background is drawn as `gui/container/crafting_table`); the earlier
        // `minecraft:`-prefixed name never resolved and the window kept the plain panel.
        assert_eq!(super::VANILLA_CRAFTING, "gui/container/crafting_table");
        let mut canvas = Recorder::default();
        super::draw_background(&mut canvas, [10.0, 20.0]);
        assert_eq!(canvas.sprites.len(), 1, "the background is the one sheet blit");
        let (name, src, dst) = &canvas.sprites[0];
        assert_eq!(name, super::VANILLA_CRAFTING);
        assert_eq!((*src, *dst), ([0.0, 0.0, 176.0, 166.0], [10.0, 20.0, 176.0, 166.0]), "the sheet's crafting window, at the window's origin");
    }
}
