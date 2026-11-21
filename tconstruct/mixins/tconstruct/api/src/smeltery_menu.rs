//! The smeltery and foundry controllers as block entities, with their windows over PlutoLib's public API (SlimeKnights
//! HeatingStructureBlockEntity, HeatingStructureContainerMenu, HeatingStructureScreen, HeatingStructureSideInventoryScreen and
//! the GuiSmelteryTank, GuiFuelModule and GuiMeltingModule it draws; MIT, see LICENSE). The window shows the tank as stacked
//! flat fills in the fluids' colours (click a fluid to move it to the bottom), the fuel gauge, and the melting slots with
//! their heat bars in a side panel of up to 4 columns of 7. The bucket slots and the fill and empty clicks wait for fluid items.
use crate::{
    common,
    fluid_values::OreRate,
    melter::{fluid_order, melt_stack, LiquidFuel, Melt},
    melting_recipes::Alloy,
    fluid_ingredient::FluidTags,
    multiblock::{Cell, Pos},
    smeltery::{Controller, Entities, Env, Kind, MAX_SLOTS},
    smeltery_data::Data,
    smeltery_tank::SmelteryTank,
};
use pluto_lib::{
    BlockEntityTick, BlockEntityType, BlockEntityTypeRegistry, BlockState, Direction, FluidRegistry, I18n, Identifier, ItemSlot, Nbt, NbtCompound,
    NbtCompoundExt, Neighbours, ScreenCanvas, ScreenHandlerRegistry, ScreenHandlerType, ScreenLayout, World,
};

pub(crate) const SMELTERY: &str = "smeltery_controller";
pub(crate) const FOUNDRY: &str = "foundry_controller";
const BACKGROUND: &str = "tconstruct:gui/heating_structure";

/// Menu data slots (at most 256, each a signed 16 bit value): the fuel module's four, the fuel tank's amount and capacity (high and
/// low halves, as a value can exceed 32767), its fluid, whether the structure is formed, the number of melting slots, the tank's
/// capacity, then the fluids of the tank (position in `fluid_order` plus one, amount high and low) and the melting slots shown.
const D_FUEL_AMOUNT: usize = 4;
const D_FUEL_CAPACITY: usize = 6;
const D_FUEL_FLUID: usize = 8;
const D_FORMED: usize = 9;
const D_SLOTS: usize = 10;
const D_CAPACITY: usize = 11;
const D_FLUID_COUNT: usize = 13;
const D_FLUIDS: usize = 14;
const MAX_FLUIDS: usize = 16;
const D_HEAT: usize = 64;
/// HeatingStructureContainerMenu.calcColumns: 7 slots to a column, 4 columns at most.
const VISIBLE: usize = 28;
const WIDTH: f32 = 176.0 + 4.0 * 18.0 + 14.0;
/// HeatingStructureScreen: the tank, the fuel gauge, the flame and the side slots (the side panel begins right of the window).
const TANK: [f32; 4] = [8.0, 16.0, 106.0, 106.0];
const FUEL: [f32; 4] = [152.0, 32.0, 16.0, 90.0];
const FLAME_AT: [f32; 2] = [153.0, 15.0];
const SIDE_X: f32 = 176.0 + 4.0;
const SIDE_Y: f32 = 4.0;
const BAR: [f32; 2] = [3.0, 16.0];

pub(crate) fn columns(slots: usize) -> usize { ((slots + 6) / 7).min(4) }

/// Where the melting slot `index` of a window with `slots` slots sits: column by column, 7 to a column, to the right of the window.
pub(crate) fn slot_position(index: usize, slots: usize) -> Option<[f32; 2]> {
    if index >= slots.min(VISIBLE) { return None; }
    let columns = columns(slots).max(1);
    let (row, column) = (index / columns, index % columns);
    Some([SIDE_X + 4.0 + 18.0 * column as f32, SIDE_Y + 18.0 * row as f32])
}

fn split(value: i32) -> (i32, i32) { (value >> 15, value & 0x7FFF) }
fn join(data: &[i32], index: usize) -> i32 { (data.get(index).copied().unwrap_or(0) << 15) | data.get(index + 1).copied().unwrap_or(0) }

