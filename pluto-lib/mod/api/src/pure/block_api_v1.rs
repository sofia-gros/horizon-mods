//! What a mod's own blocks do (Minecraft: `Block.canSurvive`, `getStateForPlacement`, `updateShape`, `entityInside`).
//!
//! A [`BlockBehavior`] belongs to one block of the mod and answers four questions the game asks about it: where it may stand, which state it
//! takes when placed, how a neighbour changes it, and what happens to an entity inside it. The block itself (its template, its state
//! properties, its shapes) is declared in the mod's content files; the behaviour is what those files cannot say.
//!
//! The callbacks see states as [`BlockState`] text and the surroundings through a [`BlockView`]. PlutoLib owns the game's hook for each
//! question once, so two mods never compete for it: each behaviour is asked only about its own block.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use super::base::Identifier;
use super::block_entity_api_v1::BlockPos;
use super::item_group_api_v1::ItemStack;
use super::world_api_v1::BlockState;

/// One of the six block faces (Minecraft: `Direction`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Direction {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl Direction {
    /// All six, in the game's order.
    pub const ALL: [Direction; 6] = [Direction::Down, Direction::Up, Direction::North, Direction::South, Direction::West, Direction::East];

    /// The name a block state spells it with: `down`, `up`, `north`, `south`, `west`, `east`.
    pub fn name(self) -> &'static str {
        match self {
            Direction::Down => "down",
            Direction::Up => "up",
            Direction::North => "north",
            Direction::South => "south",
            Direction::West => "west",
            Direction::East => "east",
        }
    }

    /// Reads a name written by [`Direction::name`].
    pub fn parse(name: &str) -> Option<Direction> {
        Direction::ALL.into_iter().find(|d| d.name() == name)
    }

    /// The step one block in this direction takes: `[0, 1, 0]` for up, `[0, 0, -1]` for north.
    pub fn offset(self) -> [i32; 3] {
        match self {
            Direction::Down => [0, -1, 0],
            Direction::Up => [0, 1, 0],
            Direction::North => [0, 0, -1],
            Direction::South => [0, 0, 1],
            Direction::West => [-1, 0, 0],
            Direction::East => [1, 0, 0],
        }
    }

    /// Reads a step written by [`Direction::offset`].
    pub fn from_offset(offset: [i32; 3]) -> Option<Direction> {
        Direction::ALL.into_iter().find(|d| d.offset() == offset)
    }

    /// The other way: down for up, north for south.
    pub fn opposite(self) -> Direction {
        match self {
            Direction::Down => Direction::Up,
            Direction::Up => Direction::Down,
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::West => Direction::East,
            Direction::East => Direction::West,
        }
    }

    /// A quarter turn clockwise seen from above (Minecraft: `getClockWise`); up and down stay as they are.
    pub fn clockwise(self) -> Direction {
        match self {
            Direction::North => Direction::East,
            Direction::East => Direction::South,
            Direction::South => Direction::West,
            Direction::West => Direction::North,
            vertical => vertical,
        }
    }

    /// The six directions ordered by how closely a player turned `yaw` degrees and looking `pitch` degrees (negative looks up) faces them,
    /// nearest first (Minecraft: `Direction.orderedByNearest`).
    pub fn nearest_looking(yaw: f32, pitch: f32) -> [Direction; 6] {
        let pitch = pitch * (std::f32::consts::PI / 180.0);
        let yaw = -yaw * (std::f32::consts::PI / 180.0);
        let (sin_pitch, cos_pitch, sin_yaw, cos_yaw) = (pitch.sin(), pitch.cos(), yaw.sin(), yaw.cos());
        let east = sin_yaw > 0.0;
        let up = sin_pitch < 0.0;
        let south = cos_yaw > 0.0;
        let x = if east { sin_yaw } else { -sin_yaw };
        let y = if up { -sin_pitch } else { sin_pitch };
        let z = if south { cos_yaw } else { -cos_yaw };
        let (x_cos, z_cos) = (x * cos_pitch, z * cos_pitch);
        let (dx, dy, dz) = (
            if east { Direction::East } else { Direction::West },
            if up { Direction::Up } else { Direction::Down },
            if south { Direction::South } else { Direction::North },
        );
        let array = |a: Direction, b: Direction, c: Direction| [a, b, c, c.opposite(), b.opposite(), a.opposite()];
        if x > z {
            if y > x_cos {
                array(dy, dx, dz)
            } else if z_cos > y {
                array(dx, dz, dy)
            } else {
                array(dx, dy, dz)
            }
        } else if y > z_cos {
            array(dy, dz, dx)
        } else if x_cos > y {
            array(dz, dx, dy)
        } else {
            array(dz, dy, dx)
        }
    }

    /// The position one block from `pos` in this direction.
    pub fn relative(self, pos: BlockPos) -> BlockPos {
        let [x, y, z] = self.offset();
        [pos[0] + x, pos[1] + y, pos[2] + z]
    }
}

