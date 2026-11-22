//! The scorched alloyer block entity and its window over PlutoLib's public API (SlimeKnights AlloyerBlockEntity,
//! AlloyerContainerMenu, AlloyerScreen and the GuiFuelModule/GuiTankModule it draws; MIT, see LICENSE). The window
//! shows the output tank and the input tanks beside the block with their fluids' still textures tiled over flat
//! fills of their colours, and the fuel gauge; it has no slots of its own (the
//! original's fuel slot is the heater's own and lives in the heater's window, as with the melter). Putting fluid in
//! and out by clicking a tank with a held container is not ported; the block itself trades with a held container on
//! use, like the melter. The fuel gauge's fluid fill is not drawn, again as with the melter's window.
use crate::{
    alloyer::{self, Env, Side, State, Tank, ALLOYER_CAPACITY, DATA_INPUTS, DATA_OUTPUT_AMOUNT, DATA_OUTPUT_FLUID, DATA_SLOTS, MAX_INPUTS},
    common,
    fluid_ingredient::FluidTags,
    fluid_items,
    melter::{self, FuelSource},
    melting_recipes::Alloy,
    smeltery_data::Data,
};
use pluto_lib::{
    BlockEntityTick, BlockEntityType, BlockEntityTypeRegistry, BlockPos, BlockState, Direction, FluidRegistry, I18n, Identifier,
    Neighbours, ScreenCanvas, ScreenHandlerRegistry, ScreenHandlerType, ScreenLayout, World,
};

pub(crate) const ALLOYER: &str = "scorched_alloyer";
const BACKGROUND: &str = "tconstruct:gui/alloyer";
/// AlloyerScreen: the output tank, the input tanks' left edges, the fuel gauge and the flame.
const OUTPUT: [f32; 4] = [114.0, 16.0, 34.0, 52.0];
const INPUT_X: [f32; 5] = [54.0, 22.0, 38.0, 70.0, 6.0];
const INPUT: [f32; 2] = [14.0, 52.0];
const FUEL_AT: [f32; 4] = [153.0, 32.0, 12.0, 36.0];
const FLAME_AT: [f32; 2] = [152.0, 15.0];
/// The sides an alloyer takes inputs from, in the original's order (Direction order without down).
const SIDES: [Direction; 5] = [Direction::Up, Direction::North, Direction::South, Direction::West, Direction::East];
const ALLOYER_TANKS: &str = "tconstruct:alloyer_tanks";
const FUEL_TANKS: &str = "tconstruct:fuel_tanks";

/// The live surroundings of a ticking alloyer: the loaded recipes and tags, the block below and the tanks beside it.
pub(crate) struct Live<'a> {
    pub data: &'a Data,
    pub pos: BlockPos,
    below: Option<Option<melter::Below>>,
}