fn set(tick: &mut BlockEntityTick<'_>, index: usize, value: i32) { tick.set_menu_data(index, value); }
fn set_pair(tick: &mut BlockEntityTick<'_>, index: usize, value: i32) {
    let (high, low) = split(value.max(0));
    set(tick, index, high);
    set(tick, index + 1, low);
}

fn fluid_index(id: &Identifier) -> i32 { fluid_order().iter().position(|own| own == id).map_or(0, |position| position as i32 + 1) }

// --- the live surroundings of a ticking controller

const SMELTERY_TAGS: [&str; 5] = ["tconstruct:smeltery", "tconstruct:smeltery/floor", "tconstruct:smeltery/tanks", "tconstruct:smeltery/wall", "tconstruct:structure_air"];
const FOUNDRY_TAGS: [&str; 5] = ["tconstruct:foundry", "tconstruct:foundry/floor", "tconstruct:foundry/tanks", "tconstruct:foundry/wall", "tconstruct:structure_air"];

/// What a block state is to the structure of `kind`: air, the `in_structure` property and the block tags.
pub(crate) fn classify(kind: Kind, state: &BlockState) -> Cell {
    let tags = match kind { Kind::Smeltery => SMELTERY_TAGS, Kind::Foundry => FOUNDRY_TAGS };
    let air = matches!(state.block.path(), "air" | "cave_air" | "void_air") || state.is_in_tag(tags[4]);
    Cell {
        air,
        in_structure: state.property("in_structure").map(|value| value == "true"),
        foreign: false,
        any: state.is_in_tag(tags[0]),
        floor: state.is_in_tag(tags[1]),
        tank: state.is_in_tag(tags[2]),
        wall: state.is_in_tag(tags[3]),
    }
}

pub(crate) struct Live<'a> {
    pub data: &'a Data,
    pub item_tags: Option<&'a pluto_lib::RegistryLookup>,
    entity_tags: Option<std::sync::Arc<pluto_lib::RegistryLookup>>,
    pos: Pos,
    seed: u32,
}

impl<'a> Live<'a> {
    pub(crate) fn new(data: &'a Data, item_tags: Option<&'a pluto_lib::RegistryLookup>, pos: Pos) -> Self {
        let seed = (pos[0] as u32).wrapping_mul(73856093) ^ (pos[1] as u32).wrapping_mul(19349663) ^ (pos[2] as u32).wrapping_mul(83492791) | 1;
        Live { data, item_tags, entity_tags: common::entity_type_snapshot(), pos, seed }
    }

    fn own_state(&self) -> Option<BlockState> { World::ticking()?.get_block_state(self.pos) }
}