/// The blocks around a position, as far as the game lets a behaviour look.
pub trait BlockView {
    /// The state at `pos` with its properties; None where the game cannot say (not loaded).
    fn block_state(&self, pos: BlockPos) -> Option<BlockState>;
    /// Whether the block at `pos` is air.
    fn is_air(&self, pos: BlockPos) -> bool;
    /// Whether the whole `face` of the block at `pos` is solid (Minecraft: `isFaceSturdy`), something a block can be fixed to.
    fn is_face_sturdy(&self, pos: BlockPos, face: Direction) -> bool;
    /// Whether the block at `pos` is in the block tag `tag`, such as `tconstruct:slimy_leaves`.
    fn is_in_tag(&self, pos: BlockPos, tag: &str) -> bool;
    /// The light at `pos` from the sky and from blocks, 0 to 15 (Minecraft: `getMaxLocalRawBrightness`).
    fn brightness(&self, pos: BlockPos) -> u8;
    /// Whether light passes from the block at `pos` into the block above it, which is what lets grass live under a cover.
    fn light_passes_up(&self, pos: BlockPos) -> bool;
    /// Whether the block at `pos` holds water: water itself or a waterlogged block.
    fn is_water(&self, pos: BlockPos) -> bool;
    /// Whether the fluid at `pos` is a full source block of fluid (Minecraft: `getFluidState().getAmount() == 8`).
    fn is_fluid_full(&self, pos: BlockPos) -> bool;
    /// Whether the chunks within `radius` blocks of `pos` are loaded (Minecraft: `isAreaLoaded`); tick callbacks check it so that they never
    /// load a chunk by touching its edge.
    fn is_area_loaded(&self, pos: BlockPos, radius: i32) -> bool;
    /// Whether the collision shape of the block at `pos` is a whole block (Minecraft: `isCollisionShapeFullBlock`).
    fn is_full_block(&self, pos: BlockPos) -> bool;
    /// The lowest y blocks can be built at and the first y above the build limit (Minecraft: `getMinBuildHeight`, `getMaxBuildHeight`).
    fn build_height(&self) -> [i32; 2];
}

/// A [`BlockView`] for tests: the blocks set are sturdy on every face, everything else is air.
#[derive(Default, Debug, Clone)]
pub struct MemoryBlockView {
    /// The blocks set so far.
    pub blocks: BTreeMap<BlockPos, BlockState>,
    /// Successful writes in order: position, state and exact update flags. This records
    /// requests only; MemoryBlockView does not simulate neighbour/client updates.
    pub writes: Vec<(BlockPos, BlockState, i32)>,
    /// Positions that hold a block but not a sturdy one (a slab's top, glass).
    pub weak: BTreeSet<BlockPos>,
    /// The tags each position's block is in.
    pub tags: BTreeMap<BlockPos, BTreeSet<String>>,
    /// The light of each position; positions not listed have 15 (open sky).
    pub light: BTreeMap<BlockPos, u8>,
    /// Positions that hold a full source of water.
    pub water: BTreeSet<BlockPos>,
    /// Positions that are not loaded (the rest are).
    pub unloaded: BTreeSet<BlockPos>,
    /// The numbers [`BlockEditor::random_float`] hands out, first first; 0.0 once they are used up.
    pub floats: Vec<f32>,
    /// The numbers [`BlockEditor::random_int`] hands out, first first (each is taken modulo the bound); 0 once they are used up.
    pub rolls: Vec<i32>,
    /// The features placed so far: id and position.
    pub features: Vec<(String, BlockPos)>,
    /// The ticks scheduled so far: position, block and delay.
    pub scheduled: Vec<(BlockPos, String, i32)>,
}

