//! Fluid pipes: a transport pipe with a declared tank that spreads what it holds to its
//! neighbours — levelling toward a neighbouring pipe that holds less of the same fluid, and
//! filling any other tank that takes it — and an extraction pipe that, while it has a redstone
//! signal, draws fluid out of the tank it faces into its own tank. Because the tank is a
//! declared one, the rest of the world (faucets, channels, other pipes, the smeltery's own
//! plumbing) fills and drains a fluid pipe through the ordinary neighbour reach with no
//! pipe-specific code.
use crate::common::{is_fluid_pipe, FLUID_EXTRACTION_PIPE, FLUID_PIPE, NAMESPACE};
use pluto_lib::{
    BlockEntitySignal, BlockEntityTick, BlockEntityType, BlockEntityTypeRegistry, BlockPos, Direction, FluidStack,
    Identifier, NeighbourTank, Neighbours, Nbt, NbtCompound, NbtCompoundExt,
};

/// What one fluid pipe holds, in millibuckets.
pub const CAPACITY: u32 = 1000;
/// The most fluid one pipe passes on in a tick, across all its exits.
pub const RATE: u32 = 100;
/// The most an extraction pipe draws from the tank it faces in a tick.
pub const EXTRACT_RATE: u32 = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Transport,
    Extraction,
}

/// What a fluid pipe remembers between ticks (its fluid lives in its declared tank).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FlowState {
    /// The last redstone strength the signal hook reported (extraction pipes only work powered).
    pub powered: bool,
    /// Round-robin cursor over the faces, so distribution does not favour one side forever.
    pub cursor: i32,
}

impl FlowState {
    pub fn load(data: &NbtCompound) -> Self {
        FlowState { powered: data.get_int("powered", 0) != 0, cursor: data.get_int("cursor", 0) as i32 }
    }

    pub fn save(&self, data: &mut NbtCompound) {
        data.insert("powered".to_owned(), Nbt::Int(i32::from(self.powered)));
        data.insert("cursor".to_owned(), Nbt::Int(self.cursor));
    }
}

/// The blocks around a fluid pipe, as the pipe's logic sees them.
pub trait FluidWorld {
    /// The block beside the pipe on `side`, if its state can be read.
    fn block_at(&mut self, side: Direction) -> Option<Identifier>;
    /// The tanks of the block beside the pipe on `side`; `None` where there are none.
    fn tanks_at(&mut self, side: Direction) -> Option<Vec<NeighbourTank>>;
    /// Puts fluid into the block beside the pipe on `side`; how much it took (asking only when
    /// `execute` is false).
    fn fill_at(&mut self, side: Direction, stack: &FluidStack, execute: bool) -> u32;
    /// Takes up to `amount` millibuckets out of the block beside the pipe on `side`.
    fn drain_at(&mut self, side: Direction, amount: u32) -> Option<FluidStack>;
}

fn sized(stack: &FluidStack, amount: u32) -> FluidStack {
    FluidStack { fluid: stack.fluid.clone(), amount, tag: stack.tag.clone() }
}

/// The extraction step: while powered, draw from the tank the pipe faces into its own tank,
/// never mixing fluids.
fn extract_step(state: &FlowState, kind: Kind, facing: Direction, held: Option<FluidStack>, world: &mut dyn FluidWorld) -> Option<FluidStack> {
    if kind != Kind::Extraction || !state.powered {
        return held;
    }
    let room = CAPACITY - held.as_ref().map_or(0, |stack| stack.amount);
    if room == 0 {
        return held;
    }
    let Some(tanks) = world.tanks_at(facing) else { return held };
    let Some(source) = tanks.iter().find_map(|tank| tank.fluid.clone()) else { return held };
    if held.as_ref().is_some_and(|own| !own.same_fluid(&source)) {
        return held;
    }
    let Some(drained) = world.drain_at(facing, room.min(EXTRACT_RATE)) else { return held };
    if held.as_ref().is_some_and(|own| !own.same_fluid(&drained)) {
        // The source gave a fluid other than the one its tanks listed first (a multi-tank
        // block): hand it straight back rather than mixing or voiding it.
        world.fill_at(facing, &drained, true);
        return held;
    }
    Some(match held {
        Some(mut own) => {
            own.amount += drained.amount;
            own
        }
        None => drained,
    })
}

