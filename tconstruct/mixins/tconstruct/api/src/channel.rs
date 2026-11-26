//! The channel, from SlimeKnights ChannelBlock, ChannelBlockEntity, ChannelTank and ChannelSideTank (MIT; see LICENSE): a small
//! tank (four nuggets) that takes fluid from above or from a side set to `in`, and every tick passes it down (when `down` is set)
//! or, split evenly, out of the sides set to `out`. The block states, their placement and shape updates are here as pure logic;
//! toggling a side by right click needs a block use hook with the hit position and a block state write (`interact_with_side` is
//! the logic), and "a fluid handler" is a block whose block entity type exposes one to PlutoLib's registry
//! (`BlockEntityTypeRegistry::is_fluid_handler`), the port's form of the original's capability check.
use crate::faucet::MB_PER_TICK;
use pluto_lib::{
    BlockBehavior, BlockBehaviors, BlockEntityClientTick, BlockEntityDrain, BlockEntityFill, BlockEntityTick, BlockEntityType, BlockEntityTypeRegistry, BlockPos, BlockState, BlockView, Direction,
    FluidStack, Identifier, Nbt, NbtCompound, NbtCompoundExt, Neighbours, Placement,
};

pub(crate) const BLOCKS: [&str; 2] = ["seared_channel", "scorched_channel"];
pub(crate) const CAPACITY: i32 = MB_PER_TICK * 4;
/// Plane.HORIZONTAL.
const SIDES: [Direction; 4] = [Direction::North, Direction::East, Direction::South, Direction::West];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Connection { None, In, Out }

impl Connection {
    pub(crate) fn parse(text: Option<&str>) -> Connection { match text { Some("in") => Connection::In, Some("out") => Connection::Out, _ => Connection::None } }
    pub(crate) fn name(self) -> &'static str { match self { Connection::None => "none", Connection::In => "in", Connection::Out => "out" } }
    pub(crate) fn can_flow(self) -> bool { self != Connection::None }
    pub(crate) fn opposite(self) -> Connection { match self { Connection::In => Connection::Out, Connection::Out => Connection::In, Connection::None => Connection::None } }
    pub(crate) fn next(self, reverse: bool) -> Connection {
        match (self, reverse) {
            (Connection::None, false) => Connection::In,
            (Connection::In, false) => Connection::Out,
            (Connection::Out, false) => Connection::None,
            (Connection::None, true) => Connection::Out,
            (Connection::Out, true) => Connection::In,
            (Connection::In, true) => Connection::None,
        }
    }
}

/// The connection properties of a channel state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Shape { pub down: bool, pub sides: [Connection; 4] }

impl Shape {
    pub(crate) fn of(state: &BlockState) -> Shape {
        Shape { down: state.property("down") == Some("true"), sides: SIDES.map(|side| Connection::parse(state.property(side.name()))) }
    }
    fn side(&self, side: Direction) -> Connection { SIDES.iter().position(|own| *own == side).map_or(Connection::None, |index| self.sides[index]) }
    fn is_output(&self, side: Direction) -> bool {
        match side { Direction::Up => false, Direction::Down => self.down, side => self.side(side) == Connection::Out }
    }
    fn outputs(&self) -> i32 { self.sides.iter().filter(|side| **side == Connection::Out).count() as i32 }
}

fn is_channel(state: &BlockState) -> bool { state.block.namespace() == "tconstruct" && BLOCKS.contains(&state.block.path()) }
fn is_handler(state: &BlockState) -> bool { BlockEntityTypeRegistry::is_fluid_handler(&state.block.to_string()) }
fn is_air(state: &BlockState) -> bool { state.block.namespace() == "minecraft" && state.block.path().ends_with("air") }

fn can_connect(view: &dyn BlockView, pos: BlockPos, side: Direction) -> bool {
    view.block_state(side.relative(pos)).is_some_and(|state| is_channel(&state) || is_handler(&state))
}