impl BlockView for MemoryBlockView {
    fn block_state(&self, pos: BlockPos) -> Option<BlockState> {
        Some(self.blocks.get(&pos).cloned().unwrap_or_else(|| BlockState::parse("minecraft:air").expect("a fixed id")))
    }

    fn is_air(&self, pos: BlockPos) -> bool {
        !self.blocks.contains_key(&pos)
    }

    fn is_face_sturdy(&self, pos: BlockPos, _face: Direction) -> bool {
        self.blocks.contains_key(&pos) && !self.weak.contains(&pos)
    }

    fn is_in_tag(&self, pos: BlockPos, tag: &str) -> bool {
        self.tags.get(&pos).is_some_and(|tags| tags.contains(tag))
    }

    fn brightness(&self, pos: BlockPos) -> u8 {
        self.light.get(&pos).copied().unwrap_or(15)
    }

    fn light_passes_up(&self, pos: BlockPos) -> bool {
        let above = [pos[0], pos[1] + 1, pos[2]];
        !self.blocks.contains_key(&above) || self.weak.contains(&above)
    }

    fn is_water(&self, pos: BlockPos) -> bool {
        self.water.contains(&pos)
    }

    fn is_fluid_full(&self, pos: BlockPos) -> bool {
        self.water.contains(&pos)
    }

    fn is_area_loaded(&self, pos: BlockPos, radius: i32) -> bool {
        !self.unloaded.iter().any(|u| (u[0] - pos[0]).abs() <= radius && (u[2] - pos[2]).abs() <= radius)
    }

    fn is_full_block(&self, pos: BlockPos) -> bool {
        self.blocks.contains_key(&pos) && !self.weak.contains(&pos)
    }

    fn build_height(&self) -> [i32; 2] {
        [-64, 320]
    }
}

/// What a callback may change: a [`BlockView`] that can also write blocks, roll dice and schedule ticks (Minecraft: `ServerLevel`).
pub trait BlockEditor: BlockView {
    /// Sets the block at `pos` to `state` (Minecraft: `setBlock` with the flags that update neighbours and clients); false if it could
    /// not be set (not loaded, unknown block).
    fn set_block_state(&mut self, pos: BlockPos, state: &BlockState) -> bool;
    /// Sets a block with explicit Minecraft update flags. The live adapter forwards
    /// flags unchanged (1 neighbours, 2 clients, 16 known shape). Existing custom
    /// editors support only flags=3 by default; others fail without writing rather
    /// than silently changing update semantics. MemoryBlockView records the flags.
    fn set_block_state_with_flags(&mut self, pos: BlockPos, state: &BlockState, flags: i32) -> bool {
        flags == 3 && self.set_block_state(pos, state)
    }
    /// Replaces the block at `pos` with air.
    fn remove_block(&mut self, pos: BlockPos) -> bool {
        match BlockState::parse("minecraft:air") {
            Ok(air) => self.set_block_state(pos, &air),
            Err(_) => false,
        }
    }
    /// A random whole number from 0 up to but not including `bound` (Minecraft: `RandomSource.nextInt`), drawn from the level's own random
    /// source so the order of draws is the game's.
    fn random_int(&mut self, bound: i32) -> i32;
    /// A number from 0.0 up to but not including 1.0 from the level's own random source (Minecraft: `RandomSource.nextFloat`).
    fn random_float(&mut self) -> f32;
    /// Asks the game to tick `block` at `pos` after `delay` game ticks (Minecraft: `scheduleTick`); false if it could not be scheduled.
    fn schedule_tick(&mut self, pos: BlockPos, block: &str, delay: i32) -> bool;
    /// Places the configured feature `feature` at `pos` (Minecraft: `ConfiguredFeature.place`): a worldgen feature declared in data, such as
    /// `tconstruct:slime_fungus/blood_earth_slime_grass`. False if the feature is unknown or found no room.
    fn place_feature(&mut self, feature: &str, pos: BlockPos) -> bool;
}

impl BlockEditor for MemoryBlockView {
    fn set_block_state(&mut self, pos: BlockPos, state: &BlockState) -> bool {
        self.set_block_state_with_flags(pos, state, 3)
    }

