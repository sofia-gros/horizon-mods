//! A mod fluid as blocks in the world: its source and flowing cells and the buckets that place and pick them up (Forge:
//! `ForgeFlowingFluid` with `LiquidBlock` and `BucketItem`; Fabric: `FlowableFluid`).
//!
//! Horizon has no fluid kind for a mod, so a world fluid is one block of the mod, `<fluid id>_fluid` (as Mantle names it), with a `level` state: 0 is a source, 1 to 7 a flowing cell
//! (the higher, the thinner: its amount is `8 - level`) and 8 a column falling from above. [`FluidWorld`] holds the rules of
//! `FlowingFluid` over that block, through the block behaviour hooks (`scheduled_tick`, `update_shape`), so nothing here is a host
//! attachment of its own. The mod declares the block (see [`FluidWorld::BLOCK_LEVELS`]) and the bucket item in its content files.

use std::collections::HashMap;

use super::base::{ActionResult, Identifier};
use super::block_api_v1::{BlockBehavior, BlockBehaviors, BlockEditor, BlockView, Direction, ShapeUpdateContext, TickContext};
use super::block_entity_api_v1::BlockPos;
use super::fluid_api_v1::{FluidDef, FluidFlow, FluidRegistry};
use super::fluid_item_api_v1::{FluidItemDef, FluidItems};
use super::player_api::StackData;
use super::world_api_v1::{BlockState, World};

/// What one position around a fluid cell holds, for the fluid being spread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cell {
    /// Air or a replaceable block (grass, snow) that a fluid washes away: the fluid may spread into it.
    Empty,
    /// A cell of this fluid: its amount (1 to 8) and whether it is a source or a falling column.
    Own { amount: u8, source: bool },
    /// Anything else: stone, another fluid.
    Blocked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Level {
    amount: u8,
    falling: bool,
}

impl Level {
    fn source() -> Self {
        Level { amount: 8, falling: false }
    }
    /// The `level` state value: 0 for a source, `8 - amount` for a flowing cell, 8 for a falling one.
    fn state_level(self, source: bool) -> u8 {
        if source {
            0
        } else if self.falling {
            8
        } else {
            8 - self.amount.min(7)
        }
    }
}

/// The rules of a mod fluid in the world.
pub struct FluidWorld;