impl Env for Live<'_> {
    fn cell(&self, kind: Kind, pos: Pos) -> Option<Cell> {
        let world = World::ticking()?;
        let state = world.get_block_state(pos)?;
        let mut cell = classify(kind, &state);
        // IServantLogic.isValidMaster: a drain, duct or tank of a structure that is still there answers to that controller only
        if cell.in_structure == Some(true) && crate::smeltery_io::keeps_master(state.block.path()) {
            if let Some(data) = world.block_entity_data(pos) {
                cell.foreign = crate::smeltery_io::foreign_to(&data, self.pos, |master| {
                    world.get_block_state(master).is_some_and(|controller| matches!(controller.block.path(), SMELTERY | FOUNDRY) && controller.property("in_structure") == Some("true"))
                });
            }
        }
        Some(cell)
    }

    fn mark(&mut self, pos: Pos, in_structure: bool) {
        let Some(world) = World::ticking() else { return };
        let Some(state) = world.get_block_state(pos) else { return };
        let wanted = if in_structure { "true" } else { "false" };
        if state.property("in_structure").is_some_and(|value| value != wanted) { world.set_block_state(pos, &state.clone().with("in_structure", wanted)); }
        // drains, ducts and tanks keep their master in their block entity (ServantTileEntity.setMaster)
        if crate::smeltery_io::keeps_master(state.block.path()) {
            world.merge_block_entity_data(pos, &crate::smeltery_io::master_data(in_structure.then_some(self.pos)));
        }
    }

    fn block_state(&self) -> (bool, bool) {
        self.own_state().map_or((false, false), |state| (state.property("in_structure") == Some("true"), state.property("active") == Some("true")))
    }

    fn set_block_state(&mut self, in_structure: bool, active: bool) {
        let (Some(world), Some(state)) = (World::ticking(), self.own_state()) else { return };
        let flag = |value: bool| if value { "true" } else { "false" };
        world.set_block_state(self.pos, &state.with("in_structure", flag(in_structure)).with("active", flag(active)));
    }

    fn melt(&self, kind: Kind, stack: &ItemSlot) -> Option<Melt> {
        let rate = match kind { Kind::Smeltery => OreRate::SMELTERY, Kind::Foundry => OreRate::FOUNDRY };
        melt_stack(self.data, self.item_tags, stack, &rate, &OreRate::FOUNDRY_BYPRODUCT).map(|(melt, _)| melt)
    }

    fn byproducts(&self, stack: &ItemSlot) -> Vec<(Identifier, i32)> {
        melt_stack(self.data, self.item_tags, stack, &OreRate::FOUNDRY, &OreRate::FOUNDRY_BYPRODUCT).map(|(_, byproducts)| byproducts).unwrap_or_default()
    }

    fn liquid_fuel(&self, fluid: &Identifier) -> Option<LiquidFuel> {
        let in_tag = |fluid: &Identifier, tag: &Identifier| self.data.tags.contains(fluid, tag);
        let fuel = self.data.recipes.find_fuel(fluid, &in_tag)?;
        Some(LiquidFuel { amount: fuel.amount(fluid, &in_tag), duration: fuel.duration, temperature: fuel.temperature, rate: fuel.rate })
    }

    fn alloys(&self) -> &[Alloy] { &self.data.recipes.alloys }
    fn fluid_tags(&self) -> &FluidTags { &self.data.tags }

    /// The tanks in the walls are asked through PlutoLib's neighbour reach, from their top face.
    fn tank_fluid(&mut self, pos: Pos) -> Option<(Identifier, i32)> {
        let stack = Neighbours::fluid_tanks(pos, Direction::Up)?.into_iter().next()?.fluid?;
        Some((stack.fluid, stack.amount.min(i32::MAX as u32) as i32))
    }

    fn tank_drain(&mut self, pos: Pos, _: &Identifier, amount: i32) -> i32 {
        Neighbours::drain_fluid(pos, Direction::Up, amount.max(0) as u32).map_or(0, |drained| drained.amount as i32)
    }

    fn entity_melting(&self, entity: &Identifier) -> Option<(Identifier, i32, i32)> {
        let in_tag = |entity: &Identifier, tag: &Identifier| self.entity_tags.as_ref().and_then(|tags| tags.get_tag(tag)).is_some_and(|members| members.contains(entity));
        let recipe = self.data.recipes.entity_melting.iter().find(|recipe| recipe.entity.matches(entity, &in_tag))?;
        Some((recipe.output.resolve(&self.data.tags)?, recipe.output.amount, recipe.damage))
    }

    fn melting_blacklisted(&self, entity: &Identifier) -> bool {
        let tag = Identifier::parse("tconstruct:melting/blacklist").expect("valid tag");
        self.entity_tags.as_ref().and_then(|tags| tags.get_tag(&tag)).is_some_and(|members| members.contains(entity))
    }

    /// No entity reach in tickers yet (see the docs): nothing is picked up or hurt.
    fn entities(&mut self) -> Option<&mut dyn Entities> { None }

    fn random(&mut self, bound: usize) -> usize {
        // xorshift32, seeded from the position: only the order of mutually exclusive alloys depends on it
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed as usize) % bound.max(1)
    }
}

// --- the ticker

fn tick_controller(kind: Kind, tick: &mut BlockEntityTick<'_>) {
    let Some(data) = common::smeltery_data() else { return };
    let Ok(data) = data.read() else { return };
    if !data.loaded() { return; }
    let item_tags = common::item_snapshot();
    let mut env = Live::new(&data, item_tags.as_deref(), tick.pos);
    let facing = match env.own_state().as_ref().and_then(|state| state.property("facing")) {
        Some("east") => Direction::East, Some("south") => Direction::South, Some("west") => Direction::West, _ => Direction::North,
    };
    let mut controller = Controller::load(kind, tick.pos, facing, tick.data, tick.items);
    controller.step(tick.items, &mut env);
    controller.save(tick.data);
    publish(&controller, tick, &mut env);
}