fn with_side(state: BlockState, side: Direction, connection: Connection) -> BlockState { state.with(side.name(), connection.name()) }

/// ChannelBlock.getStateForPlacement.
pub(crate) fn placement(state: BlockState, pos: BlockPos, face: Direction, sneaking: bool, view: &dyn BlockView) -> BlockState {
    let state = state.with("powered", if view.has_neighbor_signal(pos) == Some(true) { "true" } else { "false" });
    match face {
        Direction::Down => state,
        Direction::Up => state.with("down", if can_connect(view, pos, Direction::Down) { "true" } else { "false" }),
        side => {
            let placed_on = side.opposite().relative(pos);
            let connection = match view.block_state(placed_on) {
                Some(on) if is_channel(&on) => if sneaking { Connection::In } else { Connection::Out },
                Some(on) if is_handler(&on) => Connection::Out,
                _ => Connection::None,
            };
            with_side(state, side.opposite(), connection)
        }
    }
}

/// ChannelBlock.updateShape: `neighbour` is the changed neighbour on `direction`.
pub(crate) fn shape_update(state: &BlockState, direction: Direction, neighbour: Option<&BlockState>) -> Option<BlockState> {
    let mut state = state.clone();
    match direction {
        Direction::Down => {
            if state.property("down") == Some("true") && neighbour.is_some_and(is_air) { state = state.with("down", "false"); }
        }
        Direction::Up => {}
        side => match neighbour {
            // a neighbouring channel mirrors the connection, inverted
            Some(other) if is_channel(other) => {
                let theirs = Connection::parse(other.property(side.opposite().name()));
                state = with_side(state, side, theirs.opposite());
            }
            // out is only valid facing a fluid handler
            other => {
                if Connection::parse(state.property(side.name())) != Connection::None && other.is_some_and(is_air) { state = with_side(state, side, Connection::None); }
            }
        },
    }
    Some(state)
}

/// ChannelBlock.interactWithSide: the state after clicking `side` (None when nothing changes). `can_connect` is whether that side
/// faces a channel or a fluid handler.
pub(crate) fn interact_with_side(state: &BlockState, side: Direction, facing_is_channel: bool, can_connect: bool, sneaking: bool) -> Option<BlockState> {
    if side == Direction::Down {
        let down = state.property("down") == Some("true");
        if !down && can_connect { return Some(state.clone().with("down", "true")); }
        if down { return Some(state.clone().with("down", "false")); }
        return None;
    }
    let mut next = Connection::parse(state.property(side.name())).next(sneaking);
    // out needs something to pour into
    if next == Connection::Out && !facing_is_channel && !can_connect { next = next.next(sneaking); }
    Some(with_side(state.clone(), side, next))
}

/// The channel block entity.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct State { pub fluid: Option<FluidStack>, pub locked: i32, pub flowing: [u8; 5] }

/// The neighbours a channel pours into.
pub(crate) trait Out {
    fn has(&mut self, side: Direction) -> bool;
    fn fill(&mut self, side: Direction, stack: &FluidStack, execute: bool) -> i32;
}

fn flow_index(side: Direction) -> usize {
    match side { Direction::Down | Direction::Up => 0, Direction::North => 1, Direction::South => 2, Direction::West => 3, Direction::East => 4 }
}

impl State {
    fn max_usable(&self) -> i32 { (self.fluid.as_ref().map_or(0, |fluid| fluid.amount as i32) - self.locked).max(0) }

    fn set_flow(&mut self, side: Direction, flowing: bool) { if side != Direction::Up { self.flowing[flow_index(side)] = if flowing { 2 } else { 0 }; } }

    pub(crate) fn is_flowing(&self, side: Direction) -> bool { side != Direction::Up && self.flowing[flow_index(side)] > 0 }