impl<'a> Live<'a> {
    pub(crate) fn new(data: &'a Data, pos: BlockPos) -> Self { Live { data, pos, below: None } }

    fn own_state(&self) -> Option<BlockState> { World::ticking()?.get_block_state(self.pos) }
}

impl Env for Live<'_> {
    fn liquid_fuel(&self, fluid: &Identifier) -> Option<melter::LiquidFuel> {
        let in_tag = |fluid: &Identifier, tag: &Identifier| self.data.tags.contains(fluid, tag);
        let fuel = self.data.recipes.find_fuel(fluid, &in_tag)?;
        Some(melter::LiquidFuel { amount: fuel.amount(fluid, &in_tag), duration: fuel.duration, temperature: fuel.temperature, rate: fuel.rate })
    }
    fn solid_fuel(&self) -> Option<melter::SolidFuel> {
        self.data.recipes.solid_fuel.as_ref().map(|fuel| melter::SolidFuel { temperature: fuel.temperature, rate: fuel.rate })
    }
    fn burn_ticks(&self, item: &Identifier) -> i32 { melter::burn_time(item) }
    fn remainder(&self, item: &Identifier) -> Option<Identifier> { melter::fuel_remainder(item) }
    fn source(&mut self) -> Option<&mut dyn FuelSource> {
        let pos = self.pos;
        self.below.get_or_insert_with(|| melter::Below::of(pos)).as_mut().map(|below| below as &mut dyn FuelSource)
    }
    fn alloys(&self) -> &[Alloy] { &self.data.recipes.alloys }
    fn fluid_tags(&self) -> &FluidTags { &self.data.tags }
    fn sides(&mut self) -> Vec<Side> {
        let mut sides = Vec::new();
        for direction in SIDES {
            let (pos, face) = Neighbours::beside(self.pos, direction);
            let Some(state) = Neighbours::block_state(pos) else { continue };
            if !state.is_in_tag(ALLOYER_TANKS) { continue; }
            let Some(tanks) = Neighbours::fluid_tanks(pos, face) else { continue };
            let first = tanks.into_iter().next();
            sides.push(Side {
                pos,
                face,
                fluid: first.as_ref().and_then(|tank| tank.fluid.as_ref()).map(|fluid| (fluid.fluid.clone(), fluid.amount.min(i32::MAX as u32) as i32)),
                capacity: first.map_or(0, |tank| tank.capacity.min(i32::MAX as u32) as i32),
            });
        }
        sides
    }
    fn drain_side(&mut self, pos: BlockPos, face: Direction, amount: i32) -> i32 {
        Neighbours::drain_fluid(pos, face, amount.max(0) as u32).map_or(0, |drained| drained.amount as i32)
    }
    fn block_state(&self) -> (bool, bool) {
        self.own_state().map_or((false, false), |state| (state.property("in_structure") == Some("true"), state.property("active") == Some("true")))
    }
    fn set_active(&mut self, active: bool) {
        let flag = if active { "true" } else { "false" };
        let Some(world) = World::ticking() else { return };
        if let Some(state) = self.own_state() {
            if state.property("active") != Some(flag) { world.set_block_state(self.pos, &state.with("active", flag)); }
        }
        // the block below shows the same flame state while it is a fuel tank (AlloyerBlockEntity.tick)
        let (below, _) = Neighbours::beside(self.pos, Direction::Down);
        if let Some(state) = world.get_block_state(below) {
            if state.is_in_tag(FUEL_TANKS) && state.property("active").is_some_and(|value| value != flag) {
                world.set_block_state(below, &state.with("active", flag));
            }
        }
    }
}

/// The alloyer's ticker: loads its saved state, runs one game tick of the cycle against the loaded recipes, the
/// block below and the tanks beside it, saves it and publishes the numbers its window draws.
fn tick_alloyer(tick: &mut BlockEntityTick<'_>) {
    let Some(data) = common::smeltery_data() else { return };
    let Ok(data) = data.read() else { return };
    if !data.loaded() { return; }
    let mut state = State::load(tick.data, &data.recipes.alloys);
    let mut tank = Tank::read(tick);
    let mut live = Live::new(&data, tick.pos);
    state.step(&mut tank, &mut live);
    let sides = live.sides();
    state.save(tick.data, &data.recipes.alloys);
    tank.write(tick);
    state.publish(tick, &tank, &sides);
}

/// The colour a fluid is drawn in: its defined colour, opaque.
fn fluid_color(id: &Identifier) -> u32 {
    let color = FluidRegistry::get(&id.to_string()).map_or(0xFF_FF_FF_FF, |def| def.attributes.color);
    if color >> 24 == 0 { color | 0xFF00_0000 } else { color }
}

/// The fluid at a published fluid index (one more than its position in `fluid_order`, 0 for none).
fn fluid_at(data: &[i32], fluid_index: usize) -> Option<Identifier> {
    let which = data.get(fluid_index).copied().unwrap_or(0);
    (which > 0).then(|| melter::fluid_order().get(which as usize - 1).cloned()).flatten()
}

fn get(data: &[i32], index: usize) -> i32 { data.get(index).copied().unwrap_or(0) }

