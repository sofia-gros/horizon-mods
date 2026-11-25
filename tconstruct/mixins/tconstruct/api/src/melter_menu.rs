//! The melter and heater block entities, their windows and slot rules over PlutoLib's public API
//! (SlimeKnights MelterBlockEntity, HeaterBlockEntity, MelterContainerMenu, SingleItemContainerMenu, MelterScreen and
//! the GuiMeltingModule/GuiFuelModule/GuiTankModule it draws; MIT, see LICENSE). The melter menu holds its three melting
//! slots and, as its fourth slot, the fuel slot of the block below: the original adds it at (151, 32) only when the block
//! below is a fuel tank (`MelterContainerMenu`, `TinkerTags.Blocks.FUEL_TANKS` — the seared heater, the seared tanks,
//! the scorched tanks), and here the menu-links callback answers the host's question with the block below only then, so
//! the slot shows and edits the fuel tank's own inventory in place (the burn cycle, which reads the block below, sees
//! every change in the same step). With any other block below — a chest, say — no link is answered, the linked slot holds
//! nothing and takes nothing, and the chest's slot 0 stays untouched. This window frames the slot (the
//! FUEL_SLOT background) when a heater is below, from the same menu-data flag the ticker publishes. The tank is drawn as
//! the fluid's still texture tiled over a flat fill of its colour (the fill is what shows where the host's atlas
//! holds no sprite for the texture); clicking it trades the held container with the tank, and its tooltip names the
//! fluid over the fluid and the capacity over the empty part. The fuel gauge draws the fuel tank below's fluid; its
//! own fill and empty clicks are the heater's or tank's, below, out of a menu button's reach.
use crate::{
    common,
    melter::{self, slot_data, Live, State, Tank, DATA_FUEL_AMOUNT, DATA_FUEL_CAPACITY, DATA_FUEL_FLUID, DATA_HEATER, DATA_SLOTS, DATA_TANK_AMOUNT, DATA_TANK_FLUID, SLOTS},
    heating::MELTER_CAPACITY,
};
use pluto_lib::{
    BlockEntityType, BlockEntityTypeRegistry, BlockPos, FluidItems, FluidRegistry, I18n, Identifier, MenuButton, Neighbours, ScreenCanvas, ScreenHandlerRegistry, ScreenHandlerType,
    ScreenLayout, TransferDirection,
};

/// A menu value too large for one 16 bit slot, in two: the high half and the low half.
pub(crate) fn split_pair(value: i32) -> (i32, i32) { (value >> 15, value & 0x7FFF) }
pub(crate) fn join_pair(data: &[i32], index: usize) -> i32 { (data.get(index).copied().unwrap_or(0) << 15) | data.get(index + 1).copied().unwrap_or(0) }

pub(crate) const MELTER: &str = "seared_melter";
pub(crate) const HEATER: &str = "seared_heater";
const MELTER_BACKGROUND: &str = "tconstruct:gui/melter";
const HEATER_BACKGROUND: &str = "tconstruct:gui/heater";
/// MelterContainerMenu: the melting slots sit at x 22, 18 apart from y 16.
const SLOT_X: f32 = 22.0;
const SLOT_Y: f32 = 16.0;
/// MelterScreen: the tank, its scale, the fuel flame and the fuel gauge.
const TANK: [f32; 4] = [90.0, 16.0, 52.0, 52.0];
const FLAME_AT: [f32; 2] = [152.0, 15.0];
const FUEL_AT: [f32; 4] = [153.0, 32.0, 12.0, 36.0];
/// GuiMeltingModule: the four heat bars (progress, no heat, no space, unmeltable), 3 wide and 16 high, in the texture at x 176 + 3 * kind.
const BAR: [f32; 2] = [3.0, 16.0];

/// What the heat bar of a slot shows (GuiMeltingModule.drawHeatBars): which of the four bars and how full it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Bar { Progress(f32), NoHeat, NoSpace, Unmeltable }

pub(crate) fn bar_of(current: i32, required: i32, required_temp: i32, temperature: i32) -> Bar {
    if required == 0 { return Bar::Unmeltable; }
    if required_temp > temperature { return Bar::NoHeat; }
    if current < 0 { return Bar::NoSpace; }
    Bar::Progress(if current <= required { current as f32 / required as f32 } else { 1.0 })
}

