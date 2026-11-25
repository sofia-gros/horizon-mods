//! Item pipes: a transport pipe that carries stacks from side to side, and an extraction pipe
//! that — while it has a redstone signal — pulls one item every eight ticks out of the container
//! it faces and sends it into the run. Both are plain block entities with a ticker; everything
//! they touch goes through PlutoLib's neighbour reach, the same reach a hopper uses.
//!
//! How a stack crosses from one pipe to the next: a pipe declares six item slots and opens
//! exactly one of them to each face (slot = the face's index in `Direction::ALL`), for inserts
//! only, and its transfer rule refuses everything else. A pipe therefore receives a stack into
//! the slot of the face it arrived through, takes it into transit from there, and delivers by
//! inserting into the next pipe's matching slot or into the destination container. Nothing can
//! be taken out of a pipe from outside.
//!
//! Scenario note (casting table -> chest): the finished item of a casting table or basin is
//! reachable through this reach by design of the Tinkers port — the casting block entity type
//! opens its output slot for taking from every face, and its transfer rule allows the take
//! exactly while its tank is empty (a finished, cooled cast). An extraction pipe facing a
//! casting table therefore sees the product with `can_take` set as soon as the cast is done,
//! and not before. The pipe side only ever takes slots the neighbour lists as takeable, so if
//! a future host adapter reported `can_take` differently, the fallback is the same one a player
//! uses: leave the product in the table (extraction simply retries), or pull it with the
//! table's own redstone slot swap into a chest the pipe does extract from. No pipe-side change
//! would be needed either way; the in-game check is that the host's container reach reports
//! the casting output slot as takeable once the tank is empty.
use crate::common::{face_index, is_item_pipe, stack_from_nbt, stack_to_nbt, EXTRACTION_PIPE, ITEM_PIPE, NAMESPACE};
use pluto_lib::{
    BlockEntitySignal, BlockEntityTick, BlockEntityType, BlockEntityTypeRegistry, BlockPos, Direction, Identifier,
    ItemSlot, NeighbourSlot, Neighbours, Nbt, NbtCompound, NbtCompoundExt, SidedAccess, TransferMode,
};

/// Ticks a stack spends inside one pipe before it is delivered onward.
pub const TRAVEL_TICKS: i32 = 4;
/// Ticks between two delivery retries of a stack whose exit refused it.
pub const RETRY_TICKS: i32 = 4;
/// The most stacks one pipe carries at once; a full pipe stops taking more in.
pub const MAX_TRANSIT: usize = 8;
/// Ticks between two pulls of an extraction pipe (the classic eight).
pub const EXTRACT_PERIOD: i32 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Transport,
    Extraction,
}

/// One stack riding a pipe: where it came in, where it is headed, and how long it still has.
#[derive(Clone, Debug, PartialEq)]
pub struct Transit {
    pub stack: ItemSlot,
    pub from: Direction,
    pub to: Direction,
    pub ticks: i32,
    pub retries: i32,
}

/// What a pipe remembers between ticks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PipeState {
    pub transit: Vec<Transit>,
    /// Round-robin cursor over the faces, so junctions alternate their exits.
    pub cursor: i32,
    /// The last redstone strength the signal hook reported (extraction pipes only work powered).
    pub powered: bool,
    /// Ticks since the last pull (extraction pipes).
    pub clock: i32,
}

impl PipeState {
    pub fn load(data: &NbtCompound) -> Self {
        let mut transit = Vec::new();
        if let Some(Nbt::List(list)) = data.get("transit") {
            for entry in list {
                let Some(entry) = entry.as_compound() else { continue };
                let (Some(stack), Some(from), Some(to)) = (
                    stack_from_nbt(entry),
                    Direction::ALL.get(entry.get_int("From", -1) as usize).copied(),
                    Direction::ALL.get(entry.get_int("To", -1) as usize).copied(),
                ) else { continue };
                transit.push(Transit {
                    stack,
                    from,
                    to,
                    ticks: entry.get_int("Ticks", 0) as i32,
                    retries: entry.get_int("Retries", 0) as i32,
                });
            }
        }
        PipeState {
            transit,
            cursor: data.get_int("cursor", 0) as i32,
            powered: data.get_int("powered", 0) != 0,
            clock: data.get_int("clock", 0) as i32,
        }
    }