    /// ChannelTank.fill through the top, or a side set to `in` (face None counts as the top); the amount is locked for this tick so the
    /// renderer never sees an empty channel that is being drained as it fills.
    pub(crate) fn fill(&mut self, shape: &Shape, face: Option<Direction>, resource: &FluidStack) -> u32 {
        match face {
            None | Some(Direction::Up) => {}
            Some(Direction::Down) => return 0,
            Some(side) => {
                if shape.side(side) != Connection::In { return 0; }
            }
        }
        let taken = match &mut self.fluid {
            None => resource.amount.min(CAPACITY as u32),
            Some(held) if !held.same_fluid(&resource) => return 0,
            Some(held) => resource.amount.min((CAPACITY as u32).saturating_sub(held.amount)),
        };
        if taken == 0 { return 0; }
        match &mut self.fluid {
            Some(held) => held.amount += taken,
            None => self.fluid = Some(FluidStack { fluid: resource.fluid.clone(), amount: taken, tag: resource.tag.clone() }),
        }
        self.locked += taken as i32;
        if let Some(side) = face.filter(|side| SIDES.contains(side)) { self.set_flow(side, true); }
        taken
    }

    fn try_side(&mut self, side: Direction, rate: i32, shape: &Shape, out: &mut dyn Out) -> bool {
        if self.fluid.is_none() || !shape.is_output(side) || !out.has(side) { return false; }
        let usable = self.max_usable().min(rate);
        if usable > 0 {
            if let Some(held) = self.fluid.clone() {
                let offer = FluidStack { fluid: held.fluid.clone(), amount: usable as u32, tag: held.tag.clone() };
                let filled = out.fill(side, &offer, false);
                if filled > 0 {
                    let moved = FluidStack { fluid: held.fluid, amount: filled as u32, tag: held.tag };
                    out.fill(side, &moved, true);
                    let left = held.amount as i32 - filled;
                    self.fluid = (left > 0).then(|| FluidStack { fluid: moved.fluid.clone(), amount: left as u32, tag: moved.tag.clone() });
                    self.set_flow(side, true);
                    return true;
                }
            }
        }
        self.set_flow(side, false);
        false
    }

    /// ChannelBlockEntity.tick.
    pub(crate) fn tick(&mut self, shape: &Shape, out: &mut dyn Out) {
        if self.fluid.is_some() {
            let flown = shape.down && self.try_side(Direction::Down, MB_PER_TICK, shape, out);
            let outputs = shape.outputs();
            if !flown && outputs > 0 {
                let rate = (self.max_usable() / outputs).clamp(1, MB_PER_TICK);
                for side in SIDES { self.try_side(side, rate, shape, out); }
            }
        }
        for flow in &mut self.flowing { *flow = flow.saturating_sub(1); }
        self.locked = 0;
    }

    pub(crate) fn load(data: &NbtCompound, fluid: Option<FluidStack>) -> Self {
        let mut flowing = [0u8; 5];
        if let Some(Nbt::IntArray(saved)) = data.get("is_flowing") {
            for (slot, value) in flowing.iter_mut().zip(saved) { *slot = (*value).clamp(0, 2) as u8; }
        }
        State { fluid, locked: data.get_int("locked", 0) as i32, flowing }
    }

    pub(crate) fn save(&self, data: &mut NbtCompound) {
        data.insert("locked".to_owned(), Nbt::Int(self.locked));
        data.insert("is_flowing".to_owned(), Nbt::IntArray(self.flowing.iter().map(|flow| i32::from(*flow)).collect()));
    }
}

struct Beside(BlockPos);
impl Out for Beside {
    fn has(&mut self, side: Direction) -> bool { let (pos, face) = Neighbours::beside(self.0, side); Neighbours::fluid_tanks(pos, face).is_some() }
    fn fill(&mut self, side: Direction, stack: &FluidStack, execute: bool) -> i32 {
        let (pos, face) = Neighbours::beside(self.0, side);
        Neighbours::fill_fluid(pos, face, stack, execute) as i32
    }
}