/// The colour a fluid is drawn in: its defined colour, opaque.
fn fluid_color(id: &Identifier) -> u32 {
    let color = FluidRegistry::get(&id.to_string()).map_or(0xFF_FF_FF_FF, |def| def.attributes.color);
    if color >> 24 == 0 { color | 0xFF00_0000 } else { color }
}

/// `GuiUtil.renderTiledFluid`: the fluid's still texture tiled over `rect` in 16 pixel tiles, tinted `argb`. Every
/// tile restarts the texture at its top left and the last tile of a row or column is clipped to the part that fits,
/// which the canvas's tinted sub-rectangle draw expresses directly. A fluid with no texture of its own draws nothing;
/// the flat fill the caller drew under the rect first is what shows then. A gas (lighter than air) is drawn
/// unflipped: the canvas has no flipped draw, and the original's own flip carries a "causes tiling errors" note.
pub(crate) fn draw_tiled_fluid(canvas: &mut dyn ScreenCanvas, fluid: &Identifier, rect: [f32; 4], argb: u32) {
    let Some(visual) = crate::rendering::visual(fluid) else { return };
    let (mut rows_left, mut top) = (rect[3], rect[1]);
    while rows_left > 0.0 {
        let tile_height = rows_left.min(16.0);
        let (mut columns_left, mut left) = (rect[2], rect[0]);
        while columns_left > 0.0 {
            let tile_width = columns_left.min(16.0);
            canvas.sprite_tinted(&visual.still, [0.0, 0.0, tile_width, tile_height], [16.0, 16.0], [left, top, tile_width, tile_height], argb);
            left += tile_width;
            columns_left -= tile_width;
        }
        top += tile_height;
        rows_left -= tile_height;
    }
}

fn tank_fluid(data: &[i32]) -> Option<Identifier> {
    let which = data.get(DATA_TANK_FLUID).copied().unwrap_or(0);
    (which > 0).then(|| melter::fluid_order().get(which as usize - 1).cloned()).flatten()
}