/// One tick of a fluid pipe: the extraction step, then distribution of what the tank holds.
/// Returns the tank's new contents. Fluid is only ever moved, never created or voided: an
/// amount leaves this tank only when a neighbour's fill reports it taken.
pub fn tick(state: &mut FlowState, kind: Kind, facing: Direction, tank: Option<FluidStack>, world: &mut dyn FluidWorld) -> Option<FluidStack> {
    let mut held = extract_step(state, kind, facing, tank, world);
    let Some(mut current) = held.clone() else { return held };
    let mut budget = RATE;
    let start = state.cursor.rem_euclid(6) as usize;
    for step in 0..6 {
        if budget == 0 || current.amount == 0 {
            break;
        }
        let face = Direction::ALL[(start + step) % 6];
        if kind == Kind::Extraction && face == facing {
            continue;
        }
        let Some(block) = world.block_at(face) else { continue };
        if is_fluid_pipe(&block) {
            // Level toward a pipe that holds less of the same fluid (or none).
            let Some(tanks) = world.tanks_at(face) else { continue };
            let Some(their) = tanks.first() else { continue };
            if let Some(theirs) = &their.fluid {
                if !theirs.same_fluid(&current) || theirs.amount >= current.amount {
                    continue;
                }
            }
            let theirs = their.fluid.as_ref().map_or(0, |stack| stack.amount);
            let offer = ((current.amount - theirs + 1) / 2).min(budget).min(current.amount);
            if offer == 0 {
                continue;
            }
            let moved = world.fill_at(face, &sized(&current, offer), false).min(offer);
            if moved == 0 {
                continue;
            }
            world.fill_at(face, &sized(&current, moved), true);
            current.amount -= moved;
            budget -= moved;
            state.cursor = ((start + step + 1) % 6) as i32;
        } else {
            // Any other tank that takes the fluid is a sink: fill it.
            if world.tanks_at(face).is_none() {
                continue;
            }
            let offer = budget.min(current.amount);
            if offer == 0 {
                continue;
            }
            let moved = world.fill_at(face, &sized(&current, offer), false).min(offer);
            if moved == 0 {
                continue;
            }
            world.fill_at(face, &sized(&current, moved), true);
            current.amount -= moved;
            budget -= moved;
            state.cursor = ((start + step + 1) % 6) as i32;
        }
    }
    held = (current.amount > 0).then_some(current);
    held
}

/// The world around a pipe at `pos`, through PlutoLib's neighbour reach.
pub struct Around {
    pub pos: BlockPos,
}

impl FluidWorld for Around {
    fn block_at(&mut self, side: Direction) -> Option<Identifier> {
        Neighbours::block_state(side.relative(self.pos)).map(|state| state.block)
    }
    fn tanks_at(&mut self, side: Direction) -> Option<Vec<NeighbourTank>> {
        let (pos, face) = Neighbours::beside(self.pos, side);
        Neighbours::fluid_tanks(pos, face)
    }
    fn fill_at(&mut self, side: Direction, stack: &FluidStack, execute: bool) -> u32 {
        let (pos, face) = Neighbours::beside(self.pos, side);
        Neighbours::fill_fluid(pos, face, stack, execute)
    }
    fn drain_at(&mut self, side: Direction, amount: u32) -> Option<FluidStack> {
        let (pos, face) = Neighbours::beside(self.pos, side);
        Neighbours::drain_fluid(pos, face, amount)
    }
}

/// The direction an extraction pipe pulls from: its own `facing` property, north when unread.
fn facing_of(pos: BlockPos) -> Direction {
    Neighbours::block_state(pos)
        .and_then(|state| state.property("facing").and_then(Direction::parse))
        .unwrap_or(Direction::North)
}

pub(crate) fn tick_pipe(ctx: &mut BlockEntityTick<'_>, kind: Kind) {
    let mut state = FlowState::load(ctx.data);
    let mut world = Around { pos: ctx.pos };
    let held = tick(&mut state, kind, facing_of(ctx.pos), ctx.fluid(0), &mut world);
    ctx.set_fluid(0, held);
    state.save(ctx.data);
}

pub(crate) fn signal_pipe(signal: &mut BlockEntitySignal<'_>) {
    let mut state = FlowState::load(signal.data);
    state.powered = signal.powered;
    state.save(signal.data);
}

pub fn register() {
    for (name, kind) in [(FLUID_PIPE, Kind::Transport), (FLUID_EXTRACTION_PIPE, Kind::Extraction)] {
        let block = Identifier::new(NAMESPACE, name).expect("valid pipe id");
        let mut entity_type = BlockEntityType::new(block)
            .fluid_tanks([CAPACITY])
            .sync_data()
            .ticker(move |tick| tick_pipe(tick, kind));
        if kind == Kind::Extraction {
            entity_type = entity_type.on_signal(signal_pipe);
        }
        BlockEntityTypeRegistry::register(entity_type);
    }
}