    pub fn save(&self, data: &mut NbtCompound) {
        let transit = self
            .transit
            .iter()
            .map(|transit| {
                let mut entry = stack_to_nbt(&transit.stack);
                entry.insert("From".to_owned(), Nbt::Int(face_index(transit.from) as i32));
                entry.insert("To".to_owned(), Nbt::Int(face_index(transit.to) as i32));
                entry.insert("Ticks".to_owned(), Nbt::Int(transit.ticks));
                entry.insert("Retries".to_owned(), Nbt::Int(transit.retries));
                Nbt::Compound(entry)
            })
            .collect();
        data.insert("transit".to_owned(), Nbt::List(transit));
        data.insert("cursor".to_owned(), Nbt::Int(self.cursor));
        data.insert("powered".to_owned(), Nbt::Int(i32::from(self.powered)));
        data.insert("clock".to_owned(), Nbt::Int(self.clock));
    }
}

/// The blocks around an item pipe, as the pipe's logic sees them.
pub trait ItemWorld {
    /// The block beside the pipe on `side`, if its state can be read.
    fn block_at(&mut self, side: Direction) -> Option<Identifier>;
    /// The slots of the container beside the pipe on `side`; `None` where there is none.
    fn slots_at(&mut self, side: Direction) -> Option<Vec<NeighbourSlot>>;
    /// Puts `stack` into the block beside the pipe on `side`; what did not fit comes back.
    fn insert_at(&mut self, side: Direction, stack: ItemSlot) -> Option<ItemSlot>;
    /// Takes up to `count` items out of `slot` of the container beside the pipe on `side`.
    fn extract_at(&mut self, side: Direction, slot: usize, count: u32) -> Option<ItemSlot>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    Pipe,
    Container,
    None,
}

fn target(world: &mut dyn ItemWorld, side: Direction) -> Target {
    if let Some(block) = world.block_at(side) {
        if is_item_pipe(&block) {
            return Target::Pipe;
        }
    }
    if world.slots_at(side).is_some() {
        Target::Container
    } else {
        Target::None
    }
}

/// Picks the next exit for a stack that entered through `from`: a face that is not `from` (and
/// not `avoid`) and reaches a pipe or a container, alternating by the round-robin cursor.
fn choose_exit(state: &mut PipeState, world: &mut dyn ItemWorld, from: Direction, avoid: Option<Direction>) -> Option<Direction> {
    let start = state.cursor.rem_euclid(6) as usize;
    for step in 0..6 {
        let face = Direction::ALL[(start + step) % 6];
        if face == from || Some(face) == avoid {
            continue;
        }
        if target(world, face) != Target::None {
            state.cursor = ((start + step + 1) % 6) as i32;
            return Some(face);
        }
    }
    None
}

enum Delivery {
    Done,
    /// Part of the stack moved; the rest stays and retries the same exit.
    Progress,
    Failed,
}

/// One delivery attempt of a transit that reached its exit.
fn deliver(world: &mut dyn ItemWorld, transit: &mut Transit) -> Delivery {
    if target(world, transit.to) == Target::None {
        return Delivery::Failed;
    }
    let before = transit.stack.count;
    match world.insert_at(transit.to, transit.stack.clone()) {
        None => Delivery::Done,
        Some(left) if left.count < before => {
            transit.stack = left;
            transit.ticks = RETRY_TICKS;
            Delivery::Progress
        }
        Some(_) => Delivery::Failed,
    }
}

/// The extraction step: while powered, one item every [`EXTRACT_PERIOD`] ticks out of the
/// container the pipe faces, taken only from a slot the neighbour lists as takeable.
fn extract_step(state: &mut PipeState, world: &mut dyn ItemWorld, facing: Direction) {
    if !state.powered {
        state.clock = 0;
        return;
    }
    state.clock += 1;
    if state.clock < EXTRACT_PERIOD {
        return;
    }
    state.clock = 0;
    if state.transit.len() >= MAX_TRANSIT {
        return;
    }
    let Some(to) = choose_exit(state, world, facing, None) else { return };
    let Some(slots) = world.slots_at(facing) else { return };
    for slot in slots {
        if !slot.can_take || slot.stack.is_none() {
            continue;
        }
        if let Some(taken) = world.extract_at(facing, slot.slot, 1) {
            state.transit.push(Transit { stack: taken, from: facing, to, ticks: TRAVEL_TICKS, retries: 0 });
            return;
        }
    }
}