/// Draws the alloyer window: background, the input tanks' scales and fluids, the output tank and its scale, the
/// fuel gauge's background and the flame (AlloyerScreen.renderBg).
pub(crate) fn draw_alloyer(canvas: &mut dyn ScreenCanvas, origin: [f32; 2], data: &[i32]) {
    let [x, y] = origin;
    canvas.sprite(BACKGROUND, [0.0, 0.0, 176.0, 166.0], [x, y, 176.0, 166.0]);
    // no fuel slot in this window: the gauge's own background (FUEL_TANK), as the melter draws
    canvas.sprite(BACKGROUND, [194.0, 52.0, 14.0, 38.0], [x + 152.0, y + 31.0, 14.0, 38.0]);
    let (fuel, quality) = (get(data, 0), get(data, 1));
    if fuel > 0 && quality > 0 {
        let height = (14 * fuel / quality).clamp(0, 14) as f32;
        canvas.sprite(BACKGROUND, [176.0, 136.0 + 14.0 - height, 14.0, height], [x + FLAME_AT[0], y + FLAME_AT[1] + 14.0 - height, 14.0, height]);
    }
    let inputs = (get(data, DATA_INPUTS).max(0) as usize).min(MAX_INPUTS);
    for index in 0..inputs {
        canvas.sprite(BACKGROUND, [208.0, 52.0, 16.0, 54.0], [x + INPUT_X[index] - 1.0, y + 15.0, 16.0, 54.0]);
    }
    for index in 0..inputs {
        let base = alloyer::input_data(index);
        let (amount, capacity) = (get(data, base), get(data, base + 2));
        if amount > 0 && capacity > 0 {
            let height = (INPUT[1] * (amount.min(capacity) as f32) / capacity as f32).floor();
            let fluid = fluid_at(data, base + 1);
            let color = fluid.as_ref().map_or(0xFF_FF_FF_FF, |fluid| fluid_color(fluid));
            let rect = [x + INPUT_X[index], y + 16.0 + INPUT[1] - height, INPUT[0], height];
            canvas.fill(rect, color);
            if let Some(fluid) = &fluid { crate::melter_menu::draw_tiled_fluid(canvas, fluid, rect, color); }
        }
    }
    let amount = get(data, DATA_OUTPUT_AMOUNT);
    if amount > 0 {
        let height = (OUTPUT[3] * (amount.min(ALLOYER_CAPACITY) as f32) / ALLOYER_CAPACITY as f32).floor();
        let fluid = fluid_at(data, DATA_OUTPUT_FLUID);
        let color = fluid.as_ref().map_or(0xFF_FF_FF_FF, |fluid| fluid_color(fluid));
        let rect = [x + OUTPUT[0], y + OUTPUT[1] + OUTPUT[3] - height, OUTPUT[2], height];
        canvas.fill(rect, color);
        if let Some(fluid) = &fluid { crate::melter_menu::draw_tiled_fluid(canvas, fluid, rect, color); }
    }
    canvas.sprite(BACKGROUND, [176.0, 0.0, 34.0, 52.0], [x + OUTPUT[0], y + OUTPUT[1], 34.0, 52.0]);
}

fn inside(at: [f32; 2], rect: [f32; 4]) -> bool { at[0] >= rect[0] && at[0] < rect[0] + rect[2] && at[1] >= rect[1] && at[1] < rect[1] + rect[3] }

/// The window's hover text: a tank's fluid and amount, or the fuel's state (GuiTankModule and GuiFuelModule,
/// worded as the melter's window words them).
pub(crate) fn alloyer_tooltip(at: [f32; 2], data: &[i32], lines: &mut Vec<String>) {
    let tank = |amount_index: usize, fluid_index: usize, lines: &mut Vec<String>| {
        let amount = get(data, amount_index);
        if amount <= 0 { return false; }
        if let Some(fluid) = fluid_at(data, fluid_index) {
            lines.push(FluidRegistry::display_name(&fluid.to_string()).unwrap_or_else(|| fluid.to_string()));
        }
        lines.push(I18n::translate_args("gui.tconstruct.fluid.millibucket", &[&amount.to_string()]));
        true
    };
    if inside(at, [OUTPUT[0] - 1.0, OUTPUT[1] - 1.0, OUTPUT[2] + 2.0, OUTPUT[3] + 2.0]) && tank(DATA_OUTPUT_AMOUNT, DATA_OUTPUT_FLUID, lines) { return; }
    let inputs = (get(data, DATA_INPUTS).max(0) as usize).min(MAX_INPUTS);
    for index in 0..inputs {
        let rect = [INPUT_X[index] - 1.0, 15.0, INPUT[0] + 2.0, INPUT[1] + 2.0];
        if inside(at, rect) && tank(alloyer::input_data(index), alloyer::input_data(index) + 1, lines) { return; }
    }
    if inside(at, [FUEL_AT[0] - 1.0, FUEL_AT[1] - 1.0, FUEL_AT[2] + 2.0, FUEL_AT[3] + 2.0]) {
        let temperature = get(data, 2);
        if get(data, 0) > 0 && temperature > 0 {
            lines.push(I18n::translate("gui.tconstruct.melting.fuel.solid"));
            lines.push(format!("§7§o{}", I18n::translate_args("gui.tconstruct.melting.fuel.temperature", &[&temperature.to_string()])));
        } else {
            lines.push(I18n::translate("gui.tconstruct.melting.fuel.empty"));
        }
    }
}

