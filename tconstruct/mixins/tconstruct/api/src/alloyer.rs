//! The scorched alloyer's block entity logic, from SlimeKnights AlloyerBlockEntity, MixerAlloyTank,
//! SingleAlloyingModule and SolidFuelModule (MIT; see LICENSE): a tank of its own that alloy recipes fill from
//! the tanks beside it (every side but down whose block is in `alloyer_tanks`), fuel found in the block below,
//! and the 4-tick cycle (tick 0 finds fuel, tick 2 alloys and consumes it). `heating.rs` holds the fuel state
//! machine and `melting_recipes.rs` the alloy recipes; this ties them to the neighbour tanks and the block below.
//! The neighbour cache the original keeps (refreshed by `neighborChanged`) is not kept: the sides are read fresh
//! every tick, which is the same set. The comparator output is not ported, as with the melter.
use crate::{
    fluid_ingredient::FluidTags,
    fluid_values::INGOT,
    heating::Fuel,
    melter::{fluid_order, FuelSource, LiquidFuel, SolidFuel},
    melting_recipes::{Alloy, AlloyTank},
};
use pluto_lib::{BlockEntityTick, BlockPos, Direction, FluidStack, Identifier, ItemSlot, Nbt, NbtCompound, NbtCompoundExt};

/// The alloyer's own tank: `TankType.INGOT_TANK`, 48 ingots (AlloyerBlockEntity's TANK_CAPACITY).
pub(crate) const ALLOYER_CAPACITY: i32 = INGOT * 48;

/// The output tank (FluidTankAnimated over the ingot tank's capacity): one fluid, filled as far as it fits.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Tank { pub fluid: Option<FluidStack>, pub capacity: i32 }

impl Tank {
    pub(crate) fn read(tick: &BlockEntityTick<'_>) -> Self { Tank { fluid: tick.fluid(0), capacity: ALLOYER_CAPACITY } }
    pub(crate) fn write(&self, tick: &mut BlockEntityTick<'_>) { tick.set_fluid(0, self.fluid.clone()); }

    /// `FluidTank.fill`: as much as fits; a different fluid is refused while there is one. `execute` false only asks.
    pub(crate) fn fill(&mut self, fluid: &Identifier, amount: i32, execute: bool) -> i32 {
        if amount <= 0 { return 0; }
        match &mut self.fluid {
            None => {
                let filled = self.capacity.min(amount).max(0);
                if execute && filled > 0 { self.fluid = Some(FluidStack { fluid: fluid.clone(), amount: filled as u32, tag: None }); }
                filled
            }
            Some(held) if &held.fluid != fluid => 0,
            Some(held) => {
                let filled = (self.capacity - held.amount as i32).min(amount).max(0);
                if execute { held.amount += filled as u32; }
                filled
            }
        }
    }
}

/// One input beside the alloyer (MixerAlloyTank's indexed tanks): where it is, the face it is reached through,
/// what its first tank holds, and how much that tank can hold.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Side { pub pos: BlockPos, pub face: Direction, pub fluid: Option<(Identifier, i32)>, pub capacity: i32 }

/// Everything around the alloyer that the cycle asks about.
pub(crate) trait Env {
    fn liquid_fuel(&self, fluid: &Identifier) -> Option<LiquidFuel>;
    fn solid_fuel(&self) -> Option<SolidFuel>;
    /// The vanilla burn time of an item in ticks, 0 for none (ForgeHooks.getBurnTime).
    fn burn_ticks(&self, item: &Identifier) -> i32;
    /// The item a burned fuel item leaves behind (a lava bucket's bucket).
    fn remainder(&self, item: &Identifier) -> Option<Identifier>;
    /// None where nothing holds fuel.
    fn source(&mut self) -> Option<&mut dyn FuelSource>;
    fn alloys(&self) -> &[Alloy];
    fn fluid_tags(&self) -> &FluidTags;
    /// The input tanks on every side but down, in the original's order (up, north, south, west, east): a side
    /// counts when its block is an alloyer tank and answers with a fluid handler, even an empty one.
    fn sides(&mut self) -> Vec<Side>;
    /// Drains up to `amount` from the first tank of the block at `pos` through `face`; what came out.
    fn drain_side(&mut self, pos: BlockPos, face: Direction, amount: i32) -> i32;
    /// The alloyer's own block state: (`in_structure`, `active`).
    fn block_state(&self) -> (bool, bool);
    /// `ControllerBlock.ACTIVE` on the alloyer and on the fuel block below it, as the tick sets them.
    fn set_active(&mut self, active: bool);
}

/// `MixerAlloyTank`: the alloy recipes' view — the side tanks are the inputs, the alloyer's own tank the output.
pub(crate) struct Mixer<'a> {
    sides: Vec<Side>,
    output: &'a mut Tank,
    temperature: i32,
    env: &'a mut dyn Env,
}

impl AlloyTank for Mixer<'_> {
    fn tanks(&self) -> usize { self.sides.len() }
    fn fluid_in_tank(&self, tank: usize) -> Option<(Identifier, i32)> { self.sides.get(tank).and_then(|side| side.fluid.clone()) }
    fn temperature(&self) -> i32 { self.temperature }
    /// MixerAlloyTank.canFit: the whole output must fit in the alloyer's own tank; the room the inputs free is in
    /// other blocks' tanks, so `removed` buys nothing here (unlike the smeltery's shared tank).
    fn can_fit(&self, output: &(Identifier, i32), _removed: i32) -> bool {
        let mut probe = self.output.clone();
        probe.fill(&output.0, output.1, false) == output.1
    }
    fn drain(&mut self, tank: usize, _fluid: &Identifier, amount: i32) -> i32 {
        let Some(side) = self.sides.get(tank) else { return 0 };
        let (pos, face) = (side.pos, side.face);
        let drained = self.env.drain_side(pos, face, amount);
        if drained > 0 {
            if let Some((_, held)) = &mut self.sides[tank].fluid {
                *held -= drained;
                if *held <= 0 { self.sides[tank].fluid = None; }
            }
        }
        drained
    }
    fn fill(&mut self, fluid: &Identifier, amount: i32) -> i32 { self.output.fill(fluid, amount, true) }
}