fn shape_at(pos: BlockPos) -> Shape { Neighbours::block_state(pos).map_or(Shape { down: false, sides: [Connection::None; 4] }, |state| Shape::of(&state)) }

fn tick_channel(tick: &mut BlockEntityTick<'_>) {
    let mut state = State::load(tick.data, tick.fluid(0));
    state.tick(&shape_at(tick.pos), &mut Beside(tick.pos));
    state.save(tick.data);
    tick.set_fluid(0, state.fluid.clone());
}

/// The client tick of a channel: while the down side flows, the stream below it is measured from the client
/// world and kept in the transient client data (`stream`: the block states below, nearest first), so the
/// renderer can draw it (Mantle's `RenderingHelper.renderFaucetFluids` reads the same states live); when the
/// down side does not flow the stream is cleared, so nothing stale is drawn. The faucet's client tick
/// (`tick_faucet_client`) is the same shape.
pub(crate) fn tick_channel_client(tick: &mut BlockEntityClientTick<'_>) {
    let state = State::load(tick.data, None);
    if !state.is_flowing(Direction::Down) {
        tick.client_data.remove("stream");
        return;
    }
    let states = crate::faucet::stream_states(tick.world, tick.pos);
    let stream = Nbt::List(states.into_iter().map(Nbt::String).collect());
    tick.client_data.insert("stream".into(), stream);
}

fn fill_channel(fill: &mut BlockEntityFill<'_>) -> u32 {
    let mut state = State::load(fill.data, fill.fluid_in(0));
    let taken = state.fill(&shape_at(fill.pos), fill.face, &fill.fluid.clone());
    state.save(fill.data);
    fill.set_fluid(0, state.fluid.clone());
    taken
}