    fn set_block_state_with_flags(&mut self, pos: BlockPos, state: &BlockState, flags: i32) -> bool {
        self.writes.push((pos, state.clone(), flags));
        if state.block.to_string() == "minecraft:air" {
            self.blocks.remove(&pos);
        } else {
            self.blocks.insert(pos, state.clone());
        }
        true
    }

    fn random_int(&mut self, bound: i32) -> i32 {
        if self.rolls.is_empty() || bound <= 0 {
            0
        } else {
            self.rolls.remove(0).rem_euclid(bound)
        }
    }

    fn random_float(&mut self) -> f32 {
        if self.floats.is_empty() {
            0.0
        } else {
            self.floats.remove(0)
        }
    }

    fn schedule_tick(&mut self, pos: BlockPos, block: &str, delay: i32) -> bool {
        self.scheduled.push((pos, block.to_owned(), delay));
        true
    }

    fn place_feature(&mut self, feature: &str, pos: BlockPos) -> bool {
        self.features.push((feature.to_owned(), pos));
        true
    }
}

/// A tick or a bone meal reached the block: it may read and change the world around it.
pub struct TickContext<'a> {
    /// The block's state.
    pub state: BlockState,
    /// The position.
    pub pos: BlockPos,
    /// The world around, which can be changed.
    pub world: &'a mut dyn BlockEditor,
}

/// Where a plant is asked what it may stand on (Minecraft: `mayPlaceOn`).
pub struct PlaceOnContext<'a> {
    /// The plant's state.
    pub state: BlockState,
    /// The ground block's state.
    pub ground: BlockState,
    /// The ground block's position.
    pub ground_pos: BlockPos,
    /// The blocks around.
    pub view: &'a dyn BlockView,
}

/// Where a bone meal is about to be used (Minecraft: `isValidBonemealTarget`).
pub struct BonemealContext<'a> {
    /// The block's state.
    pub state: BlockState,
    /// The position.
    pub pos: BlockPos,
    /// The blocks around.
    pub view: &'a dyn BlockView,
}

/// Where a block is asked whether it may stay (Minecraft: `canSurvive`).
pub struct SurviveContext<'a> {
    /// The block's state, which may not be in the world yet.
    pub state: BlockState,
    /// The position.
    pub pos: BlockPos,
    /// The blocks around.
    pub view: &'a dyn BlockView,
}

/// What a placement is told (Minecraft: `BlockPlaceContext`).
pub struct PlacementContext<'a> {
    /// The block's default state.
    pub state: BlockState,
    /// Where the block would go.
    pub pos: BlockPos,
    /// The face that was clicked.
    pub face: Direction,
    /// The placer's turn in degrees (0 faces south, 90 west).
    pub yaw: f32,
    /// The placer's look up or down in degrees (negative looks up).
    pub pitch: f32,
    /// Whether the placer sneaks.
    pub sneaking: bool,
    /// Whether the position holds water now.
    pub in_water: bool,
    /// The item in the placer's hand, such as `tconstruct:punji`.
    pub item: String,
    /// Whether the clicked block is replaced (grass, a snow layer), so the block goes into the clicked position itself.
    pub replace_clicked: bool,
    /// The blocks around.
    pub view: &'a dyn BlockView,
}

impl PlacementContext<'_> {
    /// The directions in the order the placer looks at them, nearest first, with the side opposite the clicked face moved to the front
    /// unless the clicked block is replaced (Minecraft: `getNearestLookingDirections`).
    pub fn nearest_looking_directions(&self) -> [Direction; 6] {
        let mut nearest = Direction::nearest_looking(self.yaw, self.pitch);
        if !self.replace_clicked {
            let wanted = self.face.opposite();
            if let Some(index) = nearest.iter().position(|d| *d == wanted) {
                nearest.copy_within(0..index, 1);
                nearest[0] = wanted;
            }
        }
        nearest
    }
}

/// What a placement becomes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Placement {
    /// The game's own placement runs.
    Default,
    /// Nothing is placed and the item is kept (Minecraft: `getStateForPlacement` returning null).
    Refuse,
    /// The block is placed in this state.
    State(BlockState),
}

/// A neighbour changed (Minecraft: `updateShape`).
pub struct ShapeUpdateContext<'a> {
    /// The block's state, which may not be in the world yet.
    pub state: BlockState,
    /// The position.
    pub pos: BlockPos,
    /// Which side the change came from.
    pub direction: Direction,
    /// The neighbour's position.
    pub neighbour_pos: BlockPos,
    /// The neighbour's state, if the game can say.
    pub neighbour: Option<BlockState>,
    /// The blocks around.
    pub view: &'a dyn BlockView,
}