/// Everything the window draws, in menu data.
fn publish(controller: &Controller, tick: &mut BlockEntityTick<'_>, env: &mut Live<'_>) {
    for (index, value) in [controller.fuel.fuel, controller.fuel.quality, controller.fuel.temperature, controller.fuel.rate].into_iter().enumerate() { set(tick, index, value); }
    let info = controller.fuel_info(env, &|pos| Neighbours::fluid_tanks(pos, Direction::Up).and_then(|tanks| tanks.first().map(|tank| tank.capacity.min(i32::MAX as u32) as i32)).unwrap_or(0));
    set_pair(tick, D_FUEL_AMOUNT, info.as_ref().map_or(0, |info| info.1));
    set_pair(tick, D_FUEL_CAPACITY, info.as_ref().map_or(0, |info| info.2));
    set(tick, D_FUEL_FLUID, info.as_ref().map_or(0, |info| fluid_index(&info.0)));
    set(tick, D_FORMED, i32::from(controller.structure.is_some()));
    set(tick, D_SLOTS, controller.slots.len() as i32);
    set_pair(tick, D_CAPACITY, controller.tank.capacity());
    let fluids = controller.tank.fluids();
    set(tick, D_FLUID_COUNT, fluids.len().min(MAX_FLUIDS) as i32);
    for (index, fluid) in fluids.iter().take(MAX_FLUIDS).enumerate() {
        set(tick, D_FLUIDS + 3 * index, fluid_index(&fluid.fluid));
        set_pair(tick, D_FLUIDS + 3 * index + 1, fluid.amount);
    }
    for (index, slot) in controller.slots.iter().take(VISIBLE).enumerate() {
        let base = D_HEAT + 4 * index;
        set(tick, base, slot.current_time);
        set(tick, base + 1, slot.required_time);
        set(tick, base + 2, slot.required_temp);
        set(tick, base + 3, i32::from(slot.occupied));
    }
}

// --- the window

fn fluid_color(id: &Identifier) -> u32 {
    let color = FluidRegistry::get(&id.to_string()).map_or(0xFF_FF_FF_FF, |def| def.attributes.color);
    if color >> 24 == 0 { color | 0xFF00_0000 } else { color }
}

fn fluid_at(data: &[i32], list_index: usize) -> Option<Identifier> {
    let which = data.get(D_FLUIDS + 3 * list_index).copied().unwrap_or(0);
    (which > 0).then(|| fluid_order().get(which as usize - 1).cloned()).flatten()
}

fn inside(at: [f32; 2], rect: [f32; 4]) -> bool { at[0] >= rect[0] && at[0] < rect[0] + rect[2] && at[1] >= rect[1] && at[1] < rect[1] + rect[3] }

/// The stacked fluids of the tank from the bottom up, each as high as its share of the capacity: (list index, top, height).
pub(crate) fn tank_bands(data: &[i32]) -> Vec<(usize, f32, f32)> {
    let count = data.get(D_FLUID_COUNT).copied().unwrap_or(0).clamp(0, MAX_FLUIDS as i32) as usize;
    let contained: i32 = (0..count).map(|index| join(data, D_FLUIDS + 3 * index + 1)).sum();
    let capacity = join(data, D_CAPACITY).max(contained).max(1) as f32;
    let mut bottom = TANK[1] + TANK[3];
    let mut bands = Vec::new();
    for index in 0..count {
        let height = (TANK[3] * join(data, D_FLUIDS + 3 * index + 1) as f32 / capacity).floor();
        bottom -= height;
        bands.push((index, bottom, height));
    }
    bands
}