/// Draws the melter window: background, fuel gauge and flame, tank fill and scale, and the heat bar of every occupied slot.
pub(crate) fn draw_melter(canvas: &mut dyn ScreenCanvas, origin: [f32; 2], data: &[i32], mouse: [f32; 2]) {
    let get = |index: usize| data.get(index).copied().unwrap_or(0);
    let [x, y] = origin;
    canvas.sprite(MELTER_BACKGROUND, [0.0, 0.0, 176.0, 166.0], [x, y, 176.0, 166.0]);
    // with a heater below, the fuel slot's frame (FUEL_SLOT); otherwise the gauge's own background (FUEL_TANK)
    if get(DATA_HEATER) != 0 {
        canvas.sprite(MELTER_BACKGROUND, [176.0, 52.0, 18.0, 36.0], [x + 150.0, y + 31.0, 18.0, 36.0]);
    } else {
        canvas.sprite(MELTER_BACKGROUND, [194.0, 52.0, 14.0, 38.0], [x + 152.0, y + 31.0, 14.0, 38.0]);
    }
    let (fuel, quality) = (get(0), get(1));
    if fuel > 0 && quality > 0 {
        let height = (14 * fuel / quality).clamp(0, 14) as f32;
        canvas.sprite(MELTER_BACKGROUND, [176.0, 136.0 + 14.0 - height, 14.0, height], [x + FLAME_AT[0], y + FLAME_AT[1] + 14.0 - height, 14.0, height]);
    }
    // the fuel gauge's fluid, the fuel tank below's own
    let (fuel_amount, fuel_capacity) = (join_pair(data, DATA_FUEL_AMOUNT), join_pair(data, DATA_FUEL_CAPACITY));
    if fuel_amount > 0 && fuel_capacity > 0 {
        let height = (FUEL_AT[3] * (fuel_amount.min(fuel_capacity) as f32) / fuel_capacity as f32).floor();
        let fluid = (get(DATA_FUEL_FLUID) > 0).then(|| melter::fluid_order().get(get(DATA_FUEL_FLUID) as usize - 1).cloned()).flatten();
        let color = fluid.as_ref().map_or(0xFF_FF_FF_FF, |fluid| fluid_color(fluid));
        let rect = [x + FUEL_AT[0], y + FUEL_AT[1] + FUEL_AT[3] - height, FUEL_AT[2], height];
        canvas.fill(rect, color);
        if let Some(fluid) = &fluid { draw_tiled_fluid(canvas, fluid, rect, color); }
    }
    let amount = get(DATA_TANK_AMOUNT);
    let tank_height = if amount > 0 { (TANK[3] * (amount.min(MELTER_CAPACITY) as f32) / MELTER_CAPACITY as f32).floor() } else { 0.0 };
    if amount > 0 {
        let fluid = tank_fluid(data);
        let color = fluid.as_ref().map_or(0xFF_FF_FF_FF, |fluid| fluid_color(fluid));
        let rect = [x + TANK[0], y + TANK[1] + TANK[3] - tank_height, TANK[2], tank_height];
        canvas.fill(rect, color);
        if let Some(fluid) = &fluid { draw_tiled_fluid(canvas, fluid, rect, color); }
    }
    canvas.sprite(MELTER_BACKGROUND, [176.0, 0.0, 52.0, 52.0], [x + TANK[0], y + TANK[1], 52.0, 52.0]);
    // the hovered part of the tank lights up, split at the fluid's surface
    if inside(mouse, TANK) {
        let surface = TANK[1] + TANK[3] - tank_height;
        let rect = if mouse[1] >= surface { [x + TANK[0], y + surface, TANK[2], tank_height] } else { [x + TANK[0], y + TANK[1], TANK[2], surface - TANK[1]] };
        if rect[3] > 0.0 { canvas.fill(rect, 0x59_FF_FF_FF); }
    }
    let temperature = get(2);
    for slot in (0..SLOTS).filter(|slot| get(slot_data(*slot) + 3) != 0) {
        let base = slot_data(slot);
        let (current, required, required_temp) = (get(base), get(base + 1), get(base + 2));
        let (kind, progress) = match bar_of(current, required, required_temp, temperature) {
            Bar::Progress(progress) => (0.0, progress), Bar::NoHeat => (1.0, 1.0), Bar::NoSpace => (2.0, 1.0), Bar::Unmeltable => (3.0, 1.0),
        };
        let height = (BAR[1] * progress).floor();
        if height <= 0.0 { continue; }
        let top = SLOT_Y + 18.0 * slot as f32;
        canvas.sprite(MELTER_BACKGROUND, [176.0 + 3.0 * kind, 150.0 + BAR[1] - height, BAR[0], height], [x + SLOT_X - 4.0, y + top + BAR[1] - height, BAR[0], height]);
    }
}

fn inside(at: [f32; 2], rect: [f32; 4]) -> bool { at[0] >= rect[0] && at[0] < rect[0] + rect[2] && at[1] >= rect[1] && at[1] < rect[1] + rect[3] }

/// The tank's fluid surface, in window pixels: hovering below it is the fluid's own tooltip,
/// above it the capacity's (`GuiTankModule` splits the two at the surface).
fn tank_surface(data: &[i32]) -> f32 {
    let amount = data.get(DATA_TANK_AMOUNT).copied().unwrap_or(0);
    if amount <= 0 { return TANK[1] + TANK[3]; }
    TANK[1] + TANK[3] - (TANK[3] * (amount.min(MELTER_CAPACITY) as f32) / MELTER_CAPACITY as f32).floor()
}