impl FluidWorld {
    /// The block state property that carries the fluid's level, and its values: `0` source, `1` to `7` flowing, `8` falling. A mod declares
    /// its fluid block with this property (as an `enum` of these values), a shape only on the source (so an empty bucket can reach it) and
    /// no collision.
    pub const LEVEL: &'static str = "level";
    /// The values of [`FluidWorld::LEVEL`].
    pub const BLOCK_LEVELS: [&'static str; 9] = ["0", "1", "2", "3", "4", "5", "6", "7", "8"];

    /// The block state of the fluid's block at a level (0 source, 1 to 7 flowing, 8 falling).
    pub fn state(fluid: &Identifier, level: u8) -> BlockState {
        BlockState::new(Self::block_id(fluid)).with(Self::LEVEL, &level.min(8).to_string())
    }

    /// The id of the fluid's block: the fluid's id with `_fluid`, so a fluid and a building block of the same name do not clash.
    pub fn block_id(fluid: &Identifier) -> Identifier {
        Identifier::new(fluid.namespace(), &format!("{}_fluid", fluid.path())).expect("an id with a suffix is an id")
    }

    /// The id of the fluid's bucket item: the fluid's id with `_bucket`.
    pub fn bucket_id(fluid: &Identifier) -> Identifier {
        Identifier::new(fluid.namespace(), &format!("{}_bucket", fluid.path())).expect("an id with a suffix is an id")
    }

    /// The level of a state of the fluid's block, if it is one.
    pub fn level_of(state: &BlockState, fluid: &Identifier) -> Option<u8> {
        if state.block != Self::block_id(fluid) {
            return None;
        }
        state.property(Self::LEVEL).and_then(|text| text.parse::<u8>().ok()).filter(|level| *level <= 8)
    }

    /// The source block of a fluid that has a world form: water and lava (whose block id is the
    /// fluid's own id) and a registered fluid with world blocks (its block at level 0). None for a
    /// fluid with no world form (a potion, powdered snow), so a bucket module's world halves know
    /// the fluid cannot be placed. The forward half of the fluid's world-block form, with
    /// [`FluidWorld::fluid_of`] as the reverse.
    pub fn source_block(fluid: &Identifier) -> Option<BlockState> {
        if fluid.to_string() == "minecraft:water" || fluid.to_string() == "minecraft:lava" {
            return Some(BlockState::new(fluid.clone()));
        }
        FluidRegistry::get(&fluid.to_string()).filter(|def| def.world_blocks).map(|_| Self::state(fluid, 0))
    }

    /// The fluid and level a block state holds, if it is a fluid's world form: a registered
    /// fluid's `<id>_fluid` block with its `level` (0 source, 1 to 7 flowing, 8 falling), or water
    /// and lava by their own block id (a missing `level` reads as a source, the form
    /// [`FluidWorld::source_block`] places; a vanilla level of 8 to 15 is a falling cell, the
    /// port's 8). None for any other block. The reverse half of the fluid's world-block form.
    pub fn fluid_of(state: &BlockState) -> Option<(Identifier, u8)> {
        if state.block.to_string() == "minecraft:water" || state.block.to_string() == "minecraft:lava" {
            let level = state.property(Self::LEVEL).and_then(|text| text.parse::<u8>().ok()).unwrap_or(0);
            return Some((state.block.clone(), if level <= 7 { level } else { 8 }));
        }
        let fluid = state.block.path().strip_suffix("_fluid").and_then(|path| Identifier::new(state.block.namespace(), path).ok())?;
        Self::level_of(state, &fluid).map(|level| (fluid, level))
    }

    /// Whether `block` is the block of a defined fluid that has world blocks.
    pub fn is_world_fluid(block: &Identifier) -> bool {
        let Some(fluid) = block.path().strip_suffix("_fluid") else { return false };
        Identifier::new(block.namespace(), fluid).ok().and_then(|id| FluidRegistry::get(&id.to_string())).is_some_and(|def| def.world_blocks)
    }

    /// Puts a source of the fluid at `pos` and asks for its first tick. Only into air or a replaceable block that holds no fluid; false when
    /// there is no room or the game refused. A replaceable block (grass) is broken with its drops first.
    pub fn place_source(world: &mut dyn BlockEditor, pos: BlockPos, def: &FluidDef) -> bool {
        match probe(&*world, pos, &def.id) {
            Some(Cell::Empty) => {}
            _ => return false,
        }
        if !world.is_air(pos) {
            world.break_block(pos, true);
        }
        if !world.set_block_state(pos, &Self::state(&def.id, 0)) {
            return false;
        }
        world.schedule_tick(pos, &FluidWorld::block_id(&def.id).to_string(), i32::from(def.flow.tick_delay.max(1)));
        true
    }

    /// Removes the source at `pos` if it is a source of this fluid (a flowing cell cannot be bucketed) and tells the neighbours to settle.
    pub fn pick_up_source(world: &mut dyn BlockEditor, pos: BlockPos, def: &FluidDef) -> bool {
        let Some(state) = world.block_state(pos) else { return false };
        if Self::level_of(&state, &def.id) != Some(0) {
            return false;
        }
        let Ok(air) = BlockState::parse("minecraft:air") else { return false };
        if !world.set_block_state(pos, &air) {
            return false;
        }
        wake_neighbours(world, pos, def);
        true
    }

    /// One scheduled tick of a cell of the fluid (`FlowingFluid.tick`): a flowing cell is first set to what its neighbours make of it (or
    /// removed), then every cell spreads down, or sideways toward the nearest drop. The cells it changes are scheduled again.
    pub fn tick(world: &mut dyn BlockEditor, pos: BlockPos, def: &FluidDef) {
        let Some(state) = world.block_state(pos) else { return };
        let Some(own) = Self::level_of(&state, &def.id) else { return };
        let delay = i32::from(def.flow.tick_delay.max(1));
        let block = FluidWorld::block_id(&def.id).to_string();
        let mut here = Level { amount: if own == 0 { 8 } else if own == 8 { 8 } else { 8 - own }, falling: own == 8 };
        if own != 0 {
            match new_liquid(&*world, pos, def) {
                Some(Some(level)) => {
                    let wanted = level.state_level(false);
                    if wanted != own {
                        here = level;
                        if !world.set_block_state(pos, &Self::state(&def.id, wanted)) {
                            return;
                        }
                        world.schedule_tick(pos, &block, delay);
                        wake_neighbours(world, pos, def);
                    }
                }
                Some(None) => {
                    if let Ok(air) = BlockState::parse("minecraft:air") {
                        world.set_block_state(pos, &air);
                    }
                    wake_neighbours(world, pos, def);
                    return;
                }
                // not loaded around: try again later
                None => {
                    world.schedule_tick(pos, &block, delay);
                    return;
                }
            }
        }
        spread(world, pos, def, here, own == 0);
    }

    /// Registers the block behaviour that makes the fluid's block flow: `scheduled_tick` runs [`FluidWorld::tick`], `update_shape` asks for
    /// a tick when a neighbour changes, `random_tick` runs a tick for a cell nobody scheduled (a block placed by a command), and the block
    /// survives anywhere. Called by [`FluidRegistry::register`] for a fluid with world blocks; later behaviours for the same block should
    /// extend `BlockBehaviors::get` instead of replacing it.
    pub fn register_behaviour(def: &FluidDef) {
        let tick_def = def.clone();
        let random_def = def.clone();
        let shape_def = def.clone();
        let behaviour = BlockBehavior::new(FluidWorld::block_id(&def.id))
            .scheduled_tick(move |context: &mut TickContext<'_>| FluidWorld::tick(&mut *context.world, context.pos, &tick_def))
            .random_tick(move |context: &mut TickContext<'_>| FluidWorld::tick(&mut *context.world, context.pos, &random_def))
            .update_shape(move |context: &ShapeUpdateContext<'_>| {
                if FluidWorld::level_of(&context.state, &shape_def.id).is_some() {
                    context.schedule_tick(i32::from(shape_def.flow.tick_delay.max(1)));
                }
                None
            })
            .can_survive(|_| true);
        BlockBehaviors::register(behaviour);
    }
}

/// Asks for a tick of every neighbour of `pos`, as a changed block tells its neighbours to look again.
fn wake_neighbours(world: &mut dyn BlockEditor, pos: BlockPos, def: &FluidDef) {
    let (block, delay) = (FluidWorld::block_id(&def.id).to_string(), i32::from(def.flow.tick_delay.max(1)));
    for direction in NEIGHBOURS {
        world.schedule_tick(offset(pos, direction), &block, delay);
    }
}

const HORIZONTAL: [Direction; 4] = [Direction::North, Direction::South, Direction::West, Direction::East];
const NEIGHBOURS: [Direction; 6] = [Direction::Down, Direction::Up, Direction::North, Direction::South, Direction::West, Direction::East];

fn offset(pos: BlockPos, direction: Direction) -> BlockPos {
    direction.relative(pos)
}

/// Where this fluid falls: down, or up for a fluid lighter than air (negative density).
fn falls(def: &FluidDef) -> (Direction, Direction) {
    if def.attributes.density < 0 {
        (Direction::Up, Direction::Down)
    } else {
        (Direction::Down, Direction::Up)
    }
}

fn is_other_fluid(world: &dyn BlockView, pos: BlockPos, state: &BlockState) -> bool {
    matches!(state.block.path(), "water" | "lava" | "bubble_column") && state.block.namespace() == "minecraft"
        || world.is_water(pos)
        || FluidWorld::is_world_fluid(&state.block)
}

fn probe(world: &dyn BlockView, pos: BlockPos, fluid: &Identifier) -> Option<Cell> {
    // the build limits are walls: a fluid does not fall out of the world or climb out of it
    let [bottom, top] = world.build_height();
    if pos[1] < bottom || pos[1] >= top {
        return Some(Cell::Blocked);
    }
    let state = world.block_state(pos)?;
    if let Some(level) = FluidWorld::level_of(&state, fluid) {
        return Some(Cell::Own { amount: if level == 0 || level == 8 { 8 } else { 8 - level }, source: level == 0 });
    }
    if is_other_fluid(world, pos, &state) {
        return Some(Cell::Blocked);
    }
    Some(if world.is_air(pos) || world.is_replaceable(pos) { Cell::Empty } else { Cell::Blocked })
}

/// Reads each position once per decision.
struct Reader<'a> {
    world: &'a dyn BlockView,
    fluid: &'a Identifier,
    cells: HashMap<BlockPos, Option<Cell>>,
}

impl Reader<'_> {
    fn cell(&mut self, pos: BlockPos) -> Option<Cell> {
        *self.cells.entry(pos).or_insert_with(|| probe(self.world, pos, self.fluid))
    }
}

