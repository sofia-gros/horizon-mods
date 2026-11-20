//! The melter and heater block entities, their windows and slot rules over PlutoLib's public API
//! (SlimeKnights MelterBlockEntity, HeaterBlockEntity, MelterContainerMenu, SingleItemContainerMenu, MelterScreen and
//! the GuiMeltingModule/GuiFuelModule/GuiTankModule it draws; MIT, see LICENSE). The melter menu holds its three melting
//! slots; the original's fourth slot there is the heater's own and lives in the heater's window. The tank is drawn as a
//! flat fill in the fluid's colour and its tooltip names the amount; putting fluid in and out with a held item needs fluid
//! items, which come with the casting unit.
use crate::{
    common,
    melter::{self, slot_data, Live, State, Tank, DATA_SLOTS, DATA_TANK_AMOUNT, DATA_TANK_FLUID, SLOTS},
    heating::MELTER_CAPACITY,
};
use pluto_lib::{
    BlockEntityType, BlockEntityTypeRegistry, FluidRegistry, I18n, Identifier, ScreenCanvas, ScreenHandlerRegistry, ScreenHandlerType,
    ScreenLayout,
};

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

fn tank_fluid(data: &[i32]) -> Option<Identifier> {
    let which = data.get(DATA_TANK_FLUID).copied().unwrap_or(0);
    (which > 0).then(|| melter::fluid_order().get(which as usize - 1).cloned()).flatten()
}