/// An entity is inside the block (Minecraft: `entityInside`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntityInsideContext {
    /// The dimension, such as `minecraft:overworld`.
    pub dimension: String,
    /// The block's state.
    pub state: BlockState,
    /// The position.
    pub pos: BlockPos,
    /// The entity's id.
    pub entity: i32,
    /// The entity's type, such as `minecraft:zombie`.
    pub kind: String,
}

type SurviveFn = Arc<dyn Fn(&SurviveContext<'_>) -> bool + Send + Sync>;
type PlacementFn = Arc<dyn Fn(&PlacementContext<'_>) -> Placement + Send + Sync>;
type ShapeFn = Arc<dyn Fn(&ShapeUpdateContext<'_>) -> Option<BlockState> + Send + Sync>;
type InsideFn = Arc<dyn Fn(&EntityInsideContext) + Send + Sync>;
/// A player picks the block (middle click in creative; Minecraft: `getCloneItemStack`).
pub struct PickContext {
    /// The block's state.
    pub state: BlockState,
    /// The position.
    pub pos: BlockPos,
}

type PickFn = Arc<dyn Fn(&PickContext) -> Option<ItemStack> + Send + Sync>;
type TickFn = Arc<dyn Fn(&mut TickContext<'_>) + Send + Sync>;
type BonemealCheckFn = Arc<dyn Fn(&BonemealContext<'_>) -> bool + Send + Sync>;
type BonemealSuccessFn = Arc<dyn Fn(&mut TickContext<'_>) -> bool + Send + Sync>;
type PlaceOnFn = Arc<dyn Fn(&PlaceOnContext<'_>) -> bool + Send + Sync>;

/// What one block of the mod does. Build it, then give it to [`BlockBehaviors::register`].
#[derive(Clone)]
pub struct BlockBehavior {
    block: Identifier,
    can_survive: Option<SurviveFn>,
    state_for_placement: Option<PlacementFn>,
    update_shape: Option<ShapeFn>,
    entity_inside: Option<InsideFn>,
    random_tick: Option<TickFn>,
    scheduled_tick: Option<TickFn>,
    can_bonemeal: Option<BonemealCheckFn>,
    bonemeal_success: Option<BonemealSuccessFn>,
    on_bonemeal: Option<TickFn>,
    may_place_on: Option<PlaceOnFn>,
    pick_stack: Option<PickFn>,
}

impl BlockBehavior {
    /// A behaviour for `block` that changes nothing yet.
    pub fn new(block: Identifier) -> Self {
        Self { block, can_survive: None, state_for_placement: None, update_shape: None, entity_inside: None, random_tick: None, scheduled_tick: None, can_bonemeal: None, bonemeal_success: None, on_bonemeal: None, may_place_on: None, pick_stack: None }
    }

    /// Where the block may stay: answer `false` and it is refused when placed and breaks (with its drops) when a neighbour changes.
    pub fn can_survive(mut self, f: impl Fn(&SurviveContext<'_>) -> bool + Send + Sync + 'static) -> Self {
        self.can_survive = Some(Arc::new(f));
        self
    }

    /// The state the block takes when placed.
    pub fn state_for_placement(mut self, f: impl Fn(&PlacementContext<'_>) -> Placement + Send + Sync + 'static) -> Self {
        self.state_for_placement = Some(Arc::new(f));
        self
    }

    /// How a neighbour's change turns the block's state: `Some(state)` becomes the block's state (the same state too, which stops the game's own
    /// handling of that update), `None` leaves the update to the game. The game asks often
    /// (at least six times for each block set), so keep it short.
    pub fn update_shape(mut self, f: impl Fn(&ShapeUpdateContext<'_>) -> Option<BlockState> + Send + Sync + 'static) -> Self {
        self.update_shape = Some(Arc::new(f));
        self
    }

    /// An entity is inside the block's space.
    pub fn entity_inside(mut self, f: impl Fn(&EntityInsideContext) + Send + Sync + 'static) -> Self {
        self.entity_inside = Some(Arc::new(f));
        self
    }

    /// A random tick reached the block (Minecraft: `randomTick`): the game picks blocks at random, a few in every chunk section each tick.
    /// Having this callback is what makes the game's own random tick of the block not run, so do what the block needs.
    pub fn random_tick(mut self, f: impl Fn(&mut TickContext<'_>) + Send + Sync + 'static) -> Self {
        self.random_tick = Some(Arc::new(f));
        self
    }

    /// A tick the block scheduled came due (Minecraft: `tick`); schedule one with [`BlockEditor::schedule_tick`]. Having this callback is
    /// what makes the game's own tick of the block not run.
    pub fn scheduled_tick(mut self, f: impl Fn(&mut TickContext<'_>) + Send + Sync + 'static) -> Self {
        self.scheduled_tick = Some(Arc::new(f));
        self
    }

    /// Whether a bone meal can be used on the block (Minecraft: `isValidBonemealTarget`); a used meal is spent only if this says yes.
    pub fn can_bonemeal(mut self, f: impl Fn(&BonemealContext<'_>) -> bool + Send + Sync + 'static) -> Self {
        self.can_bonemeal = Some(Arc::new(f));
        self
    }

    /// Whether a bone meal that was used makes the block grow this time (Minecraft: `isBonemealSuccess`); roll with
    /// [`BlockEditor::random_int`]. Without it the game's own chance applies.
    pub fn bonemeal_success(mut self, f: impl Fn(&mut TickContext<'_>) -> bool + Send + Sync + 'static) -> Self {
        self.bonemeal_success = Some(Arc::new(f));
        self
    }

    /// The block grows from a bone meal (Minecraft: `performBonemeal`).
    pub fn on_bonemeal(mut self, f: impl Fn(&mut TickContext<'_>) + Send + Sync + 'static) -> Self {
        self.on_bonemeal = Some(Arc::new(f));
        self
    }

    /// What a plant may stand on (Minecraft: `mayPlaceOn`): asked for the plant with the ground below it. The block's template has to be a
    /// plant that asks, such as a crop or a sapling.
    pub fn may_place_on(mut self, f: impl Fn(&PlaceOnContext<'_>) -> bool + Send + Sync + 'static) -> Self {
        self.may_place_on = Some(Arc::new(f));
        self
    }

    /// The stack a player gets when picking the block (middle click in creative; Minecraft: `getCloneItemStack`, Fabric: `Block.getPickStack`):
    /// answer `Some(stack)`, with data components if you like, or `None` for the game's own choice.
    pub fn pick_stack(mut self, f: impl Fn(&PickContext) -> Option<ItemStack> + Send + Sync + 'static) -> Self {
        self.pick_stack = Some(Arc::new(f));
        self
    }

    /// The block this behaviour belongs to.
    pub fn block(&self) -> &Identifier {
        &self.block
    }

}

static BEHAVIORS: Mutex<Vec<BlockBehavior>> = Mutex::new(Vec::new());

/// The registry of block behaviours.
pub struct BlockBehaviors;

impl BlockBehaviors {
    /// Gives `behavior.block()` its behaviour. Registering the same block again replaces the earlier behaviour.
    pub fn register(behavior: BlockBehavior) {
        if let Ok(mut all) = BEHAVIORS.lock() {
            all.retain(|b| b.block != behavior.block);
            all.push(behavior);
        }
    }

    /// The behaviour registered for `block` (an id such as `tconstruct:punji`), if any.
    pub fn get(block: &str) -> Option<BlockBehavior> {
        BEHAVIORS.lock().ok()?.iter().find(|b| b.block.to_string() == block).cloned()
    }

    /// Every block that has a behaviour.
    pub fn registered() -> Vec<BlockBehavior> {
        BEHAVIORS.lock().map(|all| all.clone()).unwrap_or_default()
    }
}

pub(crate) fn survives(block: &str, context: &SurviveContext<'_>) -> Option<bool> {
    BlockBehaviors::get(block)?.can_survive.map(|f| f(context))
}

pub(crate) fn placement(block: &str, context: &PlacementContext<'_>) -> Placement {
    BlockBehaviors::get(block).and_then(|b| b.state_for_placement).map_or(Placement::Default, |f| f(context))
}

pub(crate) fn shape_update(block: &str, context: &ShapeUpdateContext<'_>) -> Option<BlockState> {
    BlockBehaviors::get(block)?.update_shape.and_then(|f| f(context))
}

pub(crate) fn random_tick(block: &str, context: &mut TickContext<'_>) -> bool {
    match BlockBehaviors::get(block).and_then(|b| b.random_tick) {
        Some(f) => {
            f(context);
            true
        }
        None => false,
    }
}

pub(crate) fn scheduled_tick(block: &str, context: &mut TickContext<'_>) -> bool {
    match BlockBehaviors::get(block).and_then(|b| b.scheduled_tick) {
        Some(f) => {
            f(context);
            true
        }
        None => false,
    }
}

pub(crate) fn can_bonemeal(block: &str, context: &BonemealContext<'_>) -> Option<bool> {
    BlockBehaviors::get(block)?.can_bonemeal.map(|f| f(context))
}

pub(crate) fn bonemeal_success(block: &str, context: &mut TickContext<'_>) -> Option<bool> {
    BlockBehaviors::get(block)?.bonemeal_success.map(|f| f(context))
}

pub(crate) fn on_bonemeal(block: &str, context: &mut TickContext<'_>) {
    if let Some(f) = BlockBehaviors::get(block).and_then(|b| b.on_bonemeal) {
        f(context);
    }
}

pub(crate) fn may_place_on(block: &str, context: &PlaceOnContext<'_>) -> Option<bool> {
    BlockBehaviors::get(block)?.may_place_on.map(|f| f(context))
}

pub(crate) fn pick(block: &str, context: &PickContext) -> Option<ItemStack> {
    BlockBehaviors::get(block)?.pick_stack.and_then(|f| f(context))
}

pub(crate) fn inside(block: &str, context: &EntityInsideContext) {
    if let Some(f) = BlockBehaviors::get(block).and_then(|b| b.entity_inside) {
        f(context);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(text: &str) -> Identifier {
        Identifier::parse(text).unwrap()
    }

    #[test]
    fn explicit_write_flags_are_recorded_and_legacy_editors_reject_unsupported_flags() {
        struct Legacy { writes: usize }
        impl BlockView for Legacy {
            fn block_state(&self, _: BlockPos) -> Option<BlockState> { unreachable!() }
            fn is_air(&self, _: BlockPos) -> bool { unreachable!() }
            fn is_face_sturdy(&self, _: BlockPos, _: Direction) -> bool { unreachable!() }
            fn is_in_tag(&self, _: BlockPos, _: &str) -> bool { unreachable!() }
            fn brightness(&self, _: BlockPos) -> u8 { unreachable!() }
            fn light_passes_up(&self, _: BlockPos) -> bool { unreachable!() }
            fn is_water(&self, _: BlockPos) -> bool { unreachable!() }
            fn is_fluid_full(&self, _: BlockPos) -> bool { unreachable!() }
            fn is_area_loaded(&self, _: BlockPos, _: i32) -> bool { unreachable!() }
            fn is_full_block(&self, _: BlockPos) -> bool { unreachable!() }
            fn build_height(&self) -> [i32; 2] { unreachable!() }
        }
        impl BlockEditor for Legacy {
            fn set_block_state(&mut self, _: BlockPos, _: &BlockState) -> bool { self.writes += 1; true }
            fn random_int(&mut self, _: i32) -> i32 { unreachable!() }
            fn random_float(&mut self) -> f32 { unreachable!() }
            fn schedule_tick(&mut self, _: BlockPos, _: &str, _: i32) -> bool { unreachable!() }
            fn place_feature(&mut self, _: &str, _: BlockPos) -> bool { unreachable!() }
        }
        let state = BlockState::parse("minecraft:oak_log[axis=x]").unwrap();
        let pos = [-4, 70, 9];
        let mut legacy = Legacy { writes: 0 };
        assert!(!legacy.set_block_state_with_flags(pos, &state, 18));
        assert_eq!(legacy.writes, 0, "never silently substitute default flags");
        assert!(legacy.set_block_state_with_flags(pos, &state, 3));
        assert_eq!(legacy.writes, 1);
        let mut memory = MemoryBlockView::default();
        assert!(memory.set_block_state(pos, &state));
        assert!(memory.set_block_state_with_flags(pos, &state, 18));
        assert!(memory.set_block_state_with_flags(pos, &state, 19));
        assert_eq!(memory.writes, vec![(pos, state.clone(), 3), (pos, state.clone(), 18), (pos, state, 19)]);
    }

    #[test]
    fn a_hanging_block_survives_only_under_a_solid_block_and_breaks_when_that_goes() {
        BlockBehaviors::register(
            BlockBehavior::new(id("example:lantern_hook")).can_survive(|c| c.view.is_face_sturdy(Direction::Up.relative(c.pos), Direction::Down)),
        );
        let mut view = MemoryBlockView::default();
        let ctx = |view: &MemoryBlockView| survives("example:lantern_hook", &SurviveContext { state: BlockState::parse("example:lantern_hook").unwrap(), pos: [0, 64, 0], view });
        assert_eq!(ctx(&view), Some(false), "nothing above");
        view.blocks.insert([0, 65, 0], BlockState::parse("minecraft:stone").unwrap());
        assert_eq!(ctx(&view), Some(true));
        view.weak.insert([0, 65, 0]);
        assert_eq!(ctx(&view), Some(false), "a weak block cannot hold it");
        assert_eq!(survives("example:other", &SurviveContext { state: BlockState::parse("example:other").unwrap(), pos: [0, 0, 0], view: &view }), None);
    }

    #[test]
    fn a_random_tick_grows_a_shoot_when_the_dice_say_so_and_leaves_the_world_alone_otherwise() {
        BlockBehaviors::register(BlockBehavior::new(id("example:shoot")).random_tick(|c| {
            if c.world.random_int(4) == 0 {
                c.world.set_block_state([c.pos[0], c.pos[1] + 1, c.pos[2]], &c.state.clone());
            }
        }));
        let state = BlockState::parse("example:shoot").unwrap();
        let mut world = MemoryBlockView { rolls: vec![3, 8], ..Default::default() };
        assert!(random_tick("example:shoot", &mut TickContext { state: state.clone(), pos: [0, 64, 0], world: &mut world }));
        assert!(world.blocks.is_empty(), "a 3 is not a 0");
        assert!(random_tick("example:shoot", &mut TickContext { state, pos: [0, 64, 0], world: &mut world }));
        assert_eq!(world.blocks.len(), 1, "8 modulo 4 is 0: the shoot grew upwards");
        assert!(!random_tick("example:rock", &mut TickContext { state: BlockState::parse("example:rock").unwrap(), pos: [0, 0, 0], world: &mut world }));
    }

    #[test]
    fn looking_order_follows_where_the_placer_faces_and_puts_the_clicked_side_first() {
        // facing south (yaw 0), level: south first, then down/up in some order, north last
        let level = Direction::nearest_looking(0.0, 0.0);
        assert_eq!(level[0], Direction::South);
        assert_eq!(level[5], Direction::North);
        // looking straight up
        assert_eq!(Direction::nearest_looking(0.0, -90.0)[0], Direction::Up);
        // facing west (yaw 90)
        assert_eq!(Direction::nearest_looking(90.0, 0.0)[0], Direction::West);
        let view = MemoryBlockView::default();
        let context = |replace_clicked| PlacementContext {
            state: BlockState::parse("example:vine").unwrap(),
            pos: [0, 64, 0],
            face: Direction::North,
            yaw: 0.0,
            pitch: 0.0,
            sneaking: false,
            in_water: false,
            item: String::new(),
            replace_clicked,
            view: &view,
        };
        assert_eq!(context(true).nearest_looking_directions()[0], Direction::South);
        // the clicked face was north, so the side opposite it (south) leads; it already did here, so look from the other side
        let mut other = context(false);
        other.face = Direction::East;
        assert_eq!(other.nearest_looking_directions()[0], Direction::West);
        assert_eq!(other.nearest_looking_directions()[1], Direction::South);
    }

    #[test]
    fn directions_turn_and_step_as_the_game_does() {
        assert_eq!(Direction::North.clockwise(), Direction::East);
        assert_eq!(Direction::Up.clockwise(), Direction::Up);
        assert_eq!(Direction::South.relative([1, 1, 1]), [1, 1, 2]);
        assert_eq!(Direction::from_offset(Direction::West.offset()), Some(Direction::West));
        assert!(Direction::ALL.iter().all(|d| d.opposite().opposite() == *d && Direction::parse(d.name()) == Some(*d)));
    }
}