pub(crate) fn register() {
    let alloyer_id = Identifier::new("tconstruct", ALLOYER).expect("valid alloyer id");
    let layout = ScreenLayout::new(176.0, 166.0).with_own_background();
    let window = ScreenHandlerType::new(alloyer_id.clone(), layout)
        .on_draw(|draw| draw_alloyer(draw.canvas, draw.origin, draw.data))
        .on_tooltip(|tip| alloyer_tooltip(tip.at, tip.data, tip.lines));
    ScreenHandlerRegistry::register(window);
    BlockEntityTypeRegistry::register(
        BlockEntityType::new(alloyer_id).fluid_tanks([ALLOYER_CAPACITY as u32]).menu("tconstruct:alloyer").sync_data()
            .ticker(tick_alloyer).on_use(|used| fluid_items::use_tank(used, ALLOYER_CAPACITY as u32)),
    );
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
    fn the_alloyer_window_draws_the_output_inputs_and_flame() {
        let mut canvas = Recorder::default();
        // half the fuel left (flame 7 high), the output half full, two inputs (one half full, one empty)
        let half = alloyer::input_data(0);
        let values = data(&[(0, 200), (1, 400), (2, 800), (DATA_OUTPUT_AMOUNT, ALLOYER_CAPACITY / 2),
            (DATA_OUTPUT_FLUID, 1), (DATA_INPUTS, 2), (half, 500), (half + 1, 1), (half + 2, 1000)]);
        draw_alloyer(&mut canvas, [10.0, 20.0], &values);
        assert!(canvas.sprites.iter().all(|sprite| sprite.0 == BACKGROUND), "every sprite is the window's own texture: {:?}", canvas.sprites);
        let flame = canvas.sprites.iter().find(|s| s.1[0] == 176.0 && s.1[2] == 14.0).expect("flame");
        assert_eq!((flame.1, flame.2), ([176.0, 143.0, 14.0, 7.0], [162.0, 42.0, 14.0, 7.0]), "half the fuel draws half the flame");
        assert!(canvas.sprites.iter().any(|s| s.1 == [176.0, 0.0, 34.0, 52.0]), "the output tank's scale is drawn");
        assert!(canvas.sprites.iter().any(|s| s.1 == [208.0, 52.0, 16.0, 54.0]), "the input tanks' scales are drawn");
        let fills: Vec<_> = canvas.fills.iter().map(|fill| fill.0).collect();
        assert_eq!(fills, vec![[64.0, 62.0, 14.0, 26.0], [124.0, 62.0, 34.0, 26.0]], "the half-full input and output draw fluid");
    }

    #[test]
    fn the_alloyer_hover_text_names_the_tanks_and_the_fuel() {
        pluto_lib::Translations::register("en_us", [
            ("gui.tconstruct.fluid.millibucket".to_owned(), "%s mb".to_owned()),
            ("gui.tconstruct.melting.fuel.temperature".to_owned(), "Temperature: %s°C".to_owned()),
            ("gui.tconstruct.melting.fuel.empty".to_owned(), "No fuel".to_owned()),
        ]);
        let base = alloyer::input_data(0);
        let values = data(&[(DATA_OUTPUT_AMOUNT, 500), (DATA_INPUTS, 1), (base, 250), (base + 2, 1000)]);
        let mut lines = Vec::new();
        alloyer_tooltip([120.0, 30.0], &values, &mut lines);
        assert_eq!(lines, vec!["500 mb".to_owned()], "the output tank's hover text: {lines:?}");
        lines.clear();
        alloyer_tooltip([60.0, 30.0], &values, &mut lines);
        assert_eq!(lines, vec!["250 mb".to_owned()], "an input tank's hover text: {lines:?}");
        lines.clear();
        alloyer_tooltip([159.0, 40.0], &values, &mut lines);
        assert_eq!(lines, vec!["No fuel".to_owned()], "an unfueled alloyer says so: {lines:?}");
        let fueled = data(&[(0, 100), (2, 800)]);
        lines.clear();
        alloyer_tooltip([159.0, 40.0], &fueled, &mut lines);
        assert_eq!(lines.len(), 2, "solid fuel and its temperature");
        assert!(lines[1].contains("800"));
    }

    #[test]
    fn the_alloyer_is_a_fluid_tank_with_a_window_and_a_ticker() {
        register();
        let entity = BlockEntityTypeRegistry::get("tconstruct:scorched_alloyer").expect("the alloyer block entity");
        assert_eq!(entity.id.to_string(), "tconstruct:scorched_alloyer");
        let window = ScreenHandlerRegistry::get("tconstruct:scorched_alloyer").expect("the alloyer window");
        assert!(window.layout.slots.is_empty(), "the alloyer window has no slots of its own");
    }
}