/// Draws the melter window: background, fuel gauge and flame, tank fill and scale, and the heat bar of every occupied slot.
pub(crate) fn draw_melter(canvas: &mut dyn ScreenCanvas, origin: [f32; 2], data: &[i32]) {
    let get = |index: usize| data.get(index).copied().unwrap_or(0);
    let [x, y] = origin;
    canvas.sprite(MELTER_BACKGROUND, [0.0, 0.0, 176.0, 166.0], [x, y, 176.0, 166.0]);
    // no fuel slot in this window: the gauge's own background (FUEL_TANK)
    canvas.sprite(MELTER_BACKGROUND, [194.0, 52.0, 14.0, 38.0], [x + 152.0, y + 31.0, 14.0, 38.0]);
    let (fuel, quality) = (get(0), get(1));
    if fuel > 0 && quality > 0 {
        let height = (14 * fuel / quality).clamp(0, 14) as f32;
        canvas.sprite(MELTER_BACKGROUND, [176.0, 136.0 + 14.0 - height, 14.0, height], [x + FLAME_AT[0], y + FLAME_AT[1] + 14.0 - height, 14.0, height]);
    }
    let amount = get(DATA_TANK_AMOUNT);
    if amount > 0 {
        let height = (TANK[3] * (amount.min(MELTER_CAPACITY) as f32) / MELTER_CAPACITY as f32).floor();
        let color = tank_fluid(data).map_or(0xFF_FF_FF_FF, |fluid| fluid_color(&fluid));
        canvas.fill([x + TANK[0], y + TANK[1] + TANK[3] - height, TANK[2], height], color);
    }
    canvas.sprite(MELTER_BACKGROUND, [176.0, 0.0, 52.0, 52.0], [x + TANK[0], y + TANK[1], 52.0, 52.0]);
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

/// Tooltip lines for a point of the melter window (GuiTankModule, GuiFuelModule, GuiMeltingModule.drawHeatTooltips).
pub(crate) fn melter_tooltip(at: [f32; 2], data: &[i32], lines: &mut Vec<String>) {
    let get = |index: usize| data.get(index).copied().unwrap_or(0);
    if inside(at, [TANK[0] - 1.0, TANK[1] - 1.0, TANK[2] + 2.0, TANK[3] + 2.0]) {
        if let Some(fluid) = tank_fluid(data) {
            lines.push(FluidRegistry::display_name(&fluid.to_string()).unwrap_or_else(|| fluid.to_string()));
        }
        lines.push(I18n::translate_args("gui.tconstruct.fluid.millibucket", &[&get(DATA_TANK_AMOUNT).to_string()]));
    } else if inside(at, [FUEL_AT[0] - 1.0, FUEL_AT[1] - 1.0, FUEL_AT[2] + 2.0, FUEL_AT[3] + 2.0]) {
        let temperature = get(2);
        if get(0) > 0 && temperature > 0 {
            lines.push(I18n::translate("gui.tconstruct.melting.fuel.solid"));
            lines.push(format!("§7§o{}", I18n::translate_args("gui.tconstruct.melting.fuel.temperature", &[&temperature.to_string()])));
        } else {
            lines.push(I18n::translate("gui.tconstruct.melting.fuel.empty"));
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
    state.publish(tick, &tank);
}

pub(crate) fn register() {
    let melter_id = Identifier::new("tconstruct", MELTER).expect("valid melter id");
    let layout = (0..SLOTS).fold(ScreenLayout::new(176.0, 166.0).with_own_background(), |layout, slot| layout.slot(SLOT_X, SLOT_Y + 18.0 * slot as f32));
    let window = ScreenHandlerType::new(melter_id.clone(), layout)
        .on_slot_rule(|rule| { rule.max_stack = 1; })
        .on_draw(|draw| draw_melter(draw.canvas, draw.origin, draw.data))
        .on_tooltip(|tip| melter_tooltip(tip.at, tip.data, tip.lines));
    ScreenHandlerRegistry::register(window);
    BlockEntityTypeRegistry::register(
        BlockEntityType::new(melter_id).item_slots(SLOTS as u32).fluid_tanks([MELTER_CAPACITY as u32]).menu("tconstruct:melter").ticker(tick_melter),
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

    #[derive(Default)]
    struct Recorder { sprites: Vec<(String, [f32; 4], [f32; 4])>, fills: Vec<([f32; 4], u32)> }
    impl ScreenCanvas for Recorder {
        fn fill(&mut self, rect: [f32; 4], argb: u32) { self.fills.push((rect, argb)); }
        fn sprite(&mut self, name: &str, src: [f32; 4], dst: [f32; 4]) -> bool { self.sprites.push((name.to_owned(), src, dst)); true }
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
        draw_melter(&mut canvas, [10.0, 20.0], &values);
        let flame = canvas.sprites.iter().find(|s| s.1[0] == 176.0 && s.1[2] == 14.0).expect("flame");
        assert_eq!((flame.1[3], flame.2), (7.0, [162.0, 42.0, 14.0, 7.0]), "the flame grows upward from its base");
        assert_eq!(canvas.fills, vec![([100.0, 62.0, 52.0, 26.0], 0xFFFF_FFFF)], "half the 52 pixel scale, white for a fluid no mod defined");
        let bars: Vec<_> = canvas.sprites.iter().filter(|s| s.1[0] >= 176.0 && s.1[1] >= 150.0).collect();
        assert_eq!(bars.len(), 1, "only the occupied slot has a bar");
        assert_eq!((bars[0].1, bars[0].2), ([176.0, 158.0, 3.0, 8.0], [28.0, 62.0, 3.0, 8.0]), "half of 16 pixels, drawn from the bottom of slot 1");
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
        melter_tooltip([100.0, 30.0], &values, &mut lines);
        assert_eq!(lines, vec!["90 mb".to_owned()], "an undefined fluid has no name line, just the amount");
        lines.clear();
        melter_tooltip([158.0, 40.0], &values, &mut lines);
        assert_eq!(lines.len(), 2, "solid fuel and its temperature");
        assert!(lines[1].contains("800"));
        lines.clear();
        melter_tooltip([SLOT_X - 4.0, SLOT_Y + 3.0], &values, &mut lines);
        assert_eq!(lines, vec!["Not enough heat to melt this item.".to_owned()], "the ingot wants 900 and the fuel gives 800");
    }

    #[test]
    fn the_registered_types_carry_the_slots_tank_and_menus_of_the_original() {
        register();
        let melter = BlockEntityTypeRegistry::get("tconstruct:seared_melter").expect("melter registered");
        assert_eq!(melter.id.to_string(), "tconstruct:seared_melter");
        let heater = BlockEntityTypeRegistry::get("tconstruct:seared_heater").expect("heater registered");
        assert_eq!(heater.id.to_string(), "tconstruct:seared_heater");
        let window = ScreenHandlerRegistry::get("tconstruct:seared_melter").expect("melter window");
        assert_eq!(window.layout.slots.len(), 3);
        assert_eq!(window.layout.slots[2], Some([22.0, 52.0]));
        assert_eq!(ScreenHandlerRegistry::get("tconstruct:seared_heater").unwrap().layout.slots, vec![Some([80.0, 20.0])]);
    }
}