/// `FlowingFluid.getNewLiquid`: what the cell at `pos` becomes from its neighbours. The outer `None` is "not loaded"; the inner `None` is no
/// fluid; `Some(Some(level))` a cell with that amount (falling when the fluid above feeds it).
fn new_liquid(world: &dyn BlockView, pos: BlockPos, def: &FluidDef) -> Option<Option<Level>> {
    let mut reader = Reader { world, fluid: &def.id, cells: HashMap::new() };
    new_liquid_with(&mut reader, pos, def)
}

fn new_liquid_with(reader: &mut Reader<'_>, pos: BlockPos, def: &FluidDef) -> Option<Option<Level>> {
    let (_, up) = falls(def);
    let mut strongest = 0u8;
    for direction in HORIZONTAL {
        if let Cell::Own { amount, .. } = reader.cell(offset(pos, direction))? {
            strongest = strongest.max(amount);
        }
    }
    if matches!(reader.cell(offset(pos, up))?, Cell::Own { .. }) {
        return Some(Some(Level { amount: 8, falling: true }));
    }
    let amount = i32::from(strongest) - i32::from(def.flow.level_drop);
    Some((amount > 0).then_some(Level { amount: amount as u8, falling: false }))
}

/// `FlowingFluid.spread`.
fn spread(world: &mut dyn BlockEditor, pos: BlockPos, def: &FluidDef, here: Level, source: bool) {
    let (down, _) = falls(def);
    let delay = i32::from(def.flow.tick_delay.max(1));
    let block = FluidWorld::block_id(&def.id).to_string();
    let below = offset(pos, down);
    let mut reader = Reader { world: &*world, fluid: &def.id, cells: HashMap::new() };
    let Some(below_cell) = reader.cell(below) else {
        world.schedule_tick(pos, &block, delay);
        return;
    };
    let mut sources = 0;
    for direction in HORIZONTAL {
        if matches!(reader.cell(offset(pos, direction)), Some(Cell::Own { source: true, .. })) {
            sources += 1;
        }
    }
    let mut writes: Vec<(BlockPos, Level)> = Vec::new();
    let mut to_sides = false;
    if below_cell == Cell::Empty {
        // the cell below takes the column the fluid above feeds
        writes.push((below, Level { amount: 8, falling: true }));
        to_sides = sources >= 3;
    } else if source || !is_hole(&mut reader, below_cell) {
        to_sides = true;
    }
    if to_sides {
        let amount = if here.falling { 7 } else { i32::from(here.amount) - i32::from(def.flow.level_drop) };
        if amount > 0 {
            let spreads = spread_directions(&mut reader, pos, def);
            for (position, level) in spreads {
                if reader.cell(position) == Some(Cell::Empty) {
                    writes.push((position, level));
                }
            }
        }
    }
    for (position, level) in writes {
        if !world.is_air(position) {
            world.break_block(position, true);
        }
        if world.set_block_state(position, &FluidWorld::state(&def.id, level.state_level(false))) {
            world.schedule_tick(position, &block, delay);
            wake_neighbours(world, position, def);
        }
    }
}