/// One tick of an item pipe: extraction (extraction pipes), taking arrived stacks out of the
/// inbox slots, and moving every stack in transit toward its exit. A stack whose exit refuses
/// it is re-routed to another exit, bounced back the way it came when that is the only way, or
/// held for a retry — it is never dropped or voided.
pub fn tick(state: &mut PipeState, items: &mut Vec<Option<ItemSlot>>, world: &mut dyn ItemWorld, kind: Kind, facing: Direction) {
    if kind == Kind::Extraction {
        extract_step(state, world, facing);
    }
    for face in Direction::ALL {
        if state.transit.len() >= MAX_TRANSIT {
            break;
        }
        let Some(held) = items.get_mut(face_index(face)) else { continue };
        let Some(stack) = held.take() else { continue };
        let to = choose_exit(state, world, face, None).unwrap_or(face);
        state.transit.push(Transit { stack, from: face, to, ticks: TRAVEL_TICKS, retries: 0 });
    }
    let mut index = 0;
    while index < state.transit.len() {
        if state.transit[index].ticks > 0 {
            state.transit[index].ticks -= 1;
            index += 1;
            continue;
        }
        match deliver(world, &mut state.transit[index]) {
            Delivery::Done => {
                state.transit.remove(index);
            }
            Delivery::Progress => index += 1,
            Delivery::Failed => {
                let (from, to) = (state.transit[index].from, state.transit[index].to);
                state.transit[index].retries += 1;
                if let Some(next) = choose_exit(state, world, from, Some(to)) {
                    state.transit[index].to = next;
                    state.transit[index].ticks = 1;
                } else if to != from && target(world, from) != Target::None {
                    state.transit[index].to = from;
                    state.transit[index].ticks = 1;
                } else {
                    state.transit[index].ticks = RETRY_TICKS;
                }
                index += 1;
            }
        }
    }
}

/// The world around a pipe at `pos`, through PlutoLib's neighbour reach.
pub struct Around {
    pub pos: BlockPos,
}

impl ItemWorld for Around {
    fn block_at(&mut self, side: Direction) -> Option<Identifier> {
        Neighbours::block_state(side.relative(self.pos)).map(|state| state.block)
    }
    fn slots_at(&mut self, side: Direction) -> Option<Vec<NeighbourSlot>> {
        let (pos, face) = Neighbours::beside(self.pos, side);
        Neighbours::item_slots(pos, face)
    }
    fn insert_at(&mut self, side: Direction, stack: ItemSlot) -> Option<ItemSlot> {
        let (pos, face) = Neighbours::beside(self.pos, side);
        Neighbours::insert_item(pos, face, stack)
    }
    fn extract_at(&mut self, side: Direction, slot: usize, count: u32) -> Option<ItemSlot> {
        let (pos, face) = Neighbours::beside(self.pos, side);
        Neighbours::extract_item(pos, face, slot, count)
    }
}

/// The direction an extraction pipe pulls from: its own `facing` property, north when unread.
fn facing_of(pos: BlockPos) -> Direction {
    Neighbours::block_state(pos)
        .and_then(|state| state.property("facing").and_then(Direction::parse))
        .unwrap_or(Direction::North)
}

pub(crate) fn tick_pipe(ctx: &mut BlockEntityTick<'_>, kind: Kind) {
    let mut state = PipeState::load(ctx.data);
    let mut world = Around { pos: ctx.pos };
    tick(&mut state, ctx.items, &mut world, kind, facing_of(ctx.pos));
    state.save(ctx.data);
}

pub(crate) fn signal_pipe(signal: &mut BlockEntitySignal<'_>) {
    let mut state = PipeState::load(signal.data);
    state.powered = signal.powered;
    state.save(signal.data);
}

pub fn register() {
    for (name, kind) in [(ITEM_PIPE, Kind::Transport), (EXTRACTION_PIPE, Kind::Extraction)] {
        let block = Identifier::new(NAMESPACE, name).expect("valid pipe id");
        let mut entity_type = BlockEntityType::new(block)
            .item_slots(6)
            .ticker(move |tick| tick_pipe(tick, kind))
            // One inbox slot per face, inserts only; nothing may be taken out of a pipe.
            .sided_access(|face| SidedAccess { insert: vec![face_index(face)], take: Vec::new() })
            .item_transfer_rule(|transfer| {
                transfer.mode == TransferMode::Insert
                    && transfer.face.is_some_and(|face| transfer.slot == face_index(face))
            });
        if kind == Kind::Extraction {
            entity_type = entity_type.on_signal(signal_pipe);
        }
        BlockEntityTypeRegistry::register(entity_type);
    }
}
