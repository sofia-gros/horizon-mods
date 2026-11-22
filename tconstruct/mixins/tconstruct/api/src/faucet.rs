//! The faucet, from SlimeKnights FaucetBlock and FaucetBlockEntity (MIT; see LICENSE): it draws an ingot's worth from the tank
//! behind it (the side opposite its facing) and pours it into the block below at a nugget per tick, stopping when the source runs
//! dry, the target is full or, if it was not powered, when asked to. Right click toggles it. Redstone pulses and the scheduled tick
//! need block events the host does not deliver to a block entity yet (`State::handle_redstone` is the logic); the client packet is
//! the saved `render_fluid`/`state` data, which a renderer will read.
use crate::fluid_values::{INGOT, NUGGET};
use pluto_lib::{
    ActionResult, BlockEntityTick, BlockEntityType, BlockEntityTypeRegistry, BlockEntityUse, BlockPos, Direction, FluidStack, Identifier, Nbt, NbtCompound,
    NbtCompoundExt, Neighbours,
};

/// Amount drawn from the input at a time, and what flows per tick.
pub(crate) const PACKET_SIZE: i32 = INGOT;
pub(crate) const MB_PER_TICK: i32 = NUGGET;
pub(crate) const BLOCKS: [&str; 2] = ["seared_faucet", "scorched_faucet"];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Mode { #[default] Off, Pouring, Powered }

/// What the faucet sees of the blocks it is between.
pub(crate) trait Plumbing {
    fn has_input(&mut self) -> bool;
    fn has_output(&mut self) -> bool;
    /// `drain(amount, SIMULATE)` of the input.
    fn peek_input(&mut self, amount: i32) -> Option<FluidStack>;
    fn drain_input(&mut self, amount: i32) -> Option<FluidStack>;
    /// `fill` of the output; `execute` false only asks.
    fn fill_output(&mut self, stack: &FluidStack, execute: bool) -> i32;
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct State {
    pub mode: Mode,
    pub stop: bool,
    pub drained: Option<FluidStack>,
    pub render: Option<FluidStack>,
    pub last_redstone: bool,
}

fn sized(stack: &FluidStack, amount: i32) -> FluidStack { FluidStack { fluid: stack.fluid.clone(), amount: amount.max(0) as u32, tag: stack.tag.clone() } }

impl State {
    pub(crate) fn is_pouring(&self) -> bool { self.mode != Mode::Off }

    /// syncToClient: what the renderer draws.
    fn sync(&mut self, fluid: Option<FluidStack>, pouring: bool) {
        let _ = pouring;
        self.render = fluid;
    }

    /// reset.
    fn reset(&mut self) {
        self.stop = false;
        self.drained = None;
        if self.mode != Mode::Off || self.render.is_some() {
            self.mode = Mode::Off;
            self.sync(None, false);
        }
    }

    /// activate: right click or redstone.
    pub(crate) fn activate(&mut self, plumbing: &mut dyn Plumbing) {
        match self.mode {
            Mode::Off => { self.stop = false; self.do_transfer(true, plumbing); }
            Mode::Powered => { self.mode = Mode::Off; self.sync(None, false); }
            Mode::Pouring => self.stop = true,
        }
    }

    /// handleRedstone: returns true when a scheduled tick (`activate` two ticks later) is wanted.
    pub(crate) fn handle_redstone(&mut self, powered: bool) -> bool {
        if powered == self.last_redstone { return false; }
        self.last_redstone = powered;
        if powered { return true; }
        if self.mode == Mode::Powered { self.mode = Mode::Off; self.sync(None, false); }
        false
    }

    /// tick: one game tick.
    pub(crate) fn tick(&mut self, plumbing: &mut dyn Plumbing) {
        if self.mode == Mode::Off { return; }
        if self.mode == Mode::Powered && self.do_transfer(false, plumbing) {
            self.mode = Mode::Pouring;
            return;
        }
        if self.drained.is_some() {
            self.pour(plumbing);
        } else if self.stop {
            self.reset();
        } else {
            self.do_transfer(true, plumbing);
        }
    }

    /// doTransfer: starts (or, with `execute` false, only checks) moving an ingot's worth.
    fn do_transfer(&mut self, execute: bool, plumbing: &mut dyn Plumbing) -> bool {
        if plumbing.has_input() && plumbing.has_output() {
            if let Some(peek) = plumbing.peek_input(PACKET_SIZE) {
                let filled = plumbing.fill_output(&peek, false);
                if filled > 0 {
                    // the output must also take the smallest step (copper cans do not)
                    let step = sized(&peek, MB_PER_TICK);
                    if filled <= MB_PER_TICK || plumbing.fill_output(&step, false) > 0 {
                        if execute {
                            self.drained = plumbing.drain_input(filled);
                            if self.mode == Mode::Off || self.render.as_ref().is_none_or(|render| render.fluid != step.fluid) {
                                let drained = self.drained.clone();
                                self.sync(drained, true);
                            }
                            self.mode = Mode::Pouring;
                            self.pour(plumbing);
                        }
                        return true;
                    }
                }
            }
            // powered: keep the faucet running
            if self.last_redstone {
                if execute && (self.mode == Mode::Off || self.render.is_some()) { self.sync(None, true); }
                self.mode = Mode::Powered;
                return false;
            }
        }
        if execute { self.reset(); }
        false
    }

    /// pour: one step of the buffered liquid into the output.
    fn pour(&mut self, plumbing: &mut dyn Plumbing) {
        let Some(drained) = self.drained.clone() else { return };
        if !plumbing.has_output() {
            // the output got lost: the buffered liquid with it
            self.reset();
            return;
        }
        let offer = sized(&drained, (drained.amount as i32).min(MB_PER_TICK));
        let filled = plumbing.fill_output(&offer, false);
        if filled > 0 {
            if self.render.as_ref().is_none_or(|render| render.fluid != drained.fluid) { self.sync(Some(drained.clone()), true); }
            let left = drained.amount as i32 - filled;
            self.drained = (left > 0).then(|| sized(&drained, left));
            plumbing.fill_output(&sized(&drained, filled), true);
        }
    }

    pub(crate) fn load(data: &NbtCompound) -> Self {
        let stack = |key: &str| {
            let tag = data.get(key)?.as_compound()?;
            Some(FluidStack { fluid: Identifier::parse(tag.get("Fluid")?.as_str()?).ok()?, amount: tag.get("Amount")?.as_i64()?.max(0) as u32, tag: tag.get("Tag").and_then(|tag| tag.as_compound().cloned()) }).filter(|stack| stack.amount > 0)
        };
        State {
            mode: match data.get_int("state", 0) { 1 => Mode::Pouring, 2 => Mode::Powered, _ => Mode::Off },
            stop: data.get_int("stop", 0) != 0,
            drained: stack("drained"),
            render: stack("render_fluid"),
            last_redstone: data.get_int("lastRedstone", 0) != 0,
        }
    }

    pub(crate) fn save(&self, data: &mut NbtCompound) {
        data.insert("state".to_owned(), Nbt::Int(self.mode as i32));
        data.insert("stop".to_owned(), Nbt::Int(i32::from(self.stop)));
        data.insert("lastRedstone".to_owned(), Nbt::Int(i32::from(self.last_redstone)));
        for (key, value) in [("drained", &self.drained), ("render_fluid", &self.render)] {
            match value {
                Some(stack) => {
                    let mut tag = NbtCompound::new();
                    tag.insert("Fluid".to_owned(), Nbt::String(stack.fluid.to_string()));
                    tag.insert("Amount".to_owned(), Nbt::Int(stack.amount as i32));
                    if let Some(fluid_tag) = &stack.tag {
                        tag.insert("Tag".to_owned(), Nbt::Compound(fluid_tag.clone()));
                    }
                    data.insert(key.to_owned(), Nbt::Compound(tag));
                }
                None => { data.remove(key); }
            }
        }
    }
}

/// The blocks around a faucet at `pos` facing `facing`: the tank behind it and the block below, through PlutoLib's neighbour reach.
pub(crate) struct Around { input: (BlockPos, Direction), output: (BlockPos, Direction) }

impl Around {
    pub(crate) fn of(pos: BlockPos, facing: Direction) -> Around {
        Around { input: Neighbours::beside(pos, facing.opposite()), output: Neighbours::beside(pos, Direction::Down) }
    }

    /// The faucet's facing from its own block state (`facing`, the hopper property: down or a side); north when unknown.
    pub(crate) fn facing_of(pos: BlockPos) -> Direction {
        Neighbours::block_state(pos).and_then(|state| state.property("facing").and_then(Direction::parse)).unwrap_or(Direction::North)
    }
}

impl Plumbing for Around {
    fn has_input(&mut self) -> bool { Neighbours::fluid_tanks(self.input.0, self.input.1).is_some() }
    fn has_output(&mut self) -> bool { Neighbours::fluid_tanks(self.output.0, self.output.1).is_some() }
    fn peek_input(&mut self, amount: i32) -> Option<FluidStack> {
        let fluid = Neighbours::fluid_tanks(self.input.0, self.input.1)?.into_iter().find_map(|tank| tank.fluid)?;
        Some(sized(&fluid, (fluid.amount as i32).min(amount))).filter(|stack| stack.amount > 0)
    }
    fn drain_input(&mut self, amount: i32) -> Option<FluidStack> { Neighbours::drain_fluid(self.input.0, self.input.1, amount.max(0) as u32) }
    fn fill_output(&mut self, stack: &FluidStack, execute: bool) -> i32 { Neighbours::fill_fluid(self.output.0, self.output.1, stack, execute) as i32 }
}

pub(crate) fn tick_faucet(tick: &mut BlockEntityTick<'_>) {
    let mut state = State::load(tick.data);
    if state.mode == Mode::Off { return; }
    let mut around = Around::of(tick.pos, Around::facing_of(tick.pos));
    state.tick(&mut around);
    state.save(tick.data);
}

pub(crate) fn use_faucet(used: &mut BlockEntityUse<'_>) -> ActionResult {
    if used.sneaking { return ActionResult::Pass; }
    let mut state = State::load(used.data);
    let mut around = Around::of(used.pos, Around::facing_of(used.pos));
    state.activate(&mut around);
    state.save(used.data);
    ActionResult::Success
}

pub(crate) fn register() {
    for name in BLOCKS {
        let block = Identifier::new("tconstruct", name).expect("valid faucet id");
        BlockEntityTypeRegistry::register(BlockEntityType::new(block).sync_data().ticker(tick_faucet).on_use(use_faucet));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fluid(amount: u32) -> FluidStack { FluidStack { fluid: Identifier::parse("t:molten_iron").unwrap(), amount, tag: None } }

    /// A source of `source` and a target taking up to `room`.
    struct Pipes { source: u32, room: u32, delivered: u32, output: bool }
    impl Plumbing for Pipes {
        fn has_input(&mut self) -> bool { true }
        fn has_output(&mut self) -> bool { self.output }
        fn peek_input(&mut self, amount: i32) -> Option<FluidStack> { Some(fluid(self.source.min(amount as u32))).filter(|stack| stack.amount > 0) }
        fn drain_input(&mut self, amount: i32) -> Option<FluidStack> {
            let taken = self.source.min(amount as u32);
            self.source -= taken;
            Some(fluid(taken)).filter(|stack| stack.amount > 0)
        }
        fn fill_output(&mut self, stack: &FluidStack, execute: bool) -> i32 {
            let taken = stack.amount.min(self.room);
            if execute { self.room -= taken; self.delivered += taken; }
            taken as i32
        }
    }

    #[test]
    fn a_click_draws_an_ingot_and_pours_a_nugget_a_tick_until_the_source_is_dry() {
        let mut pipes = Pipes { source: 200, room: 10_000, delivered: 0, output: true };
        let mut faucet = State::default();
        faucet.activate(&mut pipes);
        assert_eq!((faucet.mode, pipes.source, pipes.delivered), (Mode::Pouring, 110, 10), "90 drawn, the first nugget poured at once");
        for _ in 0..8 { faucet.tick(&mut pipes); }
        assert_eq!(pipes.delivered, 90);
        faucet.tick(&mut pipes);
        assert_eq!((pipes.source, pipes.delivered), (20, 100), "the buffer is empty, so the next ingot is drawn");
        for _ in 0..30 { faucet.tick(&mut pipes); }
        assert_eq!((faucet.mode, pipes.delivered, pipes.source), (Mode::Off, 200, 0), "dry: it stops by itself");
    }

    #[test]
    fn a_click_while_pouring_stops_after_the_buffer_and_a_full_target_stops_it() {
        let mut pipes = Pipes { source: 1000, room: 10_000, delivered: 0, output: true };
        let mut faucet = State::default();
        faucet.activate(&mut pipes);
        faucet.activate(&mut pipes);
        assert!(faucet.stop);
        for _ in 0..12 { faucet.tick(&mut pipes); }
        assert_eq!((faucet.mode, pipes.delivered), (Mode::Off, 90), "exactly the ingot that was already drawn");
        // it draws only what the target can take, so nothing is lost to a small target
        let mut small = Pipes { source: 1000, room: 25, delivered: 0, output: true };
        let mut faucet = State::default();
        faucet.activate(&mut small);
        for _ in 0..10 { faucet.tick(&mut small); }
        assert_eq!((small.delivered, small.source, faucet.mode), (25, 975, Mode::Off));
        let mut small = Pipes { source: 1000, room: 10_000, delivered: 0, output: true };
        let mut faucet = State::default();
        faucet.activate(&mut small);
        assert_eq!(faucet.mode, Mode::Pouring);
        small.output = false;
        faucet.tick(&mut small);
        assert_eq!((faucet.mode, faucet.drained.clone()), (Mode::Off, None), "a lost output loses the buffer");
    }

    #[test]
    fn redstone_keeps_it_powered_while_nothing_flows_and_a_pulse_asks_for_a_tick() {
        let mut pipes = Pipes { source: 0, room: 1000, delivered: 0, output: true };
        let mut faucet = State::default();
        assert!(faucet.handle_redstone(true), "a rising edge schedules activate");
        faucet.activate(&mut pipes);
        assert_eq!(faucet.mode, Mode::Powered);
        pipes.source = 90;
        faucet.tick(&mut pipes);
        assert_eq!(faucet.mode, Mode::Pouring, "a powered faucet starts as soon as there is fluid");
        assert!(!faucet.handle_redstone(false));
    }

    #[test]
    fn the_state_round_trips_through_saved_data() {
        let state = State { mode: Mode::Powered, stop: true, drained: Some(fluid(40)), render: Some(fluid(10)), last_redstone: true };
        let mut data = NbtCompound::new();
        state.save(&mut data);
        assert_eq!(State::load(&data), state);
        assert_eq!(State::load(&NbtCompound::new()), State::default());
    }

    #[test]
    fn the_faucet_reads_its_facing_and_finds_the_tank_behind_it_and_the_block_below() {
        use pluto_lib::{with_neighbours, MemoryNeighbourOps};
        let mut world = MemoryNeighbourOps::default();
        world.states.insert([0, 64, 0], "tconstruct:seared_faucet[facing=north]".to_owned());
        world.tanks.insert([0, 64, 1], vec![(Some(fluid(500)), 4000)]);
        world.tanks.insert([0, 63, 0], vec![(None, 90)]);
        with_neighbours(&mut world, || {
            let facing = Around::facing_of([0, 64, 0]);
            assert_eq!(facing, Direction::North);
            let mut faucet = State::default();
            faucet.activate(&mut Around::of([0, 64, 0], facing));
        });
        assert_eq!(world.tanks[&[0, 64, 1]][0].0.as_ref().unwrap().amount, 410, "an ingot drawn from the south");
        assert_eq!(world.tanks[&[0, 63, 0]][0].0.as_ref().unwrap().amount, 10);
    }
}