/// `isWaterHole` for the cell below a position: a place the fluid would go down into.
fn is_hole(_reader: &mut Reader<'_>, below: Cell) -> bool {
    matches!(below, Cell::Empty | Cell::Own { .. })
}

/// `FlowingFluid.getSpread`: the sideways cells the fluid goes to, and the level each takes. Those with the shortest way down within the slope
/// distance win; with no hole near, all of them.
fn spread_directions(reader: &mut Reader<'_>, pos: BlockPos, def: &FluidDef) -> Vec<(BlockPos, Level)> {
    let (down, _) = falls(def);
    let mut best = 1000;
    let mut found: Vec<(BlockPos, Level)> = Vec::new();
    for direction in HORIZONTAL {
        let next = offset(pos, direction);
        let Some(cell) = reader.cell(next) else { continue };
        if cell == Cell::Blocked {
            continue;
        }
        let Some(Some(level)) = new_liquid_with(reader, next, def) else { continue };
        let distance = if reader.cell(offset(next, down)).is_some_and(|below| is_hole(reader, below)) {
            0
        } else {
            slope_distance(reader, next, 1, direction.opposite(), def)
        };
        if distance < best {
            found.clear();
        }
        if distance <= best {
            found.push((next, level));
            best = distance;
        }
    }
    found
}

/// `FlowingFluid.getSlopeDistance`: how far, up to the fluid's slope distance, the way from `pos` leads to a place to go down.
fn slope_distance(reader: &mut Reader<'_>, pos: BlockPos, depth: u8, from: Direction, def: &FluidDef) -> i32 {
    let (down, _) = falls(def);
    let mut best = 1000;
    for direction in HORIZONTAL {
        if direction == from {
            continue;
        }
        let next = offset(pos, direction);
        let Some(cell) = reader.cell(next) else { continue };
        if cell == Cell::Blocked {
            continue;
        }
        if reader.cell(offset(next, down)).is_some_and(|below| is_hole(reader, below)) {
            return i32::from(depth);
        }
        if depth < def.flow.slope_distance {
            best = best.min(slope_distance(reader, next, depth + 1, direction.opposite(), def));
        }
    }
    best
}

// --- buckets

/// The sound a bucket makes: the fluid's own, or the water bucket's.
fn bucket_sound(def: &FluidDef, filling: bool) -> String {
    let own = if filling { &def.bucket_fill_sound } else { &def.bucket_empty_sound };
    own.clone().unwrap_or_else(|| if filling { "minecraft:item.bucket.fill" } else { "minecraft:item.bucket.empty" }.to_owned())
}

/// The fluid, if any, whose bucket item is `item`: its source is placed by it.
fn filled_bucket_fluid(item: &str) -> Option<FluidDef> {
    FluidRegistry::registered().into_iter().find(|def| def.bucket && FluidWorld::bucket_id(&def.id).to_string() == item)
}

/// A player used an item on a block: a filled bucket of a mod fluid puts its source there; an empty bucket picks up the source it is aimed
/// at. Only a world fluid with a bucket can do either (a bucket of a fluid with no world blocks is a container only).
pub(crate) fn use_bucket(used: &super::events_interaction_v0::UseItemOnBlockContext) -> ActionResult {
    let Some(held) = used.held.as_ref() else { return ActionResult::Pass };
    let world = World::new(&used.dimension);
    if let Some(def) = filled_bucket_fluid(&held.item).filter(|def| def.world_blocks) {
        // the clicked block when it has room, otherwise the block against the clicked face
        let Some(clicked) = world.get_block_state(used.pos) else { return ActionResult::Pass };
        let target = match probe_state(&world, used.pos, &clicked, &def.id) {
            Some(Cell::Empty) => used.pos,
            _ => [used.pos[0] + used.face[0], used.pos[1] + used.face[1], used.pos[2] + used.face[2]],
        };
        let Some(state) = world.get_block_state(target) else { return ActionResult::Pass };
        if probe_state(&world, target, &state, &def.id) != Some(Cell::Empty) {
            return ActionResult::Pass;
        }
        if !world.set_block_state(target, &FluidWorld::state(&def.id, 0)) {
            return ActionResult::Pass;
        }
        world.schedule_tick(target, &FluidWorld::block_id(&def.id).to_string(), i32::from(def.flow.tick_delay.max(1)));
        world.play_sound(&bucket_sound(&def, false), centre(target), 1.0, 1.0);
        swap_held(used, held, StackData { item: "minecraft:bucket".to_owned(), count: 1, ..stack_of(held) });
        return ActionResult::Success;
    }
    if held.item == "minecraft:bucket" {
        let Some(clicked) = world.get_block_state(used.pos) else { return ActionResult::Pass };
        let Some(def) = FluidRegistry::registered().into_iter().find(|def| def.world_blocks && def.bucket && FluidWorld::level_of(&clicked, &def.id) == Some(0)) else {
            return ActionResult::Pass;
        };
        let Ok(air) = BlockState::parse("minecraft:air") else { return ActionResult::Pass };
        if !world.set_block_state(used.pos, &air) {
            return ActionResult::Pass;
        }
        for direction in NEIGHBOURS {
            world.schedule_tick(offset(used.pos, direction), &FluidWorld::block_id(&def.id).to_string(), i32::from(def.flow.tick_delay.max(1)));
        }
        world.play_sound(&bucket_sound(&def, true), centre(used.pos), 1.0, 1.0);
        let filled = StackData { item: FluidWorld::bucket_id(&def.id).to_string(), count: 1, ..stack_of(held) };
        swap_held(used, held, filled);
        return ActionResult::Success;
    }
    ActionResult::Pass
}