/// Tooltip lines for a point of the melter window (GuiTankModule, GuiFuelModule, GuiMeltingModule.drawHeatTooltips).
pub(crate) fn melter_tooltip(at: [f32; 2], data: &[i32], lines: &mut Vec<String>) {
    let get = |index: usize| data.get(index).copied().unwrap_or(0);
    if inside(at, [TANK[0] - 1.0, TANK[1] - 1.0, TANK[2] + 2.0, TANK[3] + 2.0]) {
        let amount = get(DATA_TANK_AMOUNT);
        if amount > 0 && at[1] >= tank_surface(data) {
            if let Some(fluid) = tank_fluid(data) {
                lines.push(FluidRegistry::display_name(&fluid.to_string()).unwrap_or_else(|| fluid.to_string()));
            }
            lines.push(I18n::translate_args("gui.tconstruct.fluid.millibucket", &[&amount.to_string()]));
        } else {
            lines.push(I18n::translate("gui.tconstruct.melting.capacity"));
            lines.push(I18n::translate_args("gui.tconstruct.fluid.millibucket", &[&MELTER_CAPACITY.to_string()]));
            if MELTER_CAPACITY - amount > 0 {
                lines.push(I18n::translate("gui.tconstruct.melting.capacity.available"));
                lines.push(I18n::translate_args("gui.tconstruct.fluid.millibucket", &[&(MELTER_CAPACITY - amount).to_string()]));
            }
        }
    } else if inside(at, [FUEL_AT[0] - 1.0, FUEL_AT[1] - 1.0, FUEL_AT[2] + 2.0, FUEL_AT[3] + 2.0]) {
        let temperature = get(2);
        let fuel_amount = join_pair(data, DATA_FUEL_AMOUNT);
        if fuel_amount > 0 {
            // liquid fuel: its name and amount, then its temperature — or, at no temperature, the
            // word that it cannot burn
            if let Some(fluid) = (get(DATA_FUEL_FLUID) > 0).then(|| melter::fluid_order().get(get(DATA_FUEL_FLUID) as usize - 1).cloned()).flatten() {
                lines.push(FluidRegistry::display_name(&fluid.to_string()).unwrap_or_else(|| fluid.to_string()));
            }
            lines.push(I18n::translate_args("gui.tconstruct.fluid.millibucket", &[&fuel_amount.to_string()]));
            if temperature == 0 {
                lines.push(format!("§c{}", I18n::translate("gui.tconstruct.melting.fuel.invalid")));
            } else {
                lines.push(format!("§7§o{}", I18n::translate_args("gui.tconstruct.melting.fuel.temperature", &[&temperature.to_string()])));
            }
        } else if get(0) > 0 && temperature > 0 {
            lines.push(I18n::translate("gui.tconstruct.melting.fuel.solid"));
            lines.push(format!("§7§o{}", I18n::translate_args("gui.tconstruct.melting.fuel.temperature", &[&temperature.to_string()])));
        } else if join_pair(data, DATA_FUEL_CAPACITY) > 0 || get(DATA_HEATER) != 0 {
            lines.push(I18n::translate("gui.tconstruct.melting.fuel.empty"));
        } else {
            lines.push(I18n::translate("gui.tconstruct.melting.fuel.no_tank"));
        }
    } else {
        for slot in 0..SLOTS {
            let top = SLOT_Y + 18.0 * slot as f32;
            if get(slot_data(slot) + 3) != 0 && inside(at, [SLOT_X - 5.0, top - 1.0, BAR[0] + 1.0, BAR[1] + 2.0]) {
                let base = slot_data(slot);
                let key = match bar_of(get(base), get(base + 1), get(base + 2), get(2)) {
                    Bar::Unmeltable => "gui.tconstruct.melting.no_recipe", Bar::NoHeat => "gui.tconstruct.melting.no_heat",
                    Bar::NoSpace => "gui.tconstruct.melting.no_space", Bar::Progress(_) => return,
                };
                lines.push(I18n::translate(key));
                return;
            }
        }
    }
}

/// The melter's ticker: loads its saved state, runs one game tick of the cycle against the loaded recipes and the block below, saves it
/// and publishes the numbers its window draws. Nothing happens while the recipes or tags are not loaded.
fn tick_melter(tick: &mut pluto_lib::BlockEntityTick<'_>) {
    let Some(data) = crate::common::smeltery_data() else { return };
    let Ok(data) = data.read() else { return };
    if !data.loaded() { return; }
    let item_tags = common::item_snapshot();
    let mut state = State::load(tick.data, tick.items);
    let mut tank = Tank::read(tick);
    let mut world = Live::new(&data, item_tags.as_deref(), tick.pos);
    state.step(tick.items, &mut tank, &mut world);
    state.save(tick.data);
    tank.write(tick);
    let gauge = world.fuel_tank();
    let heater = world.below_is_heater();
    state.publish(tick, &tank, gauge.as_ref(), heater);
}

/// A click in the tank trades the held container with it (the tank module's click; the fuel
/// gauge's clicks belong to the block below and are not sent).
fn melter_click(at: [f32; 2]) -> Option<i32> { inside(at, TANK).then_some(0) }

/// `clickMenuButton` 0: the held container trades with this block entity's tank. A trade whose
/// result would not fit back in the hand does not happen, as on the smeltery's tank.
fn melter_press(button: &mut MenuButton<'_>) {
    if button.id != 0 { return; }
    let Some(held) = button.carried.clone() else { return };
    let mut target = crate::fluid_items::TankTarget { fluid: button.fluid(0), capacity: MELTER_CAPACITY as u32 };
    let Some(done) = FluidItems::interact(Some(&held), &mut target, TransferDirection::Auto) else { return };
    if !done.give.is_empty() { return; }
    button.set_fluid(0, target.fluid);
    *button.carried = done.held;
}