/// `SingleAlloyingModule`: one recipe a cycle, the last one tried first while it still performs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Alloying { last: Option<usize> }

impl Alloying {
    /// findRecipe: the remembered recipe while it performs, else the first that fits and performs; a found one
    /// is remembered. (The original's extra `canFit(output, 0)` filter is the same test for this tank, whose
    /// canFit ignores what is removed.)
    fn find(&mut self, alloys: &[Alloy], mixer: &mut Mixer<'_>, tags: &FluidTags) -> Option<usize> {
        if let Some(last) = self.last {
            if alloys.get(last).is_some_and(|alloy| alloy.can_perform(&*mixer, tags)) { return Some(last); }
        }
        let found = alloys.iter().position(|alloy| alloy.can_perform(&*mixer, tags));
        if found.is_some() { self.last = found; }
        found
    }

    /// canAlloy: whether a recipe could run now (temperature, amounts and room).
    pub(crate) fn can_alloy(&mut self, alloys: &[Alloy], mixer: &mut Mixer<'_>, tags: &FluidTags) -> bool { self.find(alloys, mixer, tags).is_some() }

    /// doAlloy: runs the found recipe once.
    pub(crate) fn do_alloy(&mut self, alloys: &[Alloy], mixer: &mut Mixer<'_>, tags: &FluidTags) {
        if let Some(index) = self.find(alloys, mixer, tags) { alloys[index].perform(mixer, tags); }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct State {
    pub fuel: Fuel,
    pub tick: i32,
    alloying: Alloying,
}

/// The block entity data key of the remembered recipe: the original keeps `lastRecipe` in the live block entity
/// only, a ticker here is stateless between ticks (the smeltery saves its own runtime state the same way).
const LAST_ALLOY: &str = "pluto_last_alloy";

impl State {
    /// Reads the saved data; the remembered recipe is named by id, as the recipe list is rebuilt on every load.
    pub(crate) fn load(data: &NbtCompound, alloys: &[Alloy]) -> Self {
        let mut state = State::default();
        state.fuel = Fuel {
            fuel: data.get_int("fuel", 0) as i32,
            quality: data.get_int("quality", 0) as i32,
            temperature: data.get_int("temperature", 0) as i32,
            rate: data.get_int("rate", 0) as i32,
        };
        state.tick = (data.get_int("tick", 0) as i32).rem_euclid(4);
        state.alloying.last = data.get_string(LAST_ALLOY).and_then(|text| Identifier::parse(text).ok())
            .and_then(|id| alloys.iter().position(|alloy| alloy.id == id));
        state
    }

    pub(crate) fn save(&self, data: &mut NbtCompound, alloys: &[Alloy]) {
        for (key, value) in [("fuel", self.fuel.fuel), ("quality", self.fuel.quality), ("temperature", self.fuel.temperature), ("rate", self.fuel.rate), ("tick", self.tick)] {
            data.insert(key.to_owned(), Nbt::Int(value));
        }
        match self.alloying.last.and_then(|index| alloys.get(index)) {
            Some(alloy) => { data.insert(LAST_ALLOY.to_owned(), Nbt::String(alloy.id.to_string())); }
            None => { data.remove(LAST_ALLOY); }
        }
    }

    /// FuelModule.findFuel(consume): liquid fuel first (it usually goes hotter), then solid. Returns the temperature found, 0 for none.
    pub(crate) fn find_fuel(&mut self, consume: bool, env: &mut dyn Env) -> i32 {
        let mut temperature = 0;
        if let Some(stack) = env.source().and_then(|source| source.fluid()) {
            if let Some(recipe) = env.liquid_fuel(&stack.fluid) {
                let available = stack.amount.min(i32::MAX as u32) as i32;
                let burn = consume && available >= recipe.amount;
                temperature = self.fuel.try_liquid(available, recipe.amount, recipe.duration, recipe.temperature, recipe.rate, burn);
                if burn && temperature > 0 {
                    if let Some(source) = env.source() { source.drain(recipe.amount); }
                }
            }
        }
        if temperature == 0 { temperature = self.try_solid(consume, env); }
        if temperature == 0 && consume { self.fuel.none_found(); }
        temperature
    }

    /// SolidFuelModule.trySolidFuel: the first slot of the source holding an item that burns.
    fn try_solid(&mut self, consume: bool, env: &mut dyn Env) -> i32 {
        let Some(solid) = env.solid_fuel() else { return 0 };
        let candidates = env.source().map(|source| source.items()).unwrap_or_default();
        for (slot, stack) in candidates {
            let burn = env.burn_ticks(&stack.item);
            if burn / 4 <= 0 { continue; }
            if !consume { return self.fuel.try_solid(burn, solid.temperature, solid.rate, false); }
            let taken = env.source().and_then(|source| source.take(slot));
            if taken.as_ref().is_some_and(|taken| taken.item == stack.item) {
                self.fuel.try_solid(burn, solid.temperature, solid.rate, true);
                // return the container (a lava bucket's bucket); what the source cannot take back is lost, the host cannot drop it from here
                if let Some(container) = env.remainder(&stack.item) {
                    if let Some(source) = env.source() { source.give_back(ItemSlot::new(container, 1)); }
                }
            } else {
                eprintln!("Tinkers: invalid item removed from the alloyer's fuel source");
            }
            return solid.temperature;
        }
        0
    }

    fn can_alloy(&mut self, tank: &mut Tank, env: &mut dyn Env, temperature: i32) -> bool {
        let alloys = env.alloys().to_vec();
        let tags = env.fluid_tags().clone();
        let mut mixer = Mixer { sides: env.sides(), output: tank, temperature, env };
        self.alloying.can_alloy(&alloys, &mut mixer, &tags)
    }

    fn do_alloy(&mut self, tank: &mut Tank, env: &mut dyn Env, temperature: i32) {
        let alloys = env.alloys().to_vec();
        let tags = env.fluid_tags().clone();
        let mut mixer = Mixer { sides: env.sides(), output: tank, temperature, env };
        self.alloying.do_alloy(&alloys, &mut mixer, &tags);
    }

    /// AlloyerBlockEntity.tick, one game tick.
    pub(crate) fn step(&mut self, tank: &mut Tank, env: &mut dyn Env) {
        if env.block_state().0 {
            match self.tick {
                // tick 0: find fuel; burn some when an alloy can run and none is burning
                0 => {
                    let temperature = self.find_fuel(false, env);
                    if !self.fuel.has_fuel() && self.can_alloy(tank, env, temperature) { self.find_fuel(true, env); }
                }
                // tick 2: alloy alloys and consume fuel
                2 => {
                    let has_fuel = self.fuel.has_fuel();
                    if env.block_state().1 != has_fuel { env.set_active(has_fuel); }
                    if has_fuel {
                        let temperature = self.fuel.temperature;
                        self.do_alloy(tank, env, temperature);
                        self.fuel.decrease(1);
                    }
                }
                _ => {}
            }
        // if no tank below, drain the excess leftover fuel from the last operation
        } else if self.tick == 2 && self.fuel.has_fuel() {
            self.fuel.decrease(1);
        }
        self.tick = (self.tick + 1) % 4;
    }
}

/// The synchronized menu values: 0 fuel, 1 fuel quality, 2 temperature, 3 rate, then the output tank's amount and
/// which defined fluid it holds (one more than its position in `fluid_order`, 0 for none), the number of input
/// tanks, then three per input tank (amount, fluid, capacity). The original syncs only the fuel module and reads
/// the tanks from the block entities; here the ticker publishes them, as the melter publishes its tank.
pub(crate) const DATA_OUTPUT_AMOUNT: usize = 4;
pub(crate) const DATA_OUTPUT_FLUID: usize = DATA_OUTPUT_AMOUNT + 1;
pub(crate) const DATA_INPUTS: usize = DATA_OUTPUT_FLUID + 1;
pub(crate) const fn input_data(index: usize) -> usize { DATA_INPUTS + 1 + 3 * index }
/// AlloyerScreen supports five input tanks at most.
pub(crate) const MAX_INPUTS: usize = 5;
/// The fuel gauge's own values, published by the ticker from the block below: which fluid the
/// fuel tank below holds, its amount and capacity (each a high/low pair).
pub(crate) const DATA_FUEL_FLUID: usize = input_data(MAX_INPUTS);
pub(crate) const DATA_FUEL_AMOUNT: usize = DATA_FUEL_FLUID + 1;
pub(crate) const DATA_FUEL_CAPACITY: usize = DATA_FUEL_AMOUNT + 2;
pub(crate) const DATA_SLOTS: usize = DATA_FUEL_CAPACITY + 2;

fn fluid_index(fluid: &Identifier) -> i32 {
    fluid_order().iter().position(|own| own == fluid).map_or(0, |position| position as i32 + 1)
}

impl State {
    /// Publishes the window's values; the gauge's come from the block below (the fuel tank's
    /// fluid, amount and capacity), which the state does not hold.
    pub(crate) fn publish(&self, tick: &mut BlockEntityTick<'_>, tank: &Tank, sides: &[Side], gauge: Option<&(Identifier, i32, i32)>) {
        let values = [self.fuel.fuel, self.fuel.quality, self.fuel.temperature, self.fuel.rate];
        for (index, value) in values.into_iter().enumerate() { tick.set_menu_data(index, value); }
        tick.set_menu_data(DATA_OUTPUT_AMOUNT, tank.fluid.as_ref().map_or(0, |fluid| fluid.amount as i32));
        tick.set_menu_data(DATA_OUTPUT_FLUID, tank.fluid.as_ref().map_or(0, |fluid| fluid_index(&fluid.fluid)));
        tick.set_menu_data(DATA_INPUTS, sides.len().min(MAX_INPUTS) as i32);
        for index in 0..MAX_INPUTS {
            let base = input_data(index);
            let side = sides.get(index);
            tick.set_menu_data(base, side.and_then(|side| side.fluid.as_ref()).map_or(0, |(_, amount)| *amount));
            tick.set_menu_data(base + 1, side.and_then(|side| side.fluid.as_ref()).map_or(0, |(fluid, _)| fluid_index(fluid)));
            tick.set_menu_data(base + 2, side.map_or(0, |side| side.capacity));
        }
        tick.set_menu_data(DATA_FUEL_FLUID, gauge.and_then(|(fluid, _, _)| fluid_order().iter().position(|id| id == fluid)).map_or(0, |position| position as i32 + 1));
        for (index, value) in [(DATA_FUEL_AMOUNT, gauge.map_or(0, |(_, amount, _)| *amount)), (DATA_FUEL_CAPACITY, gauge.map_or(0, |(_, _, capacity)| *capacity))] {
            tick.set_menu_data(index, value >> 15);
            tick.set_menu_data(index + 1, value & 0x7FFF);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{melter, melting_recipes::Alloy as AlloyRecipe};
    use serde_json::{json, Value};

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }

    struct Source { fluid: Option<FluidStack>, items: Vec<Option<ItemSlot>> }
    impl FuelSource for Source {
        fn fluid(&mut self) -> Option<FluidStack> { self.fluid.clone() }
        fn drain(&mut self, amount: i32) -> i32 {
            let Some(held) = self.fluid.as_mut() else { return 0 };
            let taken = (amount as u32).min(held.amount);
            held.amount -= taken;
            if held.amount == 0 { self.fluid = None; }
            taken as i32
        }
        fn items(&mut self) -> Vec<(usize, ItemSlot)> { self.items.iter().enumerate().filter_map(|(slot, stack)| stack.clone().map(|stack| (slot, stack))).collect() }
        fn take(&mut self, slot: usize) -> Option<ItemSlot> {
            let held = self.items.get_mut(slot)?.as_mut()?;
            let mut one = held.clone();
            one.count = 1;
            held.count -= 1;
            if held.count == 0 { self.items[slot] = None; }
            Some(one)
        }
        fn give_back(&mut self, stack: ItemSlot) -> Option<ItemSlot> {
            match self.items.iter_mut().find(|slot| slot.is_none()) { Some(slot) => { *slot = Some(stack); None } None => Some(stack) }
        }
    }

    /// The world of the tests: a fuel source below, side tanks in index order (their position's x is the index),
    /// the alloy recipes, and the alloyer's own block state.
    struct World {
        source: Option<Source>,
        sides: Vec<Option<(Identifier, i32)>>,
        alloys: Vec<AlloyRecipe>,
        tags: FluidTags,
        formed: bool,
        active: bool,
        lit: Vec<bool>,
    }

    impl World {
        fn new(sides: Vec<Option<(Identifier, i32)>>) -> Self {
            World { source: None, sides, alloys: vec![bronze()], tags: FluidTags::default(), formed: true, active: false, lit: Vec::new() }
        }
        fn heated(mut self) -> Self {
            self.source = Some(Source { fluid: None, items: vec![Some(ItemSlot::new(id("minecraft:coal"), 1))] });
            self
        }
        fn side(&self, index: usize) -> Option<(Identifier, i32)> { self.sides[index].clone() }
    }

    impl Env for World {
        fn liquid_fuel(&self, fluid: &Identifier) -> Option<LiquidFuel> {
            (fluid.path() == "lava").then_some(LiquidFuel { amount: 50, duration: 100, temperature: 1000, rate: 10 })
        }
        fn solid_fuel(&self) -> Option<SolidFuel> { Some(SolidFuel { temperature: 800, rate: 8 }) }
        fn burn_ticks(&self, item: &Identifier) -> i32 { melter::burn_time(item) }
        fn remainder(&self, item: &Identifier) -> Option<Identifier> { melter::fuel_remainder(item) }
        fn source(&mut self) -> Option<&mut dyn FuelSource> { self.source.as_mut().map(|source| source as &mut dyn FuelSource) }
        fn alloys(&self) -> &[Alloy] { &self.alloys }
        fn fluid_tags(&self) -> &FluidTags { &self.tags }
        fn sides(&mut self) -> Vec<Side> {
            self.sides.iter().enumerate().map(|(index, fluid)| Side { pos: [index as i32, 64, 0], face: Direction::Down, fluid: fluid.clone(), capacity: 4320 }).collect()
        }
        fn drain_side(&mut self, pos: BlockPos, _face: Direction, amount: i32) -> i32 {
            let Some(held) = self.sides.get_mut(pos[0] as usize).and_then(Option::as_mut) else { return 0 };
            let taken = amount.min(held.1);
            held.1 -= taken;
            if held.1 <= 0 { self.sides[pos[0] as usize] = None; }
            taken
        }
        fn block_state(&self) -> (bool, bool) { (self.formed, self.active) }
        fn set_active(&mut self, active: bool) { self.active = active; self.lit.push(active); }
    }

    fn alloy(name: &str, temperature: i32, inputs: Value, result: Value) -> AlloyRecipe {
        let fields = json!({"type": "tconstruct:alloy", "temperature": temperature, "inputs": inputs, "result": result});
        AlloyRecipe::decode(&id(name), fields.as_object().unwrap()).unwrap()
    }
    /// The shipped bronze: 30 copper and 10 tin at 800 C make 40.
    fn bronze() -> AlloyRecipe {
        alloy("a:bronze", 800, json!([{"amount": 30, "fluid": "a:copper"}, {"amount": 10, "fluid": "a:tin"}]), json!({"amount": 40, "fluid": "a:bronze"}))
    }
    fn empty_tank() -> Tank { Tank { fluid: None, capacity: ALLOYER_CAPACITY } }
    fn run(state: &mut State, tank: &mut Tank, world: &mut World, ticks: i32) {
        for _ in 0..ticks { state.step(tank, world); }
    }

    #[test]
    fn the_cycle_burns_fuel_only_for_an_alloy_and_mixes_the_side_tanks_into_its_own() {
        let mut world = World::new(vec![Some((id("a:copper"), 100)), Some((id("a:tin"), 100))]).heated();
        let (mut state, mut tank) = (State::default(), empty_tank());
        // tick 0: an alloy can run at the solid fuel's 800 C, so the coal burns (a quarter of 1600)
        run(&mut state, &mut tank, &mut world, 1);
        assert_eq!((state.fuel.fuel, state.fuel.temperature, state.fuel.rate), (400, 800, 8));
        assert!(world.source.as_ref().unwrap().items[0].is_none(), "the coal was taken from the block below");
        assert_eq!(tank.fluid, None, "nothing alloys before the third tick");
        // tick 2: the sides are drained by the recipe's amounts, the output lands in the alloyer's tank, one fuel is spent
        run(&mut state, &mut tank, &mut world, 2);
        assert_eq!(tank.fluid, Some(FluidStack { fluid: id("a:bronze"), amount: 40, tag: None }));
        assert_eq!((world.side(0), world.side(1)), (Some((id("a:copper"), 70)), Some((id("a:tin"), 90))));
        assert_eq!(state.fuel.fuel, 399);
        assert_eq!(world.lit, vec![true], "the block lit when the fuel state was first published");
        // the next cycle alloys again without burning more: the fuel lasts
        run(&mut state, &mut tank, &mut world, 4);
        assert_eq!(tank.fluid, Some(FluidStack { fluid: id("a:bronze"), amount: 80, tag: None }));
        assert_eq!(state.fuel.fuel, 398);
    }

    #[test]
    fn without_every_input_or_the_heat_nothing_burns_and_lava_burns_before_coal() {
        // tin is missing: tick 0 finds no recipe, the coal stays
        let mut world = World::new(vec![Some((id("a:copper"), 100))]).heated();
        let (mut state, mut tank) = (State::default(), empty_tank());
        run(&mut state, &mut tank, &mut world, 4);
        assert_eq!((state.fuel.fuel, tank.fluid.as_ref()), (0, None));
        assert_eq!(world.source.as_ref().unwrap().items[0].as_ref().unwrap().count, 1);
        // an empty side still counts as a tank but never matches: the bronze sits behind it in the order
        let mut world = World::new(vec![None, Some((id("a:copper"), 100)), Some((id("a:tin"), 100))]).heated();
        let (mut state, mut tank) = (State::default(), empty_tank());
        run(&mut state, &mut tank, &mut world, 3);
        assert_eq!(tank.fluid, Some(FluidStack { fluid: id("a:bronze"), amount: 40, tag: None }));
        // a recipe wanting 1000 C cannot run on coal (800): lava below is tried first and is hot enough
        let mut world = World::new(vec![Some((id("a:copper"), 100)), Some((id("a:tin"), 100))]).heated();
        world.alloys = vec![alloy("a:hot_bronze", 1000, json!([{"amount": 30, "fluid": "a:copper"}, {"amount": 10, "fluid": "a:tin"}]), json!({"amount": 40, "fluid": "a:bronze"}))];
        let (mut state, mut tank) = (State::default(), empty_tank());
        run(&mut state, &mut tank, &mut world, 4);
        assert_eq!((state.fuel.fuel, tank.fluid.as_ref()), (0, None), "too cold on solid fuel: nothing burned");
        world.source.as_mut().unwrap().fluid = Some(FluidStack { fluid: id("minecraft:lava"), amount: 100, tag: None });
        run(&mut state, &mut tank, &mut world, 3);
        assert_eq!((state.fuel.temperature, state.fuel.rate), (1000, 10), "lava is tried first");
        assert_eq!(world.source.as_ref().unwrap().fluid.as_ref().unwrap().amount, 50, "one burn of lava was drained");
        assert_eq!(tank.fluid, Some(FluidStack { fluid: id("a:bronze"), amount: 40, tag: None }));
    }

    #[test]
    fn the_output_must_fit_whole_it_shares_no_room_with_the_inputs() {
        // 4300 of bronze in the tank: the next 40 do not fit, though the inputs would free 40 beside it
        let mut world = World::new(vec![Some((id("a:copper"), 100)), Some((id("a:tin"), 100))]).heated();
        let (mut state, mut tank) = (State::default(), empty_tank());
        tank.fluid = Some(FluidStack { fluid: id("a:bronze"), amount: ALLOYER_CAPACITY as u32 - 20, tag: None });
        run(&mut state, &mut tank, &mut world, 1);
        assert_eq!(state.fuel.fuel, 0, "no alloy can run, so no fuel burns");
        // with fuel already burning, the alloy pass does nothing but the fuel is still spent
        state.fuel.fuel = 10;
        state.fuel.temperature = 800;
        run(&mut state, &mut tank, &mut world, 2);
        assert_eq!((state.fuel.fuel, tank.fluid.as_ref().unwrap().amount), (9, (ALLOYER_CAPACITY - 20) as u32));
        assert_eq!((world.side(0), world.side(1)), (Some((id("a:copper"), 100)), Some((id("a:tin"), 100))), "nothing was drained");
        // a different fluid in the output refuses the recipe the same way
        tank.fluid = Some(FluidStack { fluid: id("a:iron"), amount: 10, tag: None });
        state.fuel.fuel = 10;
        run(&mut state, &mut tank, &mut world, 4);
        assert_eq!(tank.fluid, Some(FluidStack { fluid: id("a:iron"), amount: 10, tag: None }));
    }

    #[test]
    fn unformed_the_alloyer_only_drains_its_leftover_fuel() {
        let mut world = World::new(vec![Some((id("a:copper"), 100)), Some((id("a:tin"), 100))]).heated();
        world.formed = false;
        let (mut state, mut tank) = (State::default(), empty_tank());
        state.fuel.fuel = 3;
        state.fuel.temperature = 800;
        run(&mut state, &mut tank, &mut world, 8);
        assert_eq!(state.fuel.fuel, 1, "one leftover fuel drains per cycle, on the third tick");
        assert_eq!(tank.fluid, None);
        assert!(world.lit.is_empty(), "the active state is the formed tick's work");
        assert_eq!(world.source.as_ref().unwrap().items[0].as_ref().unwrap().count, 1, "no new fuel is found either");
    }

    #[test]
    fn the_remembered_recipe_runs_first_and_a_catalyst_is_not_used_up() {
        let rose = alloy("a:rose", 800, json!([{"amount": 10, "fluid": "a:copper"}, {"amount": 10, "fluid": "a:gold"}]), json!({"amount": 20, "fluid": "a:rose"}));
        let mut world = World::new(vec![Some((id("a:copper"), 100)), Some((id("a:tin"), 100))]).heated();
        world.alloys = vec![rose, bronze()];
        let (mut state, mut tank) = (State::default(), empty_tank());
        // only bronze can run (no gold): it runs and is remembered
        run(&mut state, &mut tank, &mut world, 3);
        assert_eq!(tank.fluid, Some(FluidStack { fluid: id("a:bronze"), amount: 40, tag: None }));
        // gold arrives: the remembered bronze still runs first, though rose is listed first
        world.sides.push(Some((id("a:gold"), 100)));
        run(&mut state, &mut tank, &mut world, 4);
        assert_eq!(tank.fluid, Some(FluidStack { fluid: id("a:bronze"), amount: 80, tag: None }));
        // the tin runs out: the remembered recipe no longer performs, the search finds rose
        world.sides[1] = None;
        tank.fluid = None;
        run(&mut state, &mut tank, &mut world, 4);
        assert_eq!(tank.fluid, Some(FluidStack { fluid: id("a:rose"), amount: 20, tag: None }));
        // a catalyst must be present but is not drained
        let mut world = World::new(vec![Some((id("a:ember"), 50)), Some((id("a:water"), 50))]).heated();
        world.alloys = vec![alloy("a:steam", 800, json!([{"fluid": "a:ember", "amount": 10, "catalyst": true}, {"amount": 10, "fluid": "a:water"}]), json!({"amount": 10, "fluid": "a:steam"}))];
        let (mut state, mut tank) = (State::default(), empty_tank());
        run(&mut state, &mut tank, &mut world, 3);
        assert_eq!(tank.fluid, Some(FluidStack { fluid: id("a:steam"), amount: 10, tag: None }));
        assert_eq!((world.side(0), world.side(1)), (Some((id("a:ember"), 50)), Some((id("a:water"), 40))));
    }

    #[test]
    fn one_side_cannot_feed_two_inputs_and_the_tank_fills_only_as_far_as_it_fits() {
        // two copper inputs: the first side feeds the first, the second side the second
        let mut world = World::new(vec![Some((id("a:copper"), 100)), Some((id("a:copper"), 100))]).heated();
        world.alloys = vec![alloy("a:double", 800, json!([{"amount": 30, "fluid": "a:copper"}, {"amount": 20, "fluid": "a:copper"}]), json!({"amount": 50, "fluid": "a:bronze"}))];
        let (mut state, mut tank) = (State::default(), empty_tank());
        run(&mut state, &mut tank, &mut world, 3);
        assert_eq!((world.side(0), world.side(1)), (Some((id("a:copper"), 70)), Some((id("a:copper"), 80))));
        // one side alone cannot feed both
        let mut world = World::new(vec![Some((id("a:copper"), 100))]).heated();
        world.alloys = vec![alloy("a:double", 800, json!([{"amount": 30, "fluid": "a:copper"}, {"amount": 20, "fluid": "a:copper"}]), json!({"amount": 50, "fluid": "a:bronze"}))];
        let (mut state, mut tank) = (State::default(), empty_tank());
        run(&mut state, &mut tank, &mut world, 4);
        assert_eq!(tank.fluid, None);
        // the tank itself: a fill is partial, a look changes nothing, another fluid is refused
        let mut tank = empty_tank();
        assert_eq!(tank.fill(&id("a:bronze"), ALLOYER_CAPACITY + 100, false), ALLOYER_CAPACITY);
        assert_eq!(tank.fluid, None, "a simulated fill changes nothing");
        assert_eq!(tank.fill(&id("a:bronze"), ALLOYER_CAPACITY + 100, true), ALLOYER_CAPACITY);
        assert_eq!(tank.fill(&id("a:iron"), 10, true), 0);
        assert_eq!(tank.fill(&id("a:bronze"), 10, true), 0, "full");
    }

    #[test]
    fn state_survives_a_save_and_the_menu_publishes_what_the_screen_draws() {
        let mut world = World::new(vec![Some((id("a:copper"), 100)), Some((id("a:tin"), 100))]).heated();
        let (mut state, mut tank) = (State::default(), empty_tank());
        run(&mut state, &mut tank, &mut world, 3);
        let mut data = NbtCompound::new();
        let mut items: Vec<Option<ItemSlot>> = Vec::new();
        let mut tick = BlockEntityTick { pos: [0, 64, 0], data: &mut data, items: &mut items };
        state.save(tick.data, &world.alloys);
        tank.write(&mut tick);
        let sides = world.sides();
        state.publish(&mut tick, &tank, &sides, None);
        let loaded = State::load(tick.data, &world.alloys);
        assert_eq!(loaded, state);
        assert_eq!(Tank::read(&tick), tank);
        let menu = match tick.data.get("pluto:menu_data") { Some(Nbt::IntArray(values)) => values.clone(), _ => panic!("no menu data") };
        assert_eq!(menu.len(), DATA_SLOTS);
        assert_eq!((menu[0], menu[2], menu[3]), (state.fuel.fuel, 800, 8));
        assert_eq!(menu[DATA_OUTPUT_AMOUNT], 40);
        assert_eq!(menu[DATA_INPUTS], 2);
        assert_eq!((menu[input_data(0)], menu[input_data(0) + 2]), (70, ALLOYER_CAPACITY));
        assert_eq!(menu[input_data(1)], 90);
    }
}