fn centre(pos: BlockPos) -> [f64; 3] {
    [f64::from(pos[0]) + 0.5, f64::from(pos[1]) + 0.5, f64::from(pos[2]) + 0.5]
}

fn stack_of(held: &StackData) -> StackData {
    StackData { item: held.item.clone(), count: held.count, damage: 0, max_damage: None, custom_data: Default::default(), custom_model_data: None, can_place_on: held.can_place_on.clone(), enchantments: held.enchantments.clone() }
}

/// Probes a position whose state is already known.
fn probe_state(world: &World, pos: BlockPos, state: &BlockState, fluid: &Identifier) -> Option<Cell> {
    if let Some(level) = FluidWorld::level_of(state, fluid) {
        return Some(Cell::Own { amount: if level == 0 || level == 8 { 8 } else { 8 - level }, source: level == 0 });
    }
    let _ = world;
    let path = state.block.path();
    let other_fluid = state.block.namespace() == "minecraft" && matches!(path, "water" | "lava" | "bubble_column") || FluidWorld::is_world_fluid(&state.block);
    if other_fluid {
        return Some(Cell::Blocked);
    }
    let _ = pos;
    Some(if state.block.namespace() == "minecraft" && matches!(path, "air" | "cave_air" | "void_air" | "short_grass" | "tall_grass" | "fern" | "snow" | "dead_bush") { Cell::Empty } else { Cell::Blocked })
}

/// Takes one of the held stack and gives one `result` in its place (a creative player keeps the held stack and gets the filled bucket only
/// when carrying none).
fn swap_held(used: &super::events_interaction_v0::UseItemOnBlockContext, held: &StackData, result: StackData) {
    if used.creative {
        if used.player.count_item(&result.item) == 0 {
            used.player.give_stack(result);
        }
        return;
    }
    let slot = (0..36).find(|slot| used.player.stack(*slot).is_some_and(|stack| stack.item == held.item && stack.custom_data == held.custom_data));
    let Some(slot) = slot else {
        used.player.give_stack(result);
        return;
    };
    if held.count <= 1 {
        used.player.update_stack(slot, &result);
    } else {
        let mut rest = held.clone();
        rest.count -= 1;
        used.player.update_stack(slot, &rest);
        used.player.give_stack(result);
    }
}

/// The game's own fluids and their bucket items. Nobody registers water or lava (the game already has them),
/// so no fluid registration ever builds their containers — yet their buckets are containers like any fluid's:
/// a lava bucket empties into a tank and an empty bucket fills from one, as the original's tank interaction
/// (`FluidTransferHelper.interactWithTank`) does with every bucket the platform knows. They ride along with
/// every bucket registration below, so the shared empty bucket rebuilt there always knows them too.
const VANILLA_BUCKETS: [(&str, &str); 2] = [("minecraft:water", "minecraft:water_bucket"), ("minecraft:lava", "minecraft:lava_bucket")];

/// Registers the bucket of a fluid as a container and makes the buckets work on blocks. Called by [`FluidRegistry::register`] for a fluid
/// with a bucket.
pub(crate) fn register_bucket(def: &FluidDef) {
    // 1000 millibuckets, back to an empty bucket; the empty bucket fills with this fluid (and with others registered the same way).
    // A fluid whose bucket carries the tag gets the tagged container instead: the bucket item's own data is the fluid's tag.
    let empty_bucket = Identifier::new("minecraft", "bucket").expect("a fixed id");
    let bucket_item = FluidWorld::bucket_id(&def.id);
    if def.bucket_carries_tag {
        FluidItems::register(FluidItemDef::tagged_constant(bucket_item, def.id.clone(), 1000, Some(empty_bucket.clone())));
    } else {
        FluidItems::register(FluidItemDef::constant(bucket_item, def.id.clone(), 1000, Some(empty_bucket.clone())));
    }
    for (fluid, bucket) in VANILLA_BUCKETS {
        let fluid = Identifier::parse(fluid).expect("a fixed id");
        let bucket = Identifier::parse(bucket).expect("a fixed id");
        FluidItems::register(FluidItemDef::constant(bucket, fluid, 1000, Some(empty_bucket.clone())));
    }
    let mut fluids: Vec<(Identifier, Identifier, bool)> = FluidRegistry::registered()
        .into_iter()
        .filter(|other| other.bucket && other.id != def.id)
        .chain(std::iter::once(def.clone()))
        .map(|other| (other.id.clone(), FluidWorld::bucket_id(&other.id), other.bucket_carries_tag))
        .collect();
    for (fluid, bucket) in VANILLA_BUCKETS {
        let fluid = Identifier::parse(fluid).expect("a fixed id");
        if !fluids.iter().any(|(known, _, _)| *known == fluid) {
            fluids.push((fluid, Identifier::parse(bucket).expect("a fixed id"), false));
        }
    }
    let mut buckets = fluids.into_iter();
    if let Some((first, first_bucket, first_tagged)) = buckets.next() {
        let mut item = if first_tagged {
            FluidItemDef::empty_container_tagged(empty_bucket.clone(), move |fluid| fluid == &first, 1000, first_bucket)
        } else {
            FluidItemDef::empty_container(empty_bucket.clone(), move |fluid| fluid == &first, 1000, first_bucket)
        };
        for (fluid, bucket, tagged) in buckets {
            item = if tagged {
                item.fills_tagged(move |candidate| candidate == &fluid, 1000, bucket)
            } else {
                item.fills(move |candidate| candidate == &fluid, 1000, bucket)
            };
        }
        FluidItems::register(item);
    }
    install_use_callback();
}