pub(crate) fn draw_window(canvas: &mut dyn ScreenCanvas, origin: [f32; 2], data: &[i32]) {
    let get = |index: usize| data.get(index).copied().unwrap_or(0);
    let [x, y] = origin;
    canvas.sprite(BACKGROUND, [0.0, 0.0, 176.0, 220.0], [x, y, 176.0, 220.0]);
    // the side panel behind the melting slots
    let slots = get(D_SLOTS).max(0) as usize;
    canvas.fill([x + SIDE_X, y + SIDE_Y - 4.0, WIDTH - SIDE_X, 18.0 * 7.0 + 8.0], 0xFF_C6_C6_C6);
    for index in 0..slots.min(VISIBLE) {
        if let Some([sx, sy]) = slot_position(index, slots) { canvas.sprite(BACKGROUND, [0.0, 238.0, 22.0, 18.0], [x + sx - 4.0, y + sy, 22.0, 18.0]); }
    }
    // fuel: flame and gauge
    let (fuel, quality) = (get(0), get(1));
    if fuel > 0 && quality > 0 {
        let height = (14 * fuel / quality).clamp(0, 14) as f32;
        canvas.sprite(BACKGROUND, [176.0, 136.0 + 14.0 - height, 14.0, height], [x + FLAME_AT[0], y + FLAME_AT[1] + 14.0 - height, 14.0, height]);
    }
    let (amount, capacity) = (join(data, D_FUEL_AMOUNT), join(data, D_FUEL_CAPACITY));
    if amount > 0 && capacity > 0 {
        let height = (FUEL[3] * amount.min(capacity) as f32 / capacity as f32).floor();
        let color = (get(D_FUEL_FLUID) > 0).then(|| fluid_color(&fluid_order()[get(D_FUEL_FLUID) as usize - 1])).unwrap_or(0xFF_FF_FF_FF);
        canvas.fill([x + FUEL[0], y + FUEL[1] + FUEL[3] - height, FUEL[2], height], color);
    }
    // the tank
    for (index, top, height) in tank_bands(data) {
        if height > 0.0 {
            canvas.fill([x + TANK[0], y + top, TANK[2], height], fluid_at(data, index).map_or(0xFF_FF_FF_FF, |fluid| fluid_color(&fluid)));
        }
    }
    canvas.sprite(BACKGROUND, [176.0, 0.0, 80.0, 106.0], [x + TANK[0], y + TANK[1], 80.0, 106.0]);
    // heat bars
    let temperature = get(2);
    for index in 0..slots.min(VISIBLE) {
        let base = D_HEAT + 4 * index;
        if get(base + 3) == 0 { continue; }
        let Some([sx, sy]) = slot_position(index, slots) else { continue };
        let (kind, progress) = match crate::melter_menu::bar_of(get(base), get(base + 1), get(base + 2), temperature) {
            crate::melter_menu::Bar::Progress(progress) => (0.0, progress),
            crate::melter_menu::Bar::NoHeat => (1.0, 1.0),
            crate::melter_menu::Bar::NoSpace => (2.0, 1.0),
            crate::melter_menu::Bar::Unmeltable => (3.0, 1.0),
        };
        let height = (BAR[1] * progress).floor();
        if height <= 0.0 { continue; }
        canvas.sprite(BACKGROUND, [176.0 + 3.0 * kind, 150.0 + BAR[1] - height, BAR[0], height], [x + sx - 4.0, y + sy + BAR[1] - height, BAR[0], height]);
    }
}

/// The fluid of the tank under a point of the window, if one is there (`getFluidClicked`).
pub(crate) fn fluid_clicked(at: [f32; 2], data: &[i32]) -> Option<usize> {
    if !inside(at, TANK) { return None; }
    tank_bands(data).into_iter().find(|(_, top, height)| *height > 0.0 && at[1] >= *top && at[1] < top + height).map(|(index, _, _)| index)
}

pub(crate) fn tooltip(at: [f32; 2], data: &[i32], lines: &mut Vec<String>) {
    let get = |index: usize| data.get(index).copied().unwrap_or(0);
    if let Some(index) = fluid_clicked(at, data) {
        if let Some(fluid) = fluid_at(data, index) {
            lines.push(FluidRegistry::display_name(&fluid.to_string()).unwrap_or_else(|| fluid.to_string()));
        }
        lines.push(I18n::translate_args("gui.tconstruct.fluid.millibucket", &[&join(data, D_FLUIDS + 3 * index + 1).to_string()]));
    } else if inside(at, [FUEL[0] - 1.0, FUEL[1] - 1.0, FUEL[2] + 2.0, FUEL[3] + 2.0]) {
        let amount = join(data, D_FUEL_AMOUNT);
        if amount > 0 {
            if let Some(fluid) = (get(D_FUEL_FLUID) > 0).then(|| fluid_order().get(get(D_FUEL_FLUID) as usize - 1).cloned()).flatten() {
                lines.push(FluidRegistry::display_name(&fluid.to_string()).unwrap_or_else(|| fluid.to_string()));
            }
            lines.push(I18n::translate_args("gui.tconstruct.fluid.millibucket", &[&amount.to_string()]));
            lines.push(format!("§7§o{}", I18n::translate_args("gui.tconstruct.melting.fuel.temperature", &[&get(2).to_string()])));
        } else {
            lines.push(I18n::translate("gui.tconstruct.melting.fuel.empty"));
        }
    } else {
        let slots = get(D_SLOTS).max(0) as usize;
        for index in 0..slots.min(VISIBLE) {
            let base = D_HEAT + 4 * index;
            let Some([sx, sy]) = slot_position(index, slots) else { continue };
            if get(base + 3) != 0 && inside(at, [sx - 5.0, sy - 1.0, BAR[0] + 1.0, BAR[1] + 2.0]) {
                let key = match crate::melter_menu::bar_of(get(base), get(base + 1), get(base + 2), get(2)) {
                    crate::melter_menu::Bar::Unmeltable => "gui.tconstruct.melting.no_recipe",
                    crate::melter_menu::Bar::NoHeat => "gui.tconstruct.melting.no_heat",
                    crate::melter_menu::Bar::NoSpace => "gui.tconstruct.melting.no_space",
                    crate::melter_menu::Bar::Progress(_) => return,
                };
                lines.push(I18n::translate(key));
                return;
            }
        }
    }
}