/// `MelterContainerMenu`: the fuel slot is the block below's slot 0 only when that block is a fuel tank
/// (`TinkerTags.Blocks.FUEL_TANKS`). Any other block below lends no slot: the link is not answered, so the
/// menu shows no fuel slot for it and its slot 0 stays untouched.
fn fuel_links(pos: BlockPos) -> Vec<BlockPos> {
    let below = [pos[0], pos[1] - 1, pos[2]];
    if Neighbours::block_state(below).is_some_and(|state| state.is_in_tag(melter::FUEL_TANKS)) {
        vec![below]
    } else {
        Vec::new()
    }
}

pub(crate) fn register() {
    let melter_id = Identifier::new("tconstruct", MELTER).expect("valid melter id");
    // The fuel slot follows the melting slots, at the original's (151, 32), backed by the block
    // below (the heater's own slot 0): the menu type's `links` cover it (see melter.json).
    let layout = (0..SLOTS).fold(ScreenLayout::new(176.0, 166.0).with_own_background(), |layout, slot| layout.slot(SLOT_X, SLOT_Y + 18.0 * slot as f32))
        .neighbour_slot(151.0, 32.0, [0, -1, 0]);
    let window = ScreenHandlerType::new(melter_id.clone(), layout)
        // The melting slots hold one item; the fuel slot (the neighbour's own) stacks as its
        // container allows.
        .on_slot_rule(|rule| { if rule.slot < SLOTS { rule.max_stack = 1; } })
        .on_draw(|draw| draw_melter(draw.canvas, draw.origin, draw.data, draw.mouse))
        .on_click(|press| melter_click(press.at))
        .on_tooltip(|tip| melter_tooltip(tip.at, tip.data, tip.lines))
        .on_button(|button| melter_press(button))
        .on_menu_links(|context| fuel_links(context.pos));
    ScreenHandlerRegistry::register(window);
    // The registered slot count is the menu's whole slot space (PlutoLib's `menu` contract):
    // the block's own slots plus the linked range the menu declaration names, or the host
    // drops the menu for a size mismatch.
    BlockEntityTypeRegistry::register(
        BlockEntityType::new(melter_id).item_slots(SLOTS as u32 + 1).fluid_tanks([MELTER_CAPACITY as u32]).menu("tconstruct:melter").sync_data().ticker(tick_melter)
            .on_use(|used| crate::fluid_items::use_tank(used, MELTER_CAPACITY as u32)),
    );

    let heater_id = Identifier::new("tconstruct", HEATER).expect("valid heater id");
    // SingleItemContainerMenu: one slot at (80, 20) over a 133 high window, the inventory at 51.
    let layout = ScreenLayout::new(176.0, 133.0).inventory_at(8.0, 51.0).with_own_background().slot(80.0, 20.0);
    let window = ScreenHandlerType::new(heater_id.clone(), layout)
        .on_slot_rule(|rule| { rule.may_place = melter::heater_accepts(rule.item); })
        .on_accepts(|put| melter::heater_accepts(put.item))
        .on_draw(|draw| { draw.canvas.sprite(HEATER_BACKGROUND, [0.0, 0.0, 176.0, 133.0], [draw.origin[0], draw.origin[1], 176.0, 133.0]); });
    ScreenHandlerRegistry::register(window);
    BlockEntityTypeRegistry::register(BlockEntityType::new(heater_id).item_slots(1).menu("tconstruct:single_item"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::{MemoryNeighbourOps, MemoryWorldOps, with_neighbours, with_world};

    #[derive(Default)]
    struct Recorder { sprites: Vec<(String, [f32; 4], [f32; 4])>, fills: Vec<([f32; 4], u32)>, tinted: Vec<(String, [f32; 4], [f32; 4], u32)> }
    impl ScreenCanvas for Recorder {
        fn fill(&mut self, rect: [f32; 4], argb: u32) { self.fills.push((rect, argb)); }
        fn sprite(&mut self, name: &str, src: [f32; 4], dst: [f32; 4]) -> bool { self.sprites.push((name.to_owned(), src, dst)); true }
        fn sprite_tinted(&mut self, name: &str, src: [f32; 4], _: [f32; 2], dst: [f32; 4], argb: u32) -> bool { self.tinted.push((name.to_owned(), src, dst, argb)); true }
        fn text(&mut self, _: &str, _: [f32; 2], _: u32, _: bool) {}
        fn text_width(&self, _: &str) -> f32 { 0.0 }
    }

    fn data(values: &[(usize, i32)]) -> Vec<i32> {
        let mut data = vec![0; DATA_SLOTS];
        for (index, value) in values { data[*index] = *value; }
        data
    }

    #[test]
    fn a_heat_bar_follows_the_original_order_of_errors() {
        assert_eq!(bar_of(0, 0, 0, 1000), Bar::Unmeltable, "no required time is no recipe");
        assert_eq!(bar_of(0, 600, 800, 700), Bar::NoHeat);
        assert_eq!(bar_of(-1, 600, 800, 900), Bar::NoSpace);
        assert_eq!(bar_of(300, 600, 800, 900), Bar::Progress(0.5));
        assert_eq!(bar_of(900, 600, 800, 900), Bar::Progress(1.0), "an overshoot is a full bar");
    }

    #[test]
    fn the_window_draws_flame_tank_and_the_bars_of_occupied_slots_only() {
        let mut canvas = Recorder::default();
        // half the fuel left (flame 7 high), a half full tank, slot 1 halfway through an ingot, slot 2 empty
        let values = data(&[(0, 200), (1, 400), (2, 800), (DATA_TANK_AMOUNT, MELTER_CAPACITY / 2),
            (slot_data(1), 300), (slot_data(1) + 1, 600), (slot_data(1) + 2, 800), (slot_data(1) + 3, 1), (slot_data(2) + 1, 600)]);
        draw_melter(&mut canvas, [10.0, 20.0], &values, [-1.0, -1.0]);
        let flame = canvas.sprites.iter().find(|s| s.1[0] == 176.0 && s.1[2] == 14.0).expect("flame");
        assert_eq!((flame.1[3], flame.2), (7.0, [162.0, 42.0, 14.0, 7.0]), "the flame grows upward from its base");
        assert_eq!(canvas.fills, vec![([100.0, 62.0, 52.0, 26.0], 0xFFFF_FFFF)], "half the 52 pixel scale, white for a fluid no mod defined");
        let bars: Vec<_> = canvas.sprites.iter().filter(|s| s.1[0] >= 176.0 && s.1[1] >= 150.0).collect();
        assert_eq!(bars.len(), 1, "only the occupied slot has a bar");
        assert_eq!((bars[0].1, bars[0].2), ([176.0, 158.0, 3.0, 8.0], [28.0, 62.0, 3.0, 8.0]), "half of 16 pixels, drawn from the bottom of slot 1");
    }

    #[test]
    fn a_tiled_fluid_restarts_its_texture_every_sixteen_pixels_and_clips_the_last_tile_of_a_row_and_column() {
        let mut canvas = Recorder::default();
        let iron = Identifier::parse("tconstruct:molten_iron").unwrap();
        draw_tiled_fluid(&mut canvas, &iron, [10.0, 20.0, 40.0, 40.0], 0xFF12_3456);
        // 40 pixels is two full tiles and an 8 pixel rest, in both directions; every tile reads the still
        // texture from its top left, clipped to the tile's own size
        let mut expected = Vec::new();
        for (top, height) in [(20.0, 16.0), (36.0, 16.0), (52.0, 8.0)] {
            for (left, width) in [(10.0, 16.0), (26.0, 16.0), (42.0, 8.0)] {
                expected.push(("tconstruct:fluid/molten/ore/iron/still".to_owned(), [0.0, 0.0, width, height], [left, top, width, height], 0xFF12_3456));
            }
        }
        assert_eq!(canvas.tinted, expected);
        // a fluid with no texture data of its own draws nothing (the caller's fill stays)
        let mut canvas = Recorder::default();
        draw_tiled_fluid(&mut canvas, &Identifier::parse("test:unknown").unwrap(), [0.0, 0.0, 32.0, 32.0], 0xFFFF_FFFF);
        assert!(canvas.tinted.is_empty());
    }

    #[test]
    fn tooltips_name_the_amount_the_fuel_and_the_slot_problem() {
        pluto_lib::Translations::register("en_us", [
            ("gui.tconstruct.fluid.millibucket".to_owned(), "%s mb".to_owned()),
            ("gui.tconstruct.melting.fuel.temperature".to_owned(), "Temperature: %s°C".to_owned()),
            ("gui.tconstruct.melting.no_heat".to_owned(), "Not enough heat to melt this item.".to_owned()),
        ]);
        let values = data(&[(0, 100), (2, 800), (DATA_TANK_AMOUNT, 90), (slot_data(0), 0), (slot_data(0) + 1, 600), (slot_data(0) + 2, 900), (slot_data(0) + 3, 1)]);
        let mut lines = Vec::new();
        melter_tooltip([100.0, 67.0], &values, &mut lines);
        assert_eq!(lines, vec!["90 mb".to_owned()], "over the fluid: an undefined fluid has no name line, just the amount");
        lines.clear();
        melter_tooltip([100.0, 30.0], &values, &mut lines);
        assert_eq!(lines.len(), 4, "over the empty part: the capacity and the room left, each with its amount: {lines:?}");
        assert!(lines[1].contains(&MELTER_CAPACITY.to_string()));
        lines.clear();
        melter_tooltip([158.0, 40.0], &values, &mut lines);
        assert_eq!(lines.len(), 2, "solid fuel and its temperature");
        assert!(lines[1].contains("800"));
        lines.clear();
        melter_tooltip([SLOT_X - 4.0, SLOT_Y + 3.0], &values, &mut lines);
        assert_eq!(lines, vec!["Not enough heat to melt this item.".to_owned()], "the ingot wants 900 and the fuel gives 800");
    }

    #[test]
    fn the_fuel_link_answers_only_a_fuel_tank_below() {
        let mut neighbours = MemoryNeighbourOps::default();
        neighbours.states.insert([0, 63, 0], "tconstruct:seared_heater".to_owned());
        neighbours.states.insert([5, 63, 0], "minecraft:chest".to_owned());
        neighbours.states.insert([9, 63, 0], "tconstruct:seared_fuel_tank".to_owned());
        let mut world = MemoryWorldOps::default();
        world.tags.push(("tconstruct:seared_heater".to_owned(), "tconstruct:fuel_tanks".to_owned()));
        world.tags.push(("tconstruct:seared_fuel_tank".to_owned(), "tconstruct:fuel_tanks".to_owned()));
        // The scopes nest as the transport nests them around the callback: the world outside,
        // the neighbours inside (the tag is read through the world scope).
        with_world(&mut world, || {
            with_neighbours(&mut neighbours, || {
                assert_eq!(fuel_links([0, 64, 0]), vec![[0, 63, 0]], "a heater below lends its slot 0");
                assert_eq!(fuel_links([9, 64, 0]), vec![[9, 63, 0]], "a seared fuel tank below lends its slot 0");
                assert_eq!(fuel_links([5, 64, 0]), Vec::<BlockPos>::new(), "a chest below lends nothing: its slot 0 stays untouched");
                assert_eq!(fuel_links([3, 64, 0]), Vec::<BlockPos>::new(), "air below lends nothing");
            });
        });
    }

    #[test]
    fn the_registered_types_carry_the_slots_tank_and_menus_of_the_original() {
        register();
        let melter = BlockEntityTypeRegistry::get("tconstruct:seared_melter").expect("melter registered");
        assert_eq!(melter.id.to_string(), "tconstruct:seared_melter");
        let heater = BlockEntityTypeRegistry::get("tconstruct:seared_heater").expect("heater registered");
        assert_eq!(heater.id.to_string(), "tconstruct:seared_heater");
        let window = ScreenHandlerRegistry::get("tconstruct:seared_melter").expect("melter window");
        assert_eq!(window.layout.slots.len(), 4);
        assert_eq!(window.layout.slots[2], Some([22.0, 52.0]));
        assert_eq!(window.layout.slots[3], Some([151.0, 32.0]), "the fuel slot at the original's place");
        assert_eq!(window.layout.source(3), pluto_lib::SlotSource::NeighbourBlockEntity([0, -1, 0]), "backed by the block below");
        assert_eq!(ScreenHandlerRegistry::get("tconstruct:seared_heater").unwrap().layout.slots, vec![Some([80.0, 20.0])]);
    }
}