fn install_use_callback() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        super::events_interaction_v0::UseItemOnBlockCallback::EVENT.register(|used| use_bucket(used));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::block_api_v1::MemoryBlockView;

    fn id(text: &str) -> Identifier {
        Identifier::parse(text).unwrap()
    }
    fn stone(world: &mut MemoryBlockView, pos: BlockPos) {
        world.blocks.insert(pos, BlockState::parse("minecraft:stone").unwrap());
    }
    fn level(world: &MemoryBlockView, pos: BlockPos, fluid: &Identifier) -> Option<u8> {
        world.blocks.get(&pos).and_then(|state| FluidWorld::level_of(state, fluid))
    }
    /// A stone floor at y 0, `half` blocks each way from the origin.
    fn floor(half: i32) -> MemoryBlockView {
        let mut world = MemoryBlockView::default();
        for x in -half..=half {
            for z in -half..=half {
                stone(&mut world, [x, 0, z]);
            }
        }
        world
    }
    /// Runs scheduled ticks until none are left (each tick may schedule more).
    fn settle(world: &mut MemoryBlockView, def: &FluidDef) -> usize {
        let mut ticks = 0;
        while let Some((pos, _, _)) = world.scheduled.first().cloned() {
            world.scheduled.remove(0);
            FluidWorld::tick(world, pos, def);
            ticks += 1;
            assert!(ticks < 5000, "the fluid never settled");
        }
        ticks
    }

    #[test]
    fn a_source_spreads_as_far_as_its_drop_per_block_allows_and_a_source_in_the_air_pours_down() {
        let def = FluidDef::new(id("flow_test:slime")).with_world_blocks().flow(5, 1, 4);
        let mut world = floor(12);
        assert!(FluidWorld::place_source(&mut world, [0, 1, 0], &def));
        assert!(!FluidWorld::place_source(&mut world, [0, 1, 0], &def), "a source cannot be put where there is a source");
        assert_eq!(world.scheduled.first().map(|(pos, block, delay)| (*pos, block.as_str(), *delay)), Some(([0, 1, 0], "flow_test:slime_fluid", 5)));
        settle(&mut world, &def);
        let fluid = &def.id;
        assert_eq!(level(&world, [0, 1, 0], fluid), Some(0));
        // amount 7 beside the source (level 1), thinner each block, 7 blocks at most
        assert_eq!(level(&world, [1, 1, 0], fluid), Some(1));
        assert_eq!(level(&world, [3, 1, 0], fluid), Some(3));
        assert_eq!(level(&world, [7, 1, 0], fluid), Some(7));
        assert_eq!(level(&world, [8, 1, 0], fluid), None, "no fluid beyond amount 1");
        assert_eq!(level(&world, [1, 1, 1], fluid), Some(2), "the diagonal is two blocks away");

        // a source three blocks up pours a falling column and spreads at its own height too, as water does
        let mut air = floor(12);
        assert!(FluidWorld::place_source(&mut air, [0, 3, 0], &def));
        settle(&mut air, &def);
        assert_eq!(level(&air, [0, 2, 0], fluid), Some(8), "a falling column under the source");
        assert_eq!(level(&air, [0, 1, 0], fluid), Some(8));
        assert_eq!(level(&air, [1, 3, 0], fluid), Some(1), "the source spreads on top of its own column");
        assert_eq!(level(&air, [1, 2, 0], fluid), Some(8), "and those cells pour down in turn");
    }

    #[test]
    fn a_larger_drop_per_block_flows_less_far_and_a_removed_source_drains_what_it_fed() {
        let def = FluidDef::new(id("flow_test:glass")).with_world_blocks().flow(30, 3, 3);
        let mut world = floor(12);
        assert!(FluidWorld::place_source(&mut world, [0, 1, 0], &def));
        settle(&mut world, &def);
        let fluid = &def.id;
        assert_eq!(level(&world, [1, 1, 0], fluid), Some(3), "8 less 3 leaves amount 5 beside the source");
        assert_eq!(level(&world, [2, 1, 0], fluid), Some(6));
        assert_eq!(level(&world, [3, 1, 0], fluid), None, "amount 2 less 3 is nothing");
        assert!(FluidWorld::pick_up_source(&mut world, [0, 1, 0], &def));
        assert!(!FluidWorld::pick_up_source(&mut world, [1, 1, 0], &def), "a flowing cell is not bucketed");
        settle(&mut world, &def);
        assert!((-6..=6).all(|x| level(&world, [x, 1, 0], fluid).is_none()), "the flow dries up with its source");
    }

    #[test]
    fn a_fluid_flows_toward_the_nearest_drop_and_a_lighter_fluid_rises() {
        let def = FluidDef::new(id("flow_test:water")).with_world_blocks().flow(5, 1, 4);
        let mut world = floor(12);
        // a hole two blocks east of the source: the fluid goes east only
        world.blocks.remove(&[3, 0, 0]);
        assert!(FluidWorld::place_source(&mut world, [0, 1, 0], &def));
        settle(&mut world, &def);
        let fluid = &def.id;
        assert_eq!(level(&world, [1, 1, 0], fluid), Some(1));
        assert_eq!(level(&world, [-1, 1, 0], fluid), None, "no spread away from the drop");
        assert_eq!(level(&world, [0, 1, 1], fluid), None);
        assert_eq!(level(&world, [3, 0, 0], fluid), Some(8), "it pours into the hole");

        let gas = FluidDef::new(id("flow_test:ichor")).with_world_blocks().density(-1600).flow(5, 1, 4);
        let mut sky = MemoryBlockView::default();
        for x in -12..=12 {
            for z in -12..=12 {
                stone(&mut sky, [x, 6, z]);
            }
        }
        assert!(FluidWorld::place_source(&mut sky, [0, 3, 0], &gas));
        settle(&mut sky, &gas);
        assert_eq!(level(&sky, [0, 4, 0], &gas.id), Some(8), "a negative density fluid is a column that climbs");
        assert_eq!(level(&sky, [0, 5, 0], &gas.id), Some(8));
        assert_eq!(level(&sky, [1, 3, 0], &gas.id), Some(1), "the source spreads at its own height");
        assert_eq!(level(&sky, [1, 4, 0], &gas.id), Some(8), "and the cells beside it climb in turn");
        assert_eq!(level(&sky, [0, 2, 0], &gas.id), None);
    }

    #[test]
    fn a_fluid_goes_around_stone_never_into_it_and_never_replaces_another_fluid() {
        let def = FluidDef::new(id("flow_test:lava")).with_world_blocks().flow(5, 1, 4);
        let mut world = floor(12);
        stone(&mut world, [1, 1, 0]);
        world.water.insert([-1, 1, 0]);
        world.blocks.insert([-1, 1, 0], BlockState::parse("minecraft:water[level=0]").unwrap());
        world.replaceable.insert([0, 1, 1]);
        world.blocks.insert([0, 1, 1], BlockState::parse("minecraft:short_grass").unwrap());
        assert!(FluidWorld::place_source(&mut world, [0, 1, 0], &def));
        assert!(!FluidWorld::place_source(&mut world, [1, 1, 0], &def), "stone has no room");
        assert!(!FluidWorld::place_source(&mut world, [-1, 1, 0], &def), "water is not replaced by a bucket");
        settle(&mut world, &def);
        assert_eq!(level(&world, [1, 1, 0], &def.id), None);
        assert_eq!(world.blocks[&[1, 1, 0]].block.path(), "stone");
        assert_eq!(world.blocks[&[-1, 1, 0]].block.path(), "water");
        assert_eq!(level(&world, [0, 1, 1], &def.id), Some(1), "grass is washed away with its drops");
        assert!(world.break_requests.contains(&([0, 1, 1], true)));
        assert_eq!(level(&world, [2, 1, 0], &def.id), Some(4), "round the stone block: two cells further than the diagonal");
    }

    #[test]
    fn registering_a_world_fluid_gives_its_block_a_flow_behaviour_and_its_bucket_a_container() {
        FluidRegistry::register(FluidDef::new(id("flow_test:honey")).with_world_blocks().with_bucket().flow(5, 1, 4));
        let behaviour = BlockBehaviors::get("flow_test:honey_fluid").expect("the fluid block has a behaviour");
        let _ = behaviour;
        let bucket = FluidWorld::bucket_id(&id("flow_test:honey"));
        assert_eq!(bucket.to_string(), "flow_test:honey_bucket");
        let full = super::super::block_entity_api_v1::ItemSlot::new(bucket, 1);
        assert_eq!(FluidItems::contents(&full).map(|fluid| (fluid.fluid.to_string(), fluid.amount)), Some(("flow_test:honey".to_owned(), 1000)));
        let empty = super::super::block_entity_api_v1::ItemSlot::new(id("minecraft:bucket"), 1);
        let filled = FluidItems::fill(&empty, &super::super::block_entity_api_v1::FluidStack { fluid: id("flow_test:honey"), amount: 1000, tag: None });
        assert_eq!(filled.container.item.to_string(), "flow_test:honey_bucket");
        assert_eq!(FluidWorld::state(&id("flow_test:honey"), 3).to_string(), "flow_test:honey_fluid[level=3]");
        assert_eq!(FluidWorld::level_of(&FluidWorld::state(&id("flow_test:honey"), 8), &id("flow_test:honey")), Some(8));
        assert_eq!(FluidWorld::level_of(&BlockState::parse("flow_test:honey_fluid[level=9]").unwrap(), &id("flow_test:honey")), None);
        // a fluid without world blocks gets no block behaviour
        FluidRegistry::register(FluidDef::new(id("flow_test:potion")).with_bucket());
        assert!(BlockBehaviors::get("flow_test:potion_fluid").is_none());
    }

    #[test]
    fn a_bucket_registration_also_makes_the_vanilla_buckets_containers() {
        FluidRegistry::register(FluidDef::new(id("flow_test:bucketed")).with_bucket());
        let empty = super::super::block_entity_api_v1::ItemSlot::new(id("minecraft:bucket"), 1);
        // The empty bucket fills with the game's own fluids and becomes their buckets; a filled vanilla
        // bucket reads as its fluid and drains back to an empty bucket.
        for (fluid, bucket) in [("minecraft:lava", "minecraft:lava_bucket"), ("minecraft:water", "minecraft:water_bucket")] {
            let filled = FluidItems::fill(&empty, &super::super::block_entity_api_v1::FluidStack { fluid: id(fluid), amount: 1000, tag: None });
            assert_eq!((filled.amount, filled.container.item.to_string()), (1000, bucket.to_owned()), "{fluid}");
            assert_eq!(FluidItems::contents(&filled.container).map(|held| (held.fluid.to_string(), held.amount)), Some((fluid.to_owned(), 1000)));
            let drained = FluidItems::drain(&filled.container, None, 1000).unwrap();
            assert_eq!((drained.fluid.fluid.to_string(), drained.container.unwrap().item.to_string()), (fluid.to_owned(), "minecraft:bucket".to_owned()));
        }
    }

    #[test]
    fn a_tagged_bucket_keeps_the_fluids_tag_on_the_item_and_needs_no_world_blocks() {
        FluidRegistry::register(FluidDef::new(id("flow_test:tagged_potion")).with_bucket().bucket_carries_tag());
        assert!(BlockBehaviors::get("flow_test:tagged_potion_fluid").is_none(), "no world blocks, no flow behaviour");
        let mut tag = super::super::base::NbtCompound::new();
        tag.insert("Potion".to_owned(), super::super::base::Nbt::String("minecraft:healing".to_owned()));
        let potion = super::super::block_entity_api_v1::FluidStack { fluid: id("flow_test:tagged_potion"), amount: 1000, tag: Some(tag.clone()) };
        let empty = super::super::block_entity_api_v1::ItemSlot::new(id("minecraft:bucket"), 1);
        let filled = FluidItems::fill(&empty, &potion);
        assert_eq!(filled.container.item.to_string(), "flow_test:tagged_potion_bucket");
        assert_eq!(filled.container.custom_data, tag, "the tag is the bucket item's own data");
        assert_eq!(FluidItems::contents(&filled.container), Some(potion.clone()));
        let drained = FluidItems::drain(&filled.container, None, 1000).unwrap();
        assert_eq!(drained.fluid, potion);
        assert_eq!(drained.container.unwrap().item, id("minecraft:bucket"));
    }

    #[test]
    fn a_fluids_world_form_round_trips_between_block_and_fluid() {
        let def = FluidDef::new(id("flow_test:molten")).with_world_blocks().with_bucket();
        FluidRegistry::register(def.clone());
        // The forward form: a source block for a registered fluid, and for water and lava.
        assert_eq!(FluidWorld::source_block(&def.id).map(|state| state.to_string()), Some("flow_test:molten_fluid[level=0]".to_owned()));
        assert_eq!(FluidWorld::source_block(&id("minecraft:water")).map(|state| state.to_string()), Some("minecraft:water".to_owned()));
        assert_eq!(FluidWorld::source_block(&id("minecraft:lava")).map(|state| state.to_string()), Some("minecraft:lava".to_owned()));
        // A fluid with no world form gets none.
        FluidRegistry::register(FluidDef::new(id("flow_test:still_potion")).with_bucket());
        assert!(FluidWorld::source_block(&id("flow_test:still_potion")).is_none());
        assert!(FluidWorld::source_block(&id("flow_test:unknown")).is_none());
        // The reverse form: the block reads back as its fluid and level.
        assert_eq!(FluidWorld::fluid_of(&FluidWorld::state(&def.id, 0)), Some((def.id.clone(), 0)));
        assert_eq!(FluidWorld::fluid_of(&FluidWorld::state(&def.id, 3)), Some((def.id.clone(), 3)));
        assert_eq!(FluidWorld::fluid_of(&FluidWorld::state(&def.id, 8)), Some((def.id.clone(), 8)));
        assert_eq!(FluidWorld::fluid_of(&BlockState::parse("minecraft:water[level=0]").unwrap()), Some((id("minecraft:water"), 0)));
        assert_eq!(FluidWorld::fluid_of(&BlockState::parse("minecraft:water[level=4]").unwrap()), Some((id("minecraft:water"), 4)));
        assert_eq!(FluidWorld::fluid_of(&BlockState::parse("minecraft:water[level=12]").unwrap()), Some((id("minecraft:water"), 8)), "a falling water cell reads as falling");
        assert_eq!(FluidWorld::fluid_of(&BlockState::new(id("minecraft:lava"))), Some((id("minecraft:lava"), 0)), "a lava block with no level is a source");
        assert_eq!(FluidWorld::fluid_of(&BlockState::parse("minecraft:stone").unwrap()), None);
        assert_eq!(FluidWorld::fluid_of(&BlockState::parse("flow_test:molten_fluid").unwrap()), None, "a mod fluid block with no level is no form");
    }
}