/// A click on a fluid sends its index plus 4 (ids 0 to 3 are the bucket button and the fuel and tank clicks, which need fluid items).
const FLUID_BUTTON: i32 = 4;

/// `clickMenuButton` for id 4 and up with nothing held: move that fluid to the bottom of the tank.
pub(crate) fn press(id: i32, data: &mut NbtCompound) {
    if id < FLUID_BUTTON { return; }
    let Some(tag) = data.get(Controller::TAG_TANK).and_then(Nbt::as_compound).cloned() else { return };
    let mut tank = SmelteryTank::default();
    tank.read(&tag);
    tank.move_fluid_to_bottom((id - FLUID_BUTTON) as usize);
    data.insert(Controller::TAG_TANK.to_owned(), Nbt::Compound(tank.write()));
}

pub(crate) fn register() {
    for (name, kind, title) in [(SMELTERY, Kind::Smeltery, "tconstruct:smeltery"), (FOUNDRY, Kind::Foundry, "tconstruct:foundry")] {
        let id = Identifier::new("tconstruct", name).expect("valid controller id");
        let mut layout = ScreenLayout::new(WIDTH, 220.0).inventory_at(8.0, 138.0).with_own_background();
        for index in 0..MAX_SLOTS { layout = match slot_position(index, VISIBLE) { Some([x, y]) => layout.slot(x, y), None => layout.hidden_slot() }; }
        let window = ScreenHandlerType::new(id.clone(), layout)
            .on_slot_rule(|rule| { rule.max_stack = 1; })
            .on_dynamic_slots(|state| {
                let slots = state.data.get(D_SLOTS).copied().unwrap_or(0).max(0) as usize;
                Some((0..MAX_SLOTS).map(|index| slot_position(index, slots)).collect())
            })
            .on_draw(|draw| draw_window(draw.canvas, draw.origin, draw.data))
            .on_click(|press| fluid_clicked(press.at, press.data).map(|index| index as i32 + FLUID_BUTTON))
            .on_tooltip(|tip| tooltip(tip.at, tip.data, tip.lines))
            .on_button(|button| press(button.id, button.data));
        let _ = title;
        ScreenHandlerRegistry::register(window);
        BlockEntityTypeRegistry::register(
            BlockEntityType::new(id).item_slots(MAX_SLOTS as u32).menu(&format!("tconstruct:{name}")).sync_data().ticker(move |tick| tick_controller(kind, tick))
                // the tank answers drains, faucets and pipes (ISmelteryTankHandler.getFluidCapability): see smeltery_io
                .fluid_reader(crate::smeltery_io::read_controller).fluid_filler(crate::smeltery_io::fill_controller).fluid_drainer(crate::smeltery_io::drain_controller),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::{with_world, MemoryWorldOps};

    #[derive(Default)]
    struct Recorder { sprites: Vec<(String, [f32; 4], [f32; 4])>, fills: Vec<([f32; 4], u32)> }
    impl ScreenCanvas for Recorder {
        fn fill(&mut self, rect: [f32; 4], argb: u32) { self.fills.push((rect, argb)); }
        fn sprite(&mut self, name: &str, src: [f32; 4], dst: [f32; 4]) -> bool { self.sprites.push((name.to_owned(), src, dst)); true }
        fn text(&mut self, _: &str, _: [f32; 2], _: u32, _: bool) {}
        fn text_width(&self, _: &str) -> f32 { 0.0 }
    }

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }

    fn data_with(entries: &[(usize, i32)]) -> Vec<i32> {
        let mut data = vec![0; 256];
        for (index, value) in entries { data[*index] = *value; }
        data
    }

    #[test]
    fn slots_fill_a_column_of_seven_before_the_next_and_stop_at_four_columns() {
        assert_eq!((columns(1), columns(7), columns(8), columns(27), columns(100)), (1, 1, 2, 4, 4));
        // 27 slots: 4 columns, filled row by row of 4
        assert_eq!(slot_position(0, 27), Some([SIDE_X + 4.0, SIDE_Y]));
        assert_eq!(slot_position(5, 27), Some([SIDE_X + 4.0 + 18.0, SIDE_Y + 18.0]));
        assert_eq!(slot_position(27, 27), None, "past the visible 28 of 27 slots there is none");
        assert_eq!(slot_position(3, 3), None, "a smaller structure hides the rest");
        assert_eq!(slot_position(27, 100), Some([SIDE_X + 4.0 + 18.0 * 3.0, SIDE_Y + 18.0 * 6.0]));
        assert_eq!(slot_position(28, 100), None);
    }

    #[test]
    fn fluids_stack_from_the_bottom_by_their_share_of_the_capacity_and_a_click_finds_the_one_under_it() {
        // two fluids of a 1000 capacity tank: 500 and 250
        let (high, low) = split(500);
        let (high2, low2) = split(250);
        let (cap_high, cap_low) = split(1000);
        let data = data_with(&[(D_CAPACITY, cap_high), (D_CAPACITY + 1, cap_low), (D_FLUID_COUNT, 2), (D_FLUIDS + 1, high), (D_FLUIDS + 2, low), (D_FLUIDS + 4, high2), (D_FLUIDS + 5, low2)]);
        let bands = tank_bands(&data);
        assert_eq!(bands, vec![(0, 69.0, 53.0), (1, 43.0, 26.0)], "106 * 500 / 1000 = 53 and 106 * 250 / 1000 = 26 from the bottom at 122");
        assert_eq!(fluid_clicked([20.0, 100.0], &data), Some(0));
        assert_eq!(fluid_clicked([20.0, 50.0], &data), Some(1));
        assert_eq!(fluid_clicked([20.0, 20.0], &data), None, "the empty part of the tank");
        assert_eq!(fluid_clicked([150.0, 100.0], &data), None);
        // a capacity beyond 32767 survives the two halves
        let (high, low) = split(1_000_000);
        assert_eq!(join(&data_with(&[(0, high), (1, low)]), 0), 1_000_000);
    }

    #[test]
    fn clicking_a_fluid_moves_it_to_the_bottom_of_the_saved_tank() {
        let mut tank = SmelteryTank::default();
        tank.set_capacity(1000);
        tank.fill(&id("a:iron"), 100, true);
        tank.fill(&id("a:gold"), 50, true);
        let mut data = NbtCompound::new();
        data.insert("tank".to_owned(), Nbt::Compound(tank.write()));
        press(1, &mut data);
        press(5, &mut data);
        let mut read = SmelteryTank::default();
        read.read(data.get("tank").and_then(Nbt::as_compound).unwrap());
        assert_eq!(read.fluids().iter().map(|fluid| fluid.fluid.to_string()).collect::<Vec<_>>(), ["a:gold", "a:iron"]);
    }

    #[test]
    fn the_window_draws_stacked_fluids_the_gauge_and_one_heat_bar_per_occupied_slot() {
        let mut canvas = Recorder::default();
        let (cap_high, cap_low) = split(1000);
        let (high, low) = split(500);
        let data = data_with(&[(0, 200), (1, 400), (2, 800), (D_SLOTS, 3), (D_CAPACITY, cap_high), (D_CAPACITY + 1, cap_low), (D_FLUID_COUNT, 1), (D_FLUIDS + 1, high), (D_FLUIDS + 2, low),
            (D_HEAT, 300), (D_HEAT + 1, 600), (D_HEAT + 2, 800), (D_HEAT + 3, 1)]);
        draw_window(&mut canvas, [10.0, 20.0], &data);
        assert!(canvas.fills.contains(&([18.0, 89.0, 106.0, 53.0], 0xFFFF_FFFF)), "half of the tank, white for a fluid no mod defined: {:?}", canvas.fills);
        let flame = canvas.sprites.iter().find(|s| s.1[0] == 176.0 && s.1[2] == 14.0).expect("flame");
        assert_eq!(flame.1[3], 7.0);
        let bars: Vec<_> = canvas.sprites.iter().filter(|s| s.1[0] >= 176.0 && s.1[1] >= 150.0).collect();
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].1, [176.0, 158.0, 3.0, 8.0], "halfway through 600");
    }

    #[test]
    fn a_world_without_a_scope_gives_the_structure_nothing_and_a_state_is_classified_by_its_tags() {
        let mut world = MemoryWorldOps::default();
        world.tags.push(("tconstruct:seared_bricks".to_owned(), "tconstruct:smeltery/wall".to_owned()));
        world.tags.push(("tconstruct:seared_bricks".to_owned(), "tconstruct:smeltery".to_owned()));
        world.tags.push(("minecraft:light".to_owned(), "tconstruct:structure_air".to_owned()));
        let bricks = BlockState::parse("tconstruct:seared_bricks[in_structure=true]").unwrap();
        let light = BlockState::parse("minecraft:light").unwrap();
        with_world(&mut world, || {
            let cell = classify(Kind::Smeltery, &bricks);
            assert_eq!((cell.any, cell.wall, cell.floor, cell.tank, cell.air, cell.in_structure), (true, true, false, false, false, Some(true)));
            assert!(classify(Kind::Smeltery, &light).air, "structure air counts as air");
            assert!(!classify(Kind::Foundry, &bricks).wall, "another structure's tags");
        });
        assert!(World::ticking().is_none());
        let data = Data::default();
        let live = Live::new(&data, None, [0, 64, 0]);
        assert_eq!(live.cell(Kind::Smeltery, [0, 0, 0]), None);
    }

    #[test]
    fn both_controllers_register_a_window_and_a_ticking_block_entity_with_the_slots_the_menu_holds() {
        register();
        for name in ["tconstruct:smeltery_controller", "tconstruct:foundry_controller"] {
            assert!(BlockEntityTypeRegistry::get(name).is_some());
            let window = ScreenHandlerRegistry::get(name).expect("window");
            assert_eq!(window.layout.slots.len(), MAX_SLOTS);
            assert_eq!(window.layout.slots.iter().flatten().count(), VISIBLE);
            assert_eq!(window.layout.inventory_y, 138.0);
        }
    }

    #[test]
    fn joining_a_structure_tells_a_drain_its_master_and_a_drain_of_another_live_controller_is_foreign() {
        use pluto_lib::with_ticking_dimension;
        let dim = "minecraft:overworld".to_owned();
        let mut world = MemoryWorldOps::default();
        world.blocks.insert((dim.clone(), [0, 64, 0]), "tconstruct:smeltery_controller[in_structure=true]".to_owned());
        world.blocks.insert((dim.clone(), [9, 64, 0]), "tconstruct:smeltery_controller[in_structure=true]".to_owned());
        world.blocks.insert((dim.clone(), [1, 64, 0]), "tconstruct:seared_drain[in_structure=false]".to_owned());
        world.entities.insert((dim.clone(), [1, 64, 0]), NbtCompound::new());
        let data = Data::default();
        with_ticking_dimension("minecraft:overworld", || with_world(&mut world, || {
            let mut live = Live::new(&data, None, [0, 64, 0]);
            live.mark([1, 64, 0], true);
            assert_eq!(World::ticking().unwrap().block_entity_data([1, 64, 0]).unwrap().get("master"), Some(&Nbt::IntArray(vec![0, 64, 0])));
            assert_eq!(World::ticking().unwrap().get_block_state([1, 64, 0]).unwrap().property("in_structure"), Some("true"));
            assert!(!live.cell(Kind::Smeltery, [1, 64, 0]).unwrap().foreign, "our own servant");
            let other = Live::new(&data, None, [9, 64, 0]);
            assert!(other.cell(Kind::Smeltery, [1, 64, 0]).unwrap().foreign, "it answers to the controller at 0 that is still formed");
            live.mark([1, 64, 0], false);
            assert!(!other.cell(Kind::Smeltery, [1, 64, 0]).unwrap().foreign, "freed");
            assert_eq!(World::ticking().unwrap().block_entity_data([1, 64, 0]).unwrap().get("master"), Some(&Nbt::IntArray(vec![])));
        }));
    }
}