/// The channel's handler in the original is fill-only (`FillOnlyFluidHandler` on top, insert-only
/// side tanks, an empty handler facing out): fluid leaves a channel only through the channel's own
/// flow, never through a neighbour's drain — so the drain rule gives nothing, stated outright rather
/// than left to the declared tank's default drain.
fn drain_channel(_drain: &mut BlockEntityDrain<'_>) -> Option<FluidStack> { None }

pub(crate) fn register() {
    for name in BLOCKS {
        let block = Identifier::new("tconstruct", name).expect("valid channel id");
        BlockEntityTypeRegistry::register(
            BlockEntityType::new(block.clone()).fluid_tanks([CAPACITY as u32]).sync_data().ticker(tick_channel).client_ticker(tick_channel_client).fluid_filler(fill_channel).fluid_drainer(drain_channel),
        );
        BlockBehaviors::register(BlockBehavior::new(block)
            .state_for_placement(|c| Placement::State(placement(c.state.clone(), c.pos, c.face, c.sneaking, c.view)))
            .update_shape(|c| shape_update(&c.state, c.direction, c.neighbour.as_ref())));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::MemoryBlockView;

    fn fluid(amount: u32) -> FluidStack { FluidStack { fluid: Identifier::parse("t:molten_iron").unwrap(), amount, tag: None } }
    fn channel() -> BlockState { BlockState::parse("tconstruct:seared_channel").unwrap() }

    /// Neighbours with room for `room` each, by side.
    struct Sinks { room: [i32; 6], got: [i32; 6] }
    fn slot(side: Direction) -> usize { Direction::ALL.iter().position(|d| *d == side).unwrap() }
    impl Out for Sinks {
        fn has(&mut self, side: Direction) -> bool { self.room[slot(side)] > 0 }
        fn fill(&mut self, side: Direction, stack: &FluidStack, execute: bool) -> i32 {
            let taken = (stack.amount as i32).min(self.room[slot(side)]);
            if execute { self.room[slot(side)] -= taken; self.got[slot(side)] += taken; }
            taken
        }
    }

    #[test]
    fn the_top_takes_a_nugget_a_tick_up_to_four_and_a_side_only_when_it_is_set_to_in() {
        let shape = Shape::of(&channel().with("east", "in").with("west", "out"));
        let mut state = State::default();
        assert_eq!(state.fill(&shape, Some(Direction::Up), &fluid(30)), 30, "the tank holds 40");
        assert_eq!(state.fill(&shape, None, &fluid(30)), 10);
        assert_eq!(state.fill(&shape, Some(Direction::East), &fluid(30)), 0, "full");
        let mut other = State::default();
        assert_eq!(other.fill(&shape, Some(Direction::West), &fluid(10)), 0, "an out side takes nothing");
        assert_eq!(other.fill(&shape, Some(Direction::North), &fluid(10)), 0, "no connection");
        assert_eq!(other.fill(&shape, Some(Direction::East), &fluid(10)), 10);
        assert!(other.is_flowing(Direction::East), "the side that filled is marked flowing");
        assert_eq!(other.fill(&shape, Some(Direction::Up), &FluidStack { fluid: Identifier::parse("t:water").unwrap(), amount: 5, tag: None }), 0, "one fluid at a time");
    }

    #[test]
    fn fluid_that_arrived_this_tick_is_not_passed_on_and_down_wins_over_the_sides() {
        let shape = Shape::of(&channel().with("down", "true").with("north", "out").with("south", "out"));
        let mut state = State::default();
        state.fill(&shape, Some(Direction::Up), &fluid(40));
        let mut sinks = Sinks { room: [1000; 6], got: [0; 6] };
        state.tick(&shape, &mut sinks);
        assert_eq!(sinks.got, [0; 6], "the fresh 40 is locked for the tick that filled it");
        state.tick(&shape, &mut sinks);
        assert_eq!((sinks.got[slot(Direction::Down)], sinks.got[slot(Direction::North)]), (10, 0), "a nugget down, and nothing to the sides while down flows");
        sinks.room[slot(Direction::Down)] = 0;
        state.tick(&shape, &mut sinks);
        assert_eq!((sinks.got[slot(Direction::North)], sinks.got[slot(Direction::South)]), (10, 10), "down is blocked: 30 split over two sides, 10 each (the cap)");
        assert_eq!(state.fluid.as_ref().unwrap().amount, 10);
    }

    #[test]
    fn two_outputs_share_what_is_left_and_the_flow_marks_fade_after_two_ticks() {
        let shape = Shape::of(&channel().with("east", "out").with("west", "out"));
        let mut state = State { fluid: Some(fluid(8)), ..State::default() };
        let mut sinks = Sinks { room: [1000; 6], got: [0; 6] };
        state.tick(&shape, &mut sinks);
        assert_eq!((sinks.got[slot(Direction::East)], sinks.got[slot(Direction::West)]), (4, 4));
        assert!(state.is_flowing(Direction::East));
        state.tick(&shape, &mut sinks);
        state.tick(&shape, &mut sinks);
        assert!(!state.is_flowing(Direction::East) && state.fluid.is_none());
        let mut saved = NbtCompound::new();
        let flowing = State { flowing: [0, 2, 1, 0, 2], locked: 7, ..State::default() };
        flowing.save(&mut saved);
        assert_eq!(State::load(&saved, None), flowing);
    }

    #[test]
    fn connections_cycle_mirror_across_channels_and_drop_when_the_neighbour_goes() {
        let c = Connection::None;
        assert_eq!((c.next(false), c.next(false).next(false), c.next(false).next(false).next(false)), (Connection::In, Connection::Out, Connection::None));
        assert_eq!((c.next(true), Connection::In.next(true)), (Connection::Out, Connection::None));
        let state = channel().with("east", "in");
        let other = channel().with("west", "out");
        assert_eq!(shape_update(&state, Direction::East, Some(&other)).unwrap().property("east"), Some("in"), "the neighbour's out is our in");
        let theirs = channel().with("west", "in");
        assert_eq!(shape_update(&state, Direction::East, Some(&theirs)).unwrap().property("east"), Some("out"));
        let air = BlockState::parse("minecraft:air").unwrap();
        assert_eq!(shape_update(&state, Direction::East, Some(&air)).unwrap().property("east"), Some("none"));
        assert_eq!(shape_update(&channel().with("down", "true"), Direction::Down, Some(&air)).unwrap().property("down"), Some("false"));
        // clicking: out is skipped where nothing could take the fluid
        let clicked = interact_with_side(&channel().with("east", "in"), Direction::East, false, false, false).unwrap();
        assert_eq!(clicked.property("east"), Some("none"), "in -> out needs a handler, so it moves on");
        assert_eq!(interact_with_side(&channel(), Direction::Down, false, false, false), None);
        assert_eq!(interact_with_side(&channel(), Direction::Down, false, true, false).unwrap().property("down"), Some("true"));
    }

    #[test]
    fn placing_against_a_channel_or_a_tank_connects_to_it() {
        // the melter below counts as a tank: its block entity type exposes a fluid handler
        BlockEntityTypeRegistry::register(BlockEntityType::new(Identifier::parse("tconstruct:seared_melter").unwrap()).fluid_tanks([1000]));
        let mut view = MemoryBlockView::default();
        view.blocks.insert([1, 64, 0], channel());
        view.blocks.insert([0, 63, 0], BlockState::parse("tconstruct:seared_melter").unwrap());
        // the channel at x=1 is clicked on its west face, so the new block at x=0 has it on its east side
        let placed = placement(channel(), [0, 64, 0], Direction::West, false, &view);
        assert_eq!(placed.property("east"), Some("out"), "against a channel: out, or in when sneaking");
        assert_eq!(placement(channel(), [0, 64, 0], Direction::West, true, &view).property("east"), Some("in"));
        assert_eq!(placement(channel(), [0, 64, 0], Direction::Up, false, &view).property("down"), Some("true"), "on top, over a tank");
        assert_eq!(placement(channel(), [5, 64, 0], Direction::Up, false, &view).property("down"), Some("false"));
        // a seared tank is a fluid handler too: placing against it connects out toward it (the in-game failure of #1161)
        BlockEntityTypeRegistry::register(BlockEntityType::new(Identifier::parse("tconstruct:seared_ingot_tank").unwrap()).fluid_tanks([4000]));
        view.blocks.insert([5, 64, -1], BlockState::parse("tconstruct:seared_ingot_tank").unwrap());
        assert_eq!(placement(channel(), [5, 64, 0], Direction::South, false, &view).property("north"), Some("out"), "against a tank: out");
    }

    #[test]
    fn the_client_tick_keeps_the_stream_below_while_the_down_side_flows_and_clears_it_after() {
        let mut view = MemoryBlockView::default();
        view.blocks.insert([0, 63, 0], BlockState::parse("tconstruct:seared_table").unwrap());
        // the down side is flowing (flow slot 0): the stream names the block below
        let mut data = NbtCompound::new();
        State { flowing: [2, 0, 0, 0, 0], ..State::default() }.save(&mut data);
        let mut client = NbtCompound::new();
        tick_channel_client(&mut BlockEntityClientTick { pos: [0, 64, 0], data: &data, client_data: &mut client, world: &view });
        assert_eq!(client.get("stream"), Some(&Nbt::List(vec![Nbt::String("tconstruct:seared_table".into())])));
        // nothing flows down anymore: the kept stream is cleared, so the renderer draws none
        let mut data = NbtCompound::new();
        State::default().save(&mut data);
        tick_channel_client(&mut BlockEntityClientTick { pos: [0, 64, 0], data: &data, client_data: &mut client, world: &view });
        assert_eq!(client.get("stream"), None);
    }
}
