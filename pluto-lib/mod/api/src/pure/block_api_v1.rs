//! What a mod's own blocks do (Minecraft: `Block.canSurvive`, `getStateForPlacement`, `updateShape`, `entityInside`).
//!
//! A [`BlockBehavior`] belongs to one block of the mod and answers four questions the game asks about it: where it may stand, which state it
//! takes when placed, how a neighbour changes it, and what happens to an entity inside it. The block itself (its template, its state
//! properties, its shapes) is declared in the mod's content files; the behaviour is what those files cannot say.
//!
//! The callbacks see states as [`BlockState`] text and the surroundings through a [`BlockView`]. PlutoLib owns the game's hook for each
//! question once, so two mods never compete for it: each behaviour is asked only about its own block.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use super::base::Identifier;
use super::entity_events_v1::EntityAction;
use super::block_entity_api_v1::BlockPos;
use super::item_group_api_v1::ItemStack;
use super::world_api_v1::{BlockState, PlacementFacts, SurvivalFacts};

#[path = "block_path.rs"]
mod path_nodes;
pub use path_nodes::{PathNodeType, PathNodeTypeContext};

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
    /// Native state replaceability, not a collision/air approximation. Custom
    /// views default to air only; implementations can expose other states.
    fn is_replaceable(&self, pos: BlockPos) -> bool { self.is_air(pos) }
    /// The active chunk generator's vertical depth, not the build ceiling.
    /// None when the callback has no generator context (including live views).
    fn generation_depth(&self) -> Option<i32> { None }
    /// Minecraft Level.hasNeighborSignal, not this block's powered property.
    /// None when the native query is unavailable; do not interpret it as false.
    fn has_neighbor_signal(&self, _pos: BlockPos) -> Option<bool> { None }
    /// Whether the whole `face` of the block at `pos` is solid (Minecraft: `isFaceSturdy`), something a block can be fixed to.
    fn is_face_sturdy(&self, pos: BlockPos, face: Direction) -> bool;
    /// The same, when the view can say it: None where the view has no sturdiness facts for the block at `pos`
    /// (a block the game's table does not name), rather than guessing it is not sturdy. Defaults to
    /// `is_face_sturdy`, which is the same answer where the view already knows every block.
    fn face_sturdy(&self, pos: BlockPos, face: Direction) -> Option<bool> { Some(self.is_face_sturdy(pos, face)) }
    /// Whether the block at `pos` is in the block tag `tag`, such as `tconstruct:slimy_leaves`.
    fn is_in_tag(&self, pos: BlockPos, tag: &str) -> bool;
    /// Registry tag membership of a supplied state, even before it is in the world.
    /// None when this view has no registry, rather than querying another position.
    fn is_state_in_tag(&self, _state: &BlockState, _tag: &str) -> Option<bool> { None }
    /// The game's own placement facts for a supplied state, even before it is in the world:
    /// its class chain and shape flags, as the block table reports them. None when this view
    /// has no block table (a memory view that was not given them).
    fn placement_facts(&self, _state: &BlockState) -> Option<super::world_api_v1::PlacementFacts> { None }
    /// The game's own survival facts for a supplied state: the solidity, support, shape,
    /// fire and fluid facts `canSurvive`'s arms read, as the block table reports them.
    /// None when this view has no block table (a memory view that was not given them).
    fn survival_facts(&self, _state: &BlockState) -> Option<super::world_api_v1::SurvivalFacts> { None }
    /// The light at `pos` from the sky and from blocks, 0 to 15 (Minecraft: `getMaxLocalRawBrightness`).
    fn brightness(&self, pos: BlockPos) -> u8;
    /// The light at `pos` with no sky darkening applied (Minecraft: `getRawBrightness(pos, 0)`): the
    /// brighter of the sky and block light, as `canSurvive` reads it for crops and mushrooms. None
    /// where the view cannot read the light at all. Defaults to `brightness`, which is the same
    /// value where the view already darkens nothing.
    fn raw_brightness(&self, pos: BlockPos) -> Option<u8> { Some(self.brightness(pos)) }
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
    /// Destruction requests. Memory removes blocks but does not simulate loot,
    /// particles, neighbour updates or replacement by the contained fluid.
    pub break_requests: Vec<(BlockPos, bool)>,
    /// Positions that hold a block but not a sturdy one (a slab's top, glass).
    pub weak: BTreeSet<BlockPos>,
    /// Non-air positions explicitly marked replaceable; no tag inference.
    pub replaceable: BTreeSet<BlockPos>,
    /// An optional generator depth, independent of the build limits.
    pub generator_depth: Option<i32>,
    /// Explicit neighbor-signal answers; absent/unloaded means unavailable,
    /// not unpowered. Memory does not simulate redstone propagation.
    pub neighbor_signals: BTreeMap<BlockPos, bool>,
    /// The tags each position's block is in.
    pub tags: BTreeMap<BlockPos, BTreeSet<String>>,
    /// Explicit registry tags by block id, independent of placed positions.
    pub state_tags: BTreeMap<String, BTreeSet<String>>,
    /// Explicit placement facts by block id (the class chain and shape flags), for
    /// `can_survive` to read without a game table.
    pub placement_facts: BTreeMap<String, PlacementFacts>,
    /// Explicit survival facts by block id (the solidity, support, shape, fire and fluid
    /// facts), for `can_survive` to read without a game table.
    pub survival_facts: BTreeMap<String, SurvivalFacts>,
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

    fn is_replaceable(&self, pos: BlockPos) -> bool {
        self.is_air(pos) || self.replaceable.contains(&pos)
    }

    fn generation_depth(&self) -> Option<i32> { self.generator_depth }
    fn has_neighbor_signal(&self, pos: BlockPos) -> Option<bool> {
        if self.unloaded.contains(&pos) { None } else { self.neighbor_signals.get(&pos).copied() }
    }

    fn is_face_sturdy(&self, pos: BlockPos, _face: Direction) -> bool {
        self.blocks.contains_key(&pos) && !self.weak.contains(&pos)
    }

    fn is_in_tag(&self, pos: BlockPos, tag: &str) -> bool {
        self.tags.get(&pos).is_some_and(|tags| tags.contains(tag))
    }
    fn is_state_in_tag(&self, state: &BlockState, tag: &str) -> Option<bool> {
        self.state_tags.get(&state.block.to_string()).map(|tags| tags.contains(tag))
    }
    fn placement_facts(&self, state: &BlockState) -> Option<PlacementFacts> {
        self.placement_facts.get(&state.block.to_string()).cloned()
    }
    fn survival_facts(&self, state: &BlockState) -> Option<SurvivalFacts> {
        self.survival_facts.get(&state.block.to_string()).cloned()
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
    /// Breaks a block using native particles, optional loot and fluid replacement
    /// (Minecraft: destroyBlock / Fabric: breakBlock). None means unsupported,
    /// Some(false) means supported but no change (for example already air).
    /// Custom editors default to no writes. Memory worlds record requests only.
    fn break_block(&mut self, _pos: BlockPos, _drop: bool) -> Option<bool> { None }
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
    /// Place a native tree configuration, then explicit extra decorators before
    /// native leaf-distance/edge-shape finalization. Default/Memory editors fail
    /// without writing. Live callback worlds reject nested features/trees until
    /// their writes can be tracked in the outer tree. No natural dispatch implied.
    fn place_tree(&mut self, _configuration: &serde_json::Value, _pos: BlockPos, _decorators: &[super::worldgen_api_v1::TreeDecorator]) -> bool { false }
    /// Native base-root simulation with a bound per-position root override. All
    /// holder/provider descriptors bind before tree writes. Unsupported editors
    /// fail without writing or invoking callbacks; ordinary native JSON is unchanged.
    fn place_tree_with_roots(&mut self, _configuration: &serde_json::Value, _pos: BlockPos, _roots: &super::worldgen_api_v1::RootPlacement, _decorators: &[super::worldgen_api_v1::TreeDecorator]) -> bool { false }
    /// Execute explicit native/custom decorator steps before finalization. Each
    /// native decorator must appear once; invalid plans fail before tree writes.
    /// Optional root overrides preserve the native simulator. Default is no-write.
    fn place_tree_with_plan(&mut self, _configuration: &serde_json::Value, _pos: BlockPos, _roots: Option<&super::worldgen_api_v1::RootPlacement>, _plan: &[super::worldgen_api_v1::TreeDecoration]) -> bool { false }
}

impl BlockEditor for MemoryBlockView {
    fn break_block(&mut self, pos: BlockPos, drop: bool) -> Option<bool> {
        self.break_requests.push((pos, drop));
        let changed = self.blocks.remove(&pos).is_some();
        self.tags.remove(&pos);
        self.weak.remove(&pos);
        self.replaceable.remove(&pos);
        Some(changed)
    }

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
    tick_delay: std::cell::Cell<Option<i32>>,
}
impl<'a> ShapeUpdateContext<'a> {
    /// Construct one update with no scheduled-tick request.
    pub fn new(state: BlockState, pos: BlockPos, direction: Direction,
        neighbour_pos: BlockPos, neighbour: Option<BlockState>, view: &'a dyn BlockView) -> Self {
        Self { state, pos, direction, neighbour_pos, neighbour, view, tick_delay: std::cell::Cell::new(None) }
    }
    /// Request a tick of this block at this position, without changing its state.
    /// Nonpositive delays reject; repeated requests keep the earliest. The native
    /// backend still owns loaded-chunk checks and queue deduplication.
    pub fn schedule_tick(&self, delay: i32) -> bool {
        if delay <= 0 { return false; }
        self.tick_delay.set(Some(self.tick_delay.get().map_or(delay, |old| old.min(delay))));
        true
    }
    /// The requested delay, for adapters and custom hosts; None if none was requested.
    pub fn scheduled_tick_delay(&self) -> Option<i32> { self.tick_delay.get() }
}

/// An entity is inside the block (Minecraft: `entityInside`).
#[derive(Clone, Debug, PartialEq)]
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
    /// The entity's bounding box, as `[min x, min y, min z, max x, max y, max z]` (`Entity.getBoundingBox`):
    /// the box the original's contact tests are computed from — a punji, for one, hurts only a living
    /// entity whose box crosses the block's mid-plane on the facing axis.
    pub bounds: [f64; 6],
    /// How far the entity has fallen, in blocks (`Entity.fallDistance`): the original's fall bonus on
    /// contact damage reads it — the same punji deals the fall distance + 1 more after a fall.
    pub fall_distance: f64,
    /// What the callback does to the entity, which the game does after the callback ran (the seams run
    /// inside entity movement, where the entity may not be changed): add to it with [`EntityInsideContext::act`].
    pub actions: RefCell<Vec<EntityAction>>,
}

impl EntityInsideContext {
    /// Queues something to do to the entity inside (see [`EntityAction`]): it lands once the movement
    /// seams are done with the entity.
    pub fn act(&self, action: EntityAction) {
        self.actions.borrow_mut().push(action);
    }

    /// The queued actions, taken out (the sink reads them after the callback).
    pub(crate) fn take_actions(&self) -> Vec<EntityAction> {
        std::mem::take(&mut *self.actions.borrow_mut())
    }
}

/// A server-side landing's calculated damage before hurt handling. This is
/// independent of bounce velocity; changing damage does not create a bounce.
/// Native player landings are connected; mob/item adapters remain unavailable.
pub struct FallOnContext {
    pub state: BlockState,
    pub pos: BlockPos,
    pub entity: i32,
    pub kind: String,
    pub distance: f64,
    pub damage: f32,
}

/// Entity motion immediately after the native horizontal collision response.
/// A registered callback owns vertical landing response; no native vertical
/// restitution is applied afterwards. Current native coverage is players.
pub struct AfterFallContext {
    pub state: BlockState,
    pub pos: BlockPos,
    pub entity: i32,
    pub kind: String,
    pub living: bool,
    pub item: bool,
    pub suppressing_bounce: bool,
    pub velocity: [f64; 3],
    pub fall_distance: f64,
    pub on_ground: bool,
}

/// Mutable motion at the block's entityInside callback, separate from the
/// existing read-only contact notification. Only finite velocity is applied.
/// Native sound/network-side effects are not implied by a velocity edit.
pub struct EntityInsideMotionContext {
    pub state: BlockState,
    pub pos: BlockPos,
    pub entity: i32,
    pub kind: String,
    pub server: bool,
    pub suppressing_bounce: bool,
    pub position: [f64; 3],
    pub previous_position: [f64; 3],
    pub velocity: [f64; 3],
    /// Request the server's existing velocity packet; never clears a prior mark.
    pub sync_velocity: bool,
    /// Request the contacted block's step sound at its centre: volume/pitch.
    /// Accepted by the authoritative same-tick queue, not an audio-delivery guarantee.
    pub step_sound: Option<[f32; 2]>,
}
type StickyFlagFn = Arc<dyn Fn(&BlockState) -> bool + Send + Sync>;
type StickToFn = Arc<dyn Fn(&BlockState, &BlockState) -> bool + Send + Sync>;
type PathNodeFn = Arc<dyn Fn(&PathNodeTypeContext<'_>) -> Option<PathNodeType> + Send + Sync>;
pub(crate) type InsideMotionFn = Arc<dyn Fn(&mut EntityInsideMotionContext) + Send + Sync>;
pub(crate) type AfterFallFn = Arc<dyn Fn(&mut AfterFallContext) + Send + Sync>;
type FallOnFn = Arc<dyn Fn(&mut FallOnContext) + Send + Sync>;
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

/// Minecraft's block piston reaction, independent of slime/sticky predicates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PistonBehavior {
    Normal = 0,
    Destroy = 1,
    Block = 2,
    Ignore = 3,
    PushOnly = 4,
}

/// What one block of the mod does. Build it, then give it to [`BlockBehaviors::register`].
#[derive(Clone)]
pub struct BlockBehavior {
    block: Identifier,
    can_survive: Option<SurviveFn>,
    state_for_placement: Option<PlacementFn>,
    update_shape: Option<ShapeFn>,
    entity_inside: Option<InsideFn>,
    fall_on: Option<FallOnFn>,
    after_fall: Option<AfterFallFn>,
    inside_motion: Option<InsideMotionFn>,
    slime_block: Option<StickyFlagFn>,
    sticky_block: Option<StickyFlagFn>,
    stick_to: Option<StickToFn>,
    path_node_type: Option<PathNodeFn>,
    piston_behavior: Option<PistonBehavior>,
    random_tick: Option<TickFn>,
    scheduled_tick: Option<TickFn>,
    can_bonemeal: Option<BonemealCheckFn>,
    bonemeal_success: Option<BonemealSuccessFn>,
    on_bonemeal: Option<TickFn>,
    may_place_on: Option<PlaceOnFn>,
    pick_stack: Option<PickFn>,
    flammable: Option<(i32, i32)>,
}

impl BlockBehavior {
    /// A behaviour for `block` that changes nothing yet.
    pub fn new(block: Identifier) -> Self {
        Self { block, can_survive: None, state_for_placement: None, update_shape: None, entity_inside: None, fall_on: None, after_fall: None, inside_motion: None, slime_block: None, sticky_block: None, stick_to: None, path_node_type: None, piston_behavior: None, random_tick: None, scheduled_tick: None, can_bonemeal: None, bonemeal_success: None, on_bonemeal: None, may_place_on: None, pick_stack: None, flammable: None }
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

    /// Classify a path node with borrowed world context. None keeps the native
    /// answer; Some(Blocked) is an explicit answer. Native adapter pending.
    pub fn path_node_type(mut self, f: impl Fn(&PathNodeTypeContext<'_>) -> Option<PathNodeType> + Send + Sync + 'static) -> Self {
        self.path_node_type = Some(Arc::new(f));
        self
    }

    /// Source isSlimeBlock predicate. Accepted but not connected to native
    /// piston/entity-push queries on the pinned host (documented stub).
    pub fn is_slime_block(mut self, f: impl Fn(&BlockState) -> bool + Send + Sync + 'static) -> Self {
        self.slime_block = Some(Arc::new(f));
        self
    }

    /// Source isStickyBlock predicate; native piston dispatch is unavailable.
    pub fn is_sticky_block(mut self, f: impl Fn(&BlockState) -> bool + Send + Sync + 'static) -> Self {
        self.sticky_block = Some(Arc::new(f));
        self
    }

    /// This block's directional canStickTo predicate, not the complete pair
    /// resolver. Native piston dispatch is unavailable on the pinned host.
    pub fn can_stick_to(mut self, f: impl Fn(&BlockState, &BlockState) -> bool + Send + Sync + 'static) -> Self {
        self.stick_to = Some(Arc::new(f));
        self
    }

    /// Edits velocity during entityInside, without implying sound or networking.
    /// The read-only entity_inside notification remains a separate callback.
    pub fn entity_inside_motion(mut self, f: impl Fn(&mut EntityInsideMotionContext) + Send + Sync + 'static) -> Self {
        self.inside_motion = Some(Arc::new(f));
        self
    }

    /// Owns vertical landing response (Minecraft: updateEntityAfterFallOn).
    /// Native horizontal collision response has already run. No additional
    /// vertical gravity/drag restitution follows. Player adapter only so far;
    /// this does not change fall damage or dispatch on side contacts.
    pub fn update_entity_after_fall_on(mut self, f: impl Fn(&mut AfterFallContext) + Send + Sync + 'static) -> Self {
        self.after_fall = Some(Arc::new(f));
        self
    }

    /// Changes server-side calculated landing damage (Minecraft: fallOn).
    /// Bounce velocity is a different operation. Player adapter implemented;
    /// other entity adapters remain unsupported. Invalid damage is ignored.
    pub fn fall_on(mut self, f: impl Fn(&mut FallOnContext) + Send + Sync + 'static) -> Self {
        self.fall_on = Some(Arc::new(f));
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

    /// Static piston reaction for every state of this declared mod block.
    /// Register before the content declaration snapshot is collected. Native
    /// attachment remains source-only behind the native attachment review gate.
    pub fn piston_behavior(mut self, behavior: PistonBehavior) -> Self {
        self.piston_behavior = Some(behavior);
        self
    }

    /// Explicit static piston reaction; None retains the content template.
    pub fn piston_behavior_override(&self) -> Option<PistonBehavior> {
        self.piston_behavior
    }

    /// The block's fire odds (Minecraft: `FireBlock.setFlammable`): how readily fire spreads onto
    /// the block (ignite) and how strongly it feeds the fire (burn). `(0, 0)` never burns. Without
    /// it the block keeps the odds of the template its declaration copies.
    pub fn flammable(mut self, ignite: i32, burn: i32) -> Self {
        self.flammable = Some((ignite, burn));
        self
    }

    /// The fire odds given with [`BlockBehavior::flammable`], if any.
    pub fn flammable_odds(&self) -> Option<(i32, i32)> {
        self.flammable
    }

    /// The block this behaviour belongs to.
    pub fn block(&self) -> &Identifier {
        &self.block
    }

}

static BEHAVIORS: Mutex<Vec<BlockBehavior>> = Mutex::new(Vec::new());

pub(crate) fn piston_behavior_declarations() -> Result<Vec<(String, PistonBehavior)>, String> {
    let behaviors=BEHAVIORS.lock().map_err(|_|"block behavior registry poisoned")?;
    Ok(behaviors.iter().filter_map(|b|b.piston_behavior.map(|p|(b.block.to_string(),p))).collect())
}

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

pub(crate) fn path_node_type(context: &PathNodeTypeContext<'_>) -> Option<PathNodeType> {
    let callback = BlockBehaviors::get(&context.state.block.to_string())?.path_node_type?;
    callback(context)
}

pub(crate) fn fire_odds(block: &str) -> Option<(i32, i32)> {
    BlockBehaviors::get(block)?.flammable
}

pub(crate) fn slime_block(state: &BlockState) -> Option<bool> {
    let callback = BlockBehaviors::get(&state.block.to_string())?.slime_block?;
    Some(callback(state))
}
pub(crate) fn sticky_block(state: &BlockState) -> Option<bool> {
    let callback = BlockBehaviors::get(&state.block.to_string())?.sticky_block?;
    Some(callback(state))
}
pub(crate) fn stick_to(state: &BlockState, other: &BlockState) -> Option<bool> {
    let callback = BlockBehaviors::get(&state.block.to_string())?.stick_to?;
    Some(callback(state, other))
}

pub(crate) fn inside_motion_handler(block: &str) -> Option<InsideMotionFn> {
    BlockBehaviors::get(block)?.inside_motion
}

pub(crate) fn apply_inside_motion(callback: &InsideMotionFn, context: &mut EntityInsideMotionContext) {
    let (velocity, sync, sound) = (context.velocity, context.sync_velocity, context.step_sound);
    callback(context);
    let sound_valid = context.step_sound.is_none_or(|values| values.iter().all(|v| v.is_finite() && *v >= 0.0));
    if !context.velocity.iter().all(|v| v.is_finite()) || !sound_valid {
        (context.velocity, context.sync_velocity, context.step_sound) = (velocity, sync, sound);
    }
}

pub(crate) fn after_fall_handler(block: &str) -> Option<AfterFallFn> {
    BlockBehaviors::get(block)?.after_fall
}

pub(crate) fn apply_after_fall(callback: &AfterFallFn, context: &mut AfterFallContext) {
    let (velocity, distance, on_ground) = (context.velocity, context.fall_distance, context.on_ground);
    callback(context);
    if !context.velocity.iter().all(|v|v.is_finite()) || !context.fall_distance.is_finite() || context.fall_distance < 0.0 {
        (context.velocity, context.fall_distance, context.on_ground) = (velocity, distance, on_ground);
    }
}

pub(crate) fn has_fall_on(block: &str) -> bool {
    BlockBehaviors::get(block).is_some_and(|b| b.fall_on.is_some())
}

pub(crate) fn fall_on(block: &str, context: &mut FallOnContext) {
    if let Some(callback) = BlockBehaviors::get(block).and_then(|b| b.fall_on) {
        let before = context.damage;
        callback(context); // no registry lock held during reentrant callbacks
        context.damage = if context.damage.is_finite() { context.damage.max(0.0) } else { before };
    }
}

pub(crate) fn survives(block: &str, context: &SurviveContext<'_>) -> Option<bool> {
    BlockBehaviors::get(block)?.can_survive.map(|f| f(context))
}

/// The game's own `BlockState.canSurvive` for a state that may not be in the world yet, as the
/// host's `Sim::can_survive` answers it (horizon-world `level/survive.rs`, host rev 441f49d):
/// a behaviour registered for the block answers first (the host's `before_can_survive` seam),
/// then the vanilla class chain, most specific first, decides. `Some(true)` survives,
/// `Some(false)` does not, `None` where the view cannot say — a cell whose answer lives in
/// a block entity (a moving piston's carried block, which no published table carries), a
/// position-dependent shape's faces, or a fact the view was not given. A caller that places
/// on `None` keeps the pre-check behaviour: the check only ever adds refusals it can prove.
pub fn can_survive(state: &BlockState, pos: BlockPos, view: &dyn BlockView) -> Option<bool> {
    if let Some(answer) = survives(&state.block.to_string(), &SurviveContext { state: state.clone(), pos, view }) {
        return Some(answer);
    }
    let facts = view.placement_facts(state)?;
    can_survive_at(&facts.classes, 0, state, pos, view)
}

fn below(pos: BlockPos) -> BlockPos { Direction::Down.relative(pos) }
fn above(pos: BlockPos) -> BlockPos { Direction::Up.relative(pos) }

fn is_air_state(state: &BlockState) -> bool {
    matches!(state.block.to_string().as_str(), "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air")
}

fn is_block(state: &BlockState, block: &str) -> bool { state.block.to_string() == block }

fn facing_of(state: &BlockState) -> Option<Direction> {
    state.property("facing").and_then(Direction::parse)
}

/// A missing `half` is the lower half: every block this reads of (doors, double plants,
/// seagrass, pitcher crops, dripleaves) defaults to it.
fn half_is_upper(state: &BlockState) -> bool { state.property("half") == Some("upper") }

fn prop_true(state: &BlockState, key: &str) -> bool { state.property(key) == Some("true") }

fn prop_int(state: &BlockState, key: &str) -> Option<i32> {
    state.property(key).and_then(|v| v.parse().ok())
}

/// `a || b` where `a` is known and `b` may be unknown: a known true decides.
fn or_known(a: bool, b: Option<bool>) -> Option<bool> {
    if a { Some(true) } else { b }
}

/// `a || b` where both may be unknown: a known true decides, two known falses refuse.
fn or_opt(a: Option<bool>, b: Option<bool>) -> Option<bool> {
    match (a, b) {
        (Some(true), _) | (_, Some(true)) => Some(true),
        (Some(false), Some(false)) => Some(false),
        _ => None,
    }
}

/// `a && b` where `b` may be unknown: a known false refuses.
fn and_opt(a: bool, b: Option<bool>) -> Option<bool> {
    if a { b } else { Some(false) }
}

/// A moving piston's support answers live in its block entity (the block it carries and
/// how far it has moved), which no published table carries: every support question about
/// the cell is unknown rather than the table's empty shape for the piston itself.
fn is_moving_piston(state: &BlockState) -> bool { is_block(state, "minecraft:moving_piston") }

/// `Sim::sturdy`: the FULL support of the block at `pos` in `face`, with the host's dynamic
/// paths — a moving piston's cell answers through its block entity, so it is unknown here.
/// A shulker box reads as its published shape (a closed cube), as the game's own table
/// reads it; the lid's animation is not published.
fn support_full(view: &dyn BlockView, pos: BlockPos, face: Direction) -> Option<bool> {
    let state = view.block_state(pos)?;
    if is_moving_piston(&state) { return None; }
    view.face_sturdy(pos, face)
}

/// `Sim::can_support_center`: the CENTER support of the block at `pos` in `face`, from the
/// published sturdy bits. A block in `unstable_bottom_center` never centres from below,
/// and a moving piston's cell is unknown, as in [`support_full`].
fn support_center(view: &dyn BlockView, pos: BlockPos, face: Direction) -> Option<bool> {
    let state = view.block_state(pos)?;
    if is_moving_piston(&state) { return None; }
    if face == Direction::Down
        && view.is_state_in_tag(&state, "minecraft:unstable_bottom_center") == Some(true)
    {
        return Some(false);
    }
    let facts = view.survival_facts(&state)?;
    Some(facts.sturdy_center & (1 << face as u8) != 0)
}

/// `Sim::can_support_rigid_block`: the RIGID support of the block at `pos` from below,
/// from the published sturdy bits; a moving piston's cell is unknown, as in
/// [`support_full`].
fn support_rigid(view: &dyn BlockView, pos: BlockPos) -> Option<bool> {
    let state = view.block_state(pos)?;
    if is_moving_piston(&state) { return None; }
    let facts = view.survival_facts(&state)?;
    Some(facts.sturdy_rigid & (1 << Direction::Up as u8) != 0)
}

/// `MultifaceBlock.canAttachTo` for the block at `pos` and a face reaching it in
/// `direction`: the block's face on the attaching side is sturdy for the full support
/// type or full across the whole cell.
fn multiface_attach(view: &dyn BlockView, pos: BlockPos, direction: Direction) -> Option<bool> {
    let face = direction.opposite();
    let state = view.block_state(pos)?;
    let full = view.survival_facts(&state).map(|facts| facts.full_faces & (1 << face as u8) != 0);
    or_opt(view.face_sturdy(pos, face), full)
}

/// `FireBlock.canBurn` over the published facts: ignite odds above zero, and a
/// waterlogged state never burns.
fn can_burn(view: &dyn BlockView, state: &BlockState) -> Option<bool> {
    if prop_true(state, "waterlogged") { return Some(false); }
    let facts = view.survival_facts(state)?;
    Some(facts.ignite_odds > 0)
}

fn can_survive_at(classes: &[String], depth: usize, state: &BlockState, pos: BlockPos, view: &dyn BlockView) -> Option<bool> {
    for (index, class) in classes.iter().enumerate().skip(depth) {
        match can_survive_class(class, index, classes, state, pos, view) {
            None => continue,
            Some(answer) => return answer,
        }
    }
    Some(true)
}

/// One class of the chain: `None` when the class has no `canSurvive` arm (the next class
/// decides), `Some(answer)` when it does — `Some(None)` where the arm needs a fact the view
/// cannot give, which stops the chain rather than letting a later class guess.
#[allow(clippy::too_many_lines)]
fn can_survive_class(class: &str, index: usize, classes: &[String], state: &BlockState, pos: BlockPos, view: &dyn BlockView) -> Option<Option<bool>> {
    let block = state.block.to_string();
    Some(match class {
        "AmethystClusterBlock" | "ShelfMushroomBlock" | "BaseCoralWallFanBlock" => {
            let Some(facing) = facing_of(state) else { return Some(None) };
            support_full(view, facing.opposite().relative(pos), facing)
        }
        "BambooSaplingBlock" | "BambooStalkBlock" => {
            let Some(below_state) = view.block_state(below(pos)) else { return Some(None) };
            view.is_state_in_tag(&below_state, "minecraft:supports_bamboo")
        }
        // `is_solid` is the SOLID flag of the host's behaviour table, which the survival
        // facts carry; `legacy_solid` (`BlockState.isSolid`) is a different field and is
        // not substituted.
        "BannerBlock" | "CakeBlock" | "CandleCakeBlock" | "StandingSignBlock" => {
            let Some(below_state) = view.block_state(below(pos)) else { return Some(None) };
            let Some(facts) = view.survival_facts(&below_state) else { return Some(None) };
            Some(facts.solid)
        }
        "WallBannerBlock" | "WallSignBlock" => {
            let Some(facing) = facing_of(state) else { return Some(None) };
            let target_pos = facing.opposite().relative(pos);
            let Some(target) = view.block_state(target_pos) else { return Some(None) };
            let Some(facts) = view.survival_facts(&target) else { return Some(None) };
            Some(facts.solid)
        }
        "CactusBlock" => {
            for direction in [Direction::North, Direction::South, Direction::West, Direction::East] {
                let at = direction.relative(pos);
                let Some(neighbour) = view.block_state(at) else { return Some(None) };
                let Some(facts) = view.survival_facts(&neighbour) else { return Some(None) };
                if facts.solid || facts.fluid_lava {
                    return Some(Some(false));
                }
            }
            let Some(below_state) = view.block_state(below(pos)) else { return Some(None) };
            let Some(above_state) = view.block_state(above(pos)) else { return Some(None) };
            let Some(above_facts) = view.survival_facts(&above_state) else { return Some(None) };
            match or_known(is_block(&below_state, &block),
                view.is_state_in_tag(&below_state, "minecraft:supports_cactus"))
            {
                Some(true) => Some(!above_facts.liquid),
                other => other,
            }
        }
        "FarmlandBlock" => {
            let Some(above_state) = view.block_state(above(pos)) else { return Some(None) };
            let Some(facts) = view.survival_facts(&above_state) else { return Some(None) };
            if !facts.solid {
                return Some(Some(true));
            }
            view.is_state_in_tag(&above_state, "minecraft:maintains_farmland")
        }
        "PathBlock" => {
            let Some(above_state) = view.block_state(above(pos)) else { return Some(None) };
            let Some(facts) = view.survival_facts(&above_state) else { return Some(None) };
            if !facts.solid {
                return Some(Some(true));
            }
            let Some(above_facts) = view.placement_facts(&above_state) else { return Some(None) };
            Some(above_facts.classes.iter().any(|class| class == "FenceGateBlock"))
        }
        "BaseCoralPlantTypeBlock" | "LeafLitterBlock" => support_full(view, below(pos), Direction::Up),
        // `canSupportRigidBlock` and `canSupportCenter` read the CENTER and RIGID sturdy
        // bits, which the survival facts carry.
        "BasePressurePlateBlock" => or_opt(
            support_rigid(view, below(pos)),
            support_center(view, below(pos), Direction::Up),
        ),
        "BaseRailBlock" | "DiodeBlock" => support_rigid(view, below(pos)),
        "BaseTorchBlock" | "CandleBlock" => support_center(view, below(pos), Direction::Up),
        "LanternBlock" => {
            let direction = if prop_true(state, "hanging") { Direction::Up } else { Direction::Down };
            support_center(view, direction.relative(pos), direction.opposite())
        }
        "SporeBlossomBlock" => {
            match support_center(view, above(pos), Direction::Down) {
                Some(true) => Some(!view.is_water(pos)),
                other => other,
            }
        }
        "CeilingHangingSignBlock" => {
            // The host reads the static table's CENTER bit here (`is_face_sturdy_center`),
            // not `canSupportCenter`: no tag exception and no dynamic path.
            let Some(above_state) = view.block_state(above(pos)) else { return Some(None) };
            let Some(facts) = view.survival_facts(&above_state) else { return Some(None) };
            Some(facts.sturdy_center & (1 << Direction::Down as u8) != 0)
        }
        "BellBlock" => {
            let connection = match state.property("attachment") {
                Some("floor") => Direction::Up,
                Some("ceiling") => Direction::Down,
                _ => {
                    let Some(facing) = facing_of(state) else { return Some(None) };
                    facing.opposite()
                }
            };
            if connection == Direction::Up {
                support_center(view, above(pos), Direction::Down)
            } else {
                support_full(view, connection.relative(pos), connection.opposite())
            }
        }
        "BigDripleafBlock" => {
            let Some(below_state) = view.block_state(below(pos)) else { return Some(None) };
            or_known(is_block(&below_state, &block) || is_block(&below_state, "minecraft:big_dripleaf_stem"),
                view.is_state_in_tag(&below_state, "minecraft:supports_big_dripleaf"))
        }
        "BigDripleafStemBlock" => {
            let Some(below_state) = view.block_state(below(pos)) else { return Some(None) };
            let Some(above_state) = view.block_state(above(pos)) else { return Some(None) };
            let below_ok = or_known(is_block(&below_state, &block),
                view.is_state_in_tag(&below_state, "minecraft:supports_big_dripleaf"));
            let above_ok = is_block(&above_state, &block) || is_block(&above_state, "minecraft:big_dripleaf");
            match below_ok {
                Some(false) => Some(false),
                Some(true) => Some(above_ok),
                None => None,
            }
        }
        "BubbleColumnBlock" => {
            let Some(below_state) = view.block_state(below(pos)) else { return Some(None) };
            or_opt(
                or_known(is_block(&below_state, &block),
                    view.is_state_in_tag(&below_state, "minecraft:enables_bubble_column_push_up")),
                view.is_state_in_tag(&below_state, "minecraft:enables_bubble_column_drag_down"),
            )
        }
        "CarpetBlock" => {
            let Some(below_state) = view.block_state(below(pos)) else { return Some(None) };
            Some(!is_air_state(&below_state))
        }
        "ChorusFlowerBlock" => {
            let Some(below_state) = view.block_state(below(pos)) else { return Some(None) };
            if or_known(is_block(&below_state, "minecraft:chorus_plant"),
                view.is_state_in_tag(&below_state, "minecraft:supports_chorus_flower")) == Some(true)
            {
                return Some(Some(true));
            }
            if !is_air_state(&below_state) {
                return Some(Some(false));
            }
            let mut one = false;
            for direction in [Direction::North, Direction::South, Direction::West, Direction::East] {
                let Some(neighbour) = view.block_state(direction.relative(pos)) else { return Some(None) };
                if is_block(&neighbour, "minecraft:chorus_plant") {
                    if one {
                        return Some(Some(false));
                    }
                    one = true;
                } else if !is_air_state(&neighbour) {
                    return Some(Some(false));
                }
            }
            Some(one)
        }
        "ChorusPlantBlock" => {
            let Some(below_state) = view.block_state(below(pos)) else { return Some(None) };
            let Some(above_state) = view.block_state(above(pos)) else { return Some(None) };
            let vertical = !is_air_state(&above_state) && !is_air_state(&below_state);
            for direction in [Direction::North, Direction::South, Direction::West, Direction::East] {
                let neighbour_pos = direction.relative(pos);
                let Some(neighbour) = view.block_state(neighbour_pos) else { return Some(None) };
                if is_block(&neighbour, &block) {
                    if vertical {
                        return Some(Some(false));
                    }
                    let Some(under) = view.block_state(below(neighbour_pos)) else { return Some(None) };
                    if or_known(is_block(&under, &block),
                        view.is_state_in_tag(&under, "minecraft:supports_chorus_plant")) == Some(true)
                    {
                        return Some(Some(true));
                    }
                }
            }
            or_known(is_block(&below_state, &block),
                view.is_state_in_tag(&below_state, "minecraft:supports_chorus_plant"))
        }
        "CocoaBlock" => {
            let Some(facing) = facing_of(state) else { return Some(None) };
            let Some(target) = view.block_state(facing.relative(pos)) else { return Some(None) };
            view.is_state_in_tag(&target, "minecraft:supports_cocoa")
        }
        "CropBlock" => {
            let Some(light) = view.raw_brightness(pos) else { return Some(None) };
            if light < 8 {
                Some(false)
            } else {
                can_survive_at(classes, index + 1, state, pos, view)
            }
        }
        "DoorBlock" => {
            let Some(below_state) = view.block_state(below(pos)) else { return Some(None) };
            if !half_is_upper(state) {
                view.face_sturdy(below(pos), Direction::Up)
            } else {
                Some(is_block(&below_state, &block))
            }
        }
        "DoublePlantBlock" => {
            if !half_is_upper(state) {
                return Some(can_survive_at(classes, index + 1, state, pos, view));
            }
            let Some(below_state) = view.block_state(below(pos)) else { return Some(None) };
            Some(is_block(&below_state, &block) && !half_is_upper(&below_state))
        }
        "SmallDripleafBlock" => {
            if half_is_upper(state) {
                return Some(can_survive_at(classes, index + 1, state, pos, view));
            }
            let Some(ground) = view.block_state(below(pos)) else { return Some(None) };
            vegetation_may_place_on(state, &ground, below(pos), view)
        }
        "TallSeagrassBlock" => {
            if half_is_upper(state) {
                let Some(below_state) = view.block_state(below(pos)) else { return Some(None) };
                return Some(Some(is_block(&below_state, &block) && !half_is_upper(&below_state)));
            }
            let rest = can_survive_at(classes, index + 1, state, pos, view);
            match rest {
                Some(false) => Some(false),
                Some(true) => Some(view.is_water(pos) && view.is_fluid_full(pos)),
                None => None,
            }
        }
        "PitcherCropBlock" => {
            let lower = is_block(state, "minecraft:pitcher_crop") && !half_is_upper(state);
            if lower {
                let Some(light) = view.raw_brightness(pos) else { return Some(None) };
                if light < 8 {
                    return Some(Some(false));
                }
            }
            can_survive_at(classes, index + 1, state, pos, view)
        }
        "FaceAttachedHorizontalDirectionalBlock" => {
            let connected = match state.property("face") {
                Some("ceiling") => Direction::Down,
                Some("floor") => Direction::Up,
                _ => {
                    let Some(facing) = facing_of(state) else { return Some(None) };
                    facing
                }
            };
            let direction = connected.opposite();
            support_full(view, direction.relative(pos), connected)
        }
        "GrindstoneBlock" => Some(true),
        "PistonHeadBlock" => {
            let Some(facing) = facing_of(state) else { return Some(None) };
            let Some(base) = view.block_state(facing.opposite().relative(pos)) else { return Some(None) };
            let Some(base_facing) = facing_of(&base) else { return Some(None) };
            // The base must be the piston the head's own type names (the sticky piston
            // for a sticky head), extended and facing the head's way — or a moving
            // piston facing the same way.
            let expected = if state.property("type") == Some("sticky") { "minecraft:sticky_piston" } else { "minecraft:piston" };
            let fitting = is_block(&base, expected)
                && prop_true(&base, "extended")
                && base_facing == facing;
            Some(fitting || (is_block(&base, "minecraft:moving_piston") && base_facing == facing))
        }
        "FireBlock" => {
            // `sturdy below || isValidFireLocation`: any neighbour that can burn holds
            // the fire, where a neighbour burns when its ignite odds are above zero.
            match support_full(view, below(pos), Direction::Up) {
                Some(true) => Some(true),
                sturdy => {
                    let mut unknown = sturdy.is_none();
                    for direction in Direction::ALL {
                        let Some(neighbour) = view.block_state(direction.relative(pos)) else { return Some(None) };
                        match can_burn(view, &neighbour) {
                            Some(true) => return Some(Some(true)),
                            Some(false) => {}
                            None => unknown = true,
                        }
                    }
                    if unknown { None } else { Some(false) }
                }
            }
        }
        "SoulFireBlock" => {
            let Some(below_state) = view.block_state(below(pos)) else { return Some(None) };
            view.is_state_in_tag(&below_state, "minecraft:soul_fire_base_blocks")
        }
        "FrogspawnBlock" => {
            // `frogspawn_may_place_on`: the fluid in the cell is in the fluid tag
            // `supports_frogspawn`, or the block in the cell is in the block tag, and
            // the fluid above is empty.
            let Some(here) = view.block_state(pos) else { return Some(None) };
            let Some(above_state) = view.block_state(above(pos)) else { return Some(None) };
            let Some(here_facts) = view.survival_facts(&here) else { return Some(None) };
            let Some(above_facts) = view.survival_facts(&above_state) else { return Some(None) };
            if !above_facts.fluid_empty {
                return Some(Some(false));
            }
            if here_facts.fluid_supports_frogspawn {
                return Some(Some(true));
            }
            view.is_state_in_tag(&here, "minecraft:supports_frogspawn")
        }
        "GrowingPlantBlock" => {
            // The growth direction is the block's own data in the host; the chain names
            // the family (kelp and twisting vines grow up, weeping vines and cave vines
            // grow down), so the chain decides it here.
            let growth = if classes.iter().any(|c| matches!(c.as_str(), "KelpBlock" | "KelpPlantBlock" | "TwistingVinesBlock" | "TwistingVinesPlantBlock")) {
                Direction::Up
            } else if classes.iter().any(|c| matches!(c.as_str(), "WeepingVinesBlock" | "WeepingVinesPlantBlock" | "CaveVinesBlock" | "CaveVinesPlantBlock")) {
                Direction::Down
            } else {
                return Some(None);
            };
            let attached_pos = growth.opposite().relative(pos);
            let Some(attached) = view.block_state(attached_pos) else { return Some(None) };
            if classes.iter().any(|c| c == "KelpBlock" || c == "KelpPlantBlock") {
                let Some(can) = view.is_state_in_tag(&attached, "minecraft:cannot_support_kelp") else { return Some(None) };
                if can {
                    return Some(Some(false));
                }
            }
            let head = block.strip_suffix("_plant").unwrap_or(&block).to_owned();
            let body = format!("{head}_plant");
            or_opt(
                Some(is_block(&attached, &head) || is_block(&attached, &body)),
                view.face_sturdy(attached_pos, growth),
            )
        }
        "HangingMossBlock" => {
            let Some(above_state) = view.block_state(above(pos)) else { return Some(None) };
            or_known(is_block(&above_state, &block), multiface_attach(view, above(pos), Direction::Up))
        }
        "MultifaceBlock" => {
            let mut any = false;
            for direction in Direction::ALL {
                if prop_true(state, direction.name()) {
                    match multiface_attach(view, direction.relative(pos), direction) {
                        Some(true) => any = true,
                        Some(false) => return Some(Some(false)),
                        None => return Some(None),
                    }
                }
            }
            Some(any)
        }
        "VineBlock" => {
            // `VineBlock.getUpdatedState` first: each face the vine claims is retested
            // against what now holds it, and the vine survives while one face is left.
            let mut faces = 0;
            if prop_true(state, "up") {
                match multiface_attach(view, above(pos), Direction::Down) {
                    Some(true) => faces += 1,
                    Some(false) => {}
                    None => return Some(None),
                }
            }
            for direction in [Direction::North, Direction::South, Direction::West, Direction::East] {
                if !prop_true(state, direction.name()) {
                    continue;
                }
                // `vine_can_support_at_face`: the face's own block holds it, or the
                // vine above carries the same face down.
                let attach = multiface_attach(view, direction.relative(pos), direction);
                let supported = match attach {
                    Some(true) => Some(true),
                    _ => {
                        let Some(above_state) = view.block_state(above(pos)) else { return Some(None) };
                        if is_block(&above_state, &block) && prop_true(&above_state, direction.name()) {
                            Some(true)
                        } else {
                            attach
                        }
                    }
                };
                match supported {
                    Some(true) => faces += 1,
                    Some(false) => {}
                    None => return Some(None),
                }
            }
            Some(faces > 0)
        }
        "HangingRootsBlock" => support_full(view, above(pos), Direction::Down),
        "LadderBlock" => {
            let Some(facing) = facing_of(state) else { return Some(None) };
            support_full(view, facing.opposite().relative(pos), facing)
        }
        "MangrovePropaguleBlock" => {
            if prop_true(state, "hanging") {
                let Some(above_state) = view.block_state(above(pos)) else { return Some(None) };
                view.is_state_in_tag(&above_state, "minecraft:supports_hanging_mangrove_propagule")
            } else {
                can_survive_at(classes, index + 1, state, pos, view)
            }
        }
        "MossyCarpetBlock" => {
            let Some(below_state) = view.block_state(below(pos)) else { return Some(None) };
            if prop_true(state, "bottom") {
                Some(!is_air_state(&below_state))
            } else {
                Some(is_block(&below_state, &block) && prop_true(&below_state, "bottom"))
            }
        }
        "MushroomBlock" => {
            let Some(below_state) = view.block_state(below(pos)) else { return Some(None) };
            if view.is_state_in_tag(&below_state, "minecraft:overrides_mushroom_light_requirement") == Some(true) {
                return Some(Some(true));
            }
            let Some(light) = view.raw_brightness(pos) else { return Some(None) };
            if light >= 13 {
                return Some(Some(false));
            }
            vegetation_may_place_on(state, &below_state, below(pos), view)
        }
        "RedstoneWallTorchBlock" | "WallTorchBlock" => {
            let Some(facing) = facing_of(state) else { return Some(None) };
            support_full(view, facing.opposite().relative(pos), facing)
        }
        "RedstoneWireBlock" => {
            let Some(below_state) = view.block_state(below(pos)) else { return Some(None) };
            or_known(is_block(&below_state, "minecraft:hopper"), support_full(view, below(pos), Direction::Up))
        }
        "ScaffoldingBlock" => {
            let Some(below_state) = view.block_state(below(pos)) else { return Some(None) };
            let mut distance = 7;
            let mut below_unknown = false;
            if is_block(&below_state, "minecraft:scaffolding") {
                let Some(d) = prop_int(&below_state, "distance") else { return Some(None) };
                distance = d;
            } else {
                match view.face_sturdy(below(pos), Direction::Up) {
                    Some(true) => return Some(Some(true)),
                    Some(false) => {}
                    // A sturdy face below would hold it at distance 0: with no other
                    // scaffolding nearby the answer stays unknown, not false.
                    None => below_unknown = true,
                }
            }
            for direction in [Direction::North, Direction::South, Direction::West, Direction::East] {
                let Some(neighbour) = view.block_state(direction.relative(pos)) else { return Some(None) };
                if is_block(&neighbour, "minecraft:scaffolding") {
                    let Some(d) = prop_int(&neighbour, "distance") else { return Some(None) };
                    distance = distance.min(d + 1);
                    if distance == 1 {
                        break;
                    }
                }
            }
            if distance < 7 {
                Some(true)
            } else if below_unknown {
                None
            } else {
                Some(false)
            }
        }
        "SeaPickleBlock" => {
            let Some(ground) = view.block_state(below(pos)) else { return Some(None) };
            vegetation_may_place_on(state, &ground, below(pos), view)
        }
        "SnowLayerBlock" => {
            let Some(below_state) = view.block_state(below(pos)) else { return Some(None) };
            match view.is_state_in_tag(&below_state, "minecraft:cannot_support_snow_layer") {
                Some(true) => Some(false),
                Some(false) => match view.is_state_in_tag(&below_state, "minecraft:support_override_snow_layer") {
                    Some(true) => Some(true),
                    Some(false) => {
                        // The shape case: the below block's full up face holds the layer,
                        // or eight layers of snow itself do. A position-dependent shape's
                        // faces are not published, so those stay unknown.
                        let Some(facts) = view.survival_facts(&below_state) else { return Some(None) };
                        if facts.dynamic_shape {
                            return Some(None);
                        }
                        Some(facts.full_faces & (1 << Direction::Up as u8) != 0
                            || (is_block(&below_state, &block) && prop_int(&below_state, "layers") == Some(8)))
                    }
                    None => None,
                },
                None => None,
            }
        }
        "SpeleothemBlock" => {
            let tip = state.property("vertical_direction").and_then(Direction::parse).unwrap_or(Direction::Up);
            let behind_pos = tip.opposite().relative(pos);
            let Some(behind) = view.block_state(behind_pos) else { return Some(None) };
            or_known(
                is_block(&behind, &block)
                    && behind.property("vertical_direction").and_then(Direction::parse) == Some(tip),
                view.face_sturdy(behind_pos, tip),
            )
        }
        "SugarCaneBlock" => {
            let Some(below_state) = view.block_state(below(pos)) else { return Some(None) };
            if is_block(&below_state, &block) {
                return Some(Some(true));
            }
            match view.is_state_in_tag(&below_state, "minecraft:supports_sugar_cane") {
                Some(false) => Some(false),
                None => None,
                Some(true) => {
                    let mut unknown = false;
                    for direction in [Direction::North, Direction::South, Direction::West, Direction::East] {
                        let at = direction.relative(below(pos));
                        let Some(adjacent) = view.block_state(at) else { return Some(None) };
                        // Either half of the check decides a direction: the fluid beside
                        // the ground in the fluid tag `supports_sugar_cane_adjacently`,
                        // or the block there in the block tag of the same name.
                        let fluid = view.survival_facts(&adjacent).map(|facts| facts.fluid_supports_sugar_cane_adjacently);
                        match or_opt(fluid, view.is_state_in_tag(&adjacent, "minecraft:supports_sugar_cane_adjacently")) {
                            Some(true) => return Some(Some(true)),
                            Some(false) => {}
                            None => unknown = true,
                        }
                    }
                    if unknown { None } else { Some(false) }
                }
            }
        }
        "TripWireHookBlock" => {
            let Some(facing) = facing_of(state) else { return Some(None) };
            if !matches!(facing, Direction::North | Direction::South | Direction::West | Direction::East) {
                return Some(Some(false));
            }
            support_full(view, facing.opposite().relative(pos), facing)
        }
        "VegetationBlock" => {
            let Some(ground) = view.block_state(below(pos)) else { return Some(None) };
            vegetation_may_place_on(state, &ground, below(pos), view)
        }
        _ => return None,
    })
}

/// `VegetationBlock.mayPlaceOn` for `state`'s block, as the host's `Sim::may_place_on` answers
/// it: a behaviour registered for the block answers first, then the first matching class in
/// the chain decides. `None` where no class matches, or where a fact the view lacks is needed
/// (a fluid tag, a shape face the table does not publish).
pub fn vegetation_may_place_on(state: &BlockState, ground: &BlockState, ground_pos: BlockPos, view: &dyn BlockView) -> Option<bool> {
    if let Some(answer) = may_place_on(&state.block.to_string(), &PlaceOnContext { state: state.clone(), ground: ground.clone(), ground_pos, view }) {
        return Some(answer);
    }
    let facts = view.placement_facts(state)?;
    for class in &facts.classes {
        match class.as_str() {
            // A stem's support set is the block's own data, which the survival facts
            // publish by name: the ground's block must be in the set.
            "AttachedStemBlock" | "NetherFungusBlock" | "NetherRootsBlock" | "StemBlock" => {
                let Some(facts) = view.survival_facts(state) else { return None };
                return Some(facts.support_blocks.iter().any(|name| name == &ground.block.to_string()));
            }
            "AzaleaBlock" => return view.is_state_in_tag(ground, "minecraft:supports_azalea"),
            "CactusFlowerBlock" => {
                return match view.is_state_in_tag(ground, "minecraft:support_override_cactus_flower") {
                    Some(true) => Some(true),
                    _ => {
                        let Some(facts) = view.survival_facts(ground) else { return None };
                        Some(facts.sturdy_center & (1 << Direction::Up as u8) != 0)
                    }
                };
            }
            "CropBlock" | "PitcherCropBlock" => return view.is_state_in_tag(ground, "minecraft:supports_crops"),
            "DryVegetationBlock" => return view.is_state_in_tag(ground, "minecraft:supports_dry_vegetation"),
            "LilyPadBlock" => {
                // The fluid in the ground cell is in the fluid tag `supports_lily_pad`
                // or the ground block is in the block tag, and the fluid above is empty.
                let Some(above_state) = view.block_state(above(ground_pos)) else { return None };
                let Some(above_facts) = view.survival_facts(&above_state) else { return None };
                if !above_facts.fluid_empty {
                    return Some(false);
                }
                let Some(ground_facts) = view.survival_facts(ground) else { return None };
                if ground_facts.fluid_supports_lily_pad {
                    return Some(true);
                }
                return view.is_state_in_tag(ground, "minecraft:supports_lily_pad");
            }
            "MangrovePropaguleBlock" => return view.is_state_in_tag(ground, "minecraft:supports_mangrove_propagule"),
            "MushroomBlock" => {
                let Some(facts) = view.placement_facts(ground) else { return None };
                return Some(facts.solid_render);
            }
            "NetherSproutsBlock" => return view.is_state_in_tag(ground, "minecraft:supports_nether_sprouts"),
            "NetherWartBlock" => return view.is_state_in_tag(ground, "minecraft:supports_nether_wart"),
            "SeaPickleBlock" => {
                // The shape half: the ground's up face is not empty, unless its shape is
                // position-dependent and the face is not published; the sturdy half is
                // the static FULL bit, as the host reads it here.
                let shape = match view.survival_facts(ground) {
                    Some(facts) if !facts.dynamic_shape => {
                        Some(facts.nonempty_faces & (1 << Direction::Up as u8) != 0)
                    }
                    _ => None,
                };
                return or_opt(shape, view.face_sturdy(ground_pos, Direction::Up));
            }
            "SeagrassBlock" | "TallSeagrassBlock" => {
                return match (view.face_sturdy(ground_pos, Direction::Up), view.is_state_in_tag(ground, "minecraft:cannot_support_seagrass")) {
                    (Some(true), Some(false)) => Some(true),
                    (Some(false), _) | (_, Some(true)) => Some(false),
                    _ => None,
                };
            }
            "SmallDripleafBlock" => {
                let second = (|| {
                    let above_state = view.block_state(above(ground_pos))?;
                    let water = is_block(&above_state, "minecraft:water")
                        && above_state.property("level").is_none_or(|v| v == "0");
                    and_opt(water, view.is_state_in_tag(ground, "minecraft:supports_vegetation"))
                })();
                return or_opt(view.is_state_in_tag(ground, "minecraft:supports_small_dripleaf"), second);
            }
            "WitherRoseBlock" => return view.is_state_in_tag(ground, "minecraft:supports_wither_rose"),
            "VegetationBlock" => return view.is_state_in_tag(ground, "minecraft:supports_vegetation"),
            _ => continue,
        }
    }
    None
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

pub(crate) fn inside(block: &str, context: &EntityInsideContext) -> Vec<EntityAction> {
    if let Some(f) = BlockBehaviors::get(block).and_then(|b| b.entity_inside) {
        f(context);
    }
    context.take_actions()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(text: &str) -> Identifier {
        Identifier::parse(text).unwrap()
    }

    #[test]
    fn fire_odds_come_from_the_registered_behaviour() {
        assert_eq!(fire_odds("example:unregistered_flammable"), None);
        BlockBehaviors::register(BlockBehavior::new(id("example:flammable")).flammable(30, 60));
        assert_eq!(fire_odds("example:flammable"), Some((30, 60)));
        BlockBehaviors::register(BlockBehavior::new(id("example:fireproof")).flammable(0, 0));
        assert_eq!(fire_odds("example:fireproof"), Some((0, 0)));
        // A behaviour without odds, and a replaced registration, answer None again.
        BlockBehaviors::register(BlockBehavior::new(id("example:plain")));
        assert_eq!(fire_odds("example:plain"), None);
        BlockBehaviors::register(BlockBehavior::new(id("example:flammable")));
        assert_eq!(fire_odds("example:flammable"), None);
    }

    #[test]
    fn stickiness_queries_preserve_directional_false_and_allow_reentrant_registration() {
        let state = BlockState::parse("example:sticky").unwrap();
        let stone = BlockState::parse("minecraft:stone").unwrap();
        assert_eq!(slime_block(&state), None);
        BlockBehaviors::register(BlockBehavior::new(id("example:sticky"))
            .is_slime_block(|_| true)
            .is_sticky_block(|_| { BlockBehaviors::register(BlockBehavior::new(id("example:reentrant_sticky"))); true })
            .can_stick_to(|state, other| state.block == other.block));
        assert_eq!(slime_block(&state), Some(true));
        assert_eq!(sticky_block(&state), Some(true));
        assert_eq!(stick_to(&state, &stone), Some(false));
        assert_eq!(stick_to(&state, &state), Some(true));
        assert_eq!(stick_to(&stone, &state), None, "never infer the opposite side's predicate");
    }

    #[test]
    fn inside_motion_captures_callbacks_and_rejects_partial_nonfinite_velocity() {
        let block = "example:inside_motion";
        BlockBehaviors::register(BlockBehavior::new(id(block)).entity_inside_motion(|c| {
            // Reentrant registration must not deadlock or replace this invocation.
            BlockBehaviors::register(BlockBehavior::new(id("example:inside_motion")));
            c.velocity = [0.4, c.velocity[1], -0.2];
        }));
        let callback = inside_motion_handler(block).unwrap();
        let mut c = EntityInsideMotionContext { state: BlockState::parse(block).unwrap(), pos:[0,64,0], entity:1,
            kind:"minecraft:player".into(),server:true,suppressing_bounce:false,
            position:[0.0,64.2,0.5],previous_position:[-0.2,64.2,0.5],velocity:[-0.3,-0.08,0.1],sync_velocity:false,step_sound:None };
        apply_inside_motion(&callback, &mut c);
        assert_eq!(c.velocity, [0.4,-0.08,-0.2]);
        assert!(inside_motion_handler(block).is_none());
        let invalid: InsideMotionFn = Arc::new(|c| { c.velocity = [1.0,f64::INFINITY,2.0]; c.sync_velocity = true; });
        apply_inside_motion(&invalid, &mut c);
        assert_eq!(c.velocity, [0.4,-0.08,-0.2]);
        assert!(!c.sync_velocity, "invalid motion cannot request a packet");
        let invalid_sound: InsideMotionFn = Arc::new(|c| { c.velocity[0]=2.0; c.sync_velocity=true; c.step_sound=Some([1.0,f32::NAN]); });
        apply_inside_motion(&invalid_sound, &mut c);
        assert_eq!(c.velocity, [0.4,-0.08,-0.2]);
        assert!(!c.sync_velocity && c.step_sound.is_none(), "invalid sound rejects all outputs together");
    }

    #[test]
    fn captured_after_fall_handler_survives_registry_replacement_and_invalid_motion_is_atomic() {
        let block="example:after_fall";
        BlockBehaviors::register(BlockBehavior::new(id(block)).update_entity_after_fall_on(|c| {
            c.velocity[1] = -c.velocity[1]; c.fall_distance = 0.0; c.on_ground = false;
        }));
        let callback=after_fall_handler(block).unwrap();
        BlockBehaviors::register(BlockBehavior::new(id(block)));
        assert!(after_fall_handler(block).is_none());
        let mut c=AfterFallContext { state:BlockState::parse(block).unwrap(),pos:[0,64,0],entity:1,
            kind:"minecraft:player".into(),living:true,item:false,suppressing_bounce:false,
            velocity:[0.2,-0.7,0.3],fall_distance:10.0,on_ground:true };
        apply_after_fall(&callback,&mut c);
        assert_eq!((c.velocity,c.fall_distance,c.on_ground),([0.2,0.7,0.3],0.0,false));
        let invalid:AfterFallFn=Arc::new(|c| {c.velocity[0]=f64::NAN; c.fall_distance=20.0; c.on_ground=true;});
        apply_after_fall(&invalid,&mut c);
        assert_eq!((c.velocity,c.fall_distance,c.on_ground),([0.2,0.7,0.3],0.0,false));
    }
    #[test]
    fn fall_damage_callbacks_are_reentrant_and_reject_invalid_damage() {
        let block="example:fall_damage";
        BlockBehaviors::register(BlockBehavior::new(id(block)).fall_on(|c| {
            assert!(BlockBehaviors::get("example:fall_damage").is_some());
            c.damage=0.0;
        }));
        let mut c=FallOnContext {state:BlockState::parse(block).unwrap(),pos:[0,64,0],entity:1,kind:"minecraft:player".into(),distance:10.0,damage:7.0};
        assert!(has_fall_on(block));
        fall_on(block,&mut c);
        assert_eq!(c.damage,0.0);
        BlockBehaviors::register(BlockBehavior::new(id(block)).fall_on(|c|c.damage=f32::NAN));
        c.damage=7.0;
        fall_on(block,&mut c);
        assert_eq!(c.damage,7.0);
        BlockBehaviors::register(BlockBehavior::new(id(block)).fall_on(|c|c.damage=-3.0));
        fall_on(block,&mut c);
        assert_eq!(c.damage,0.0);
        assert!(!has_fall_on("example:unregistered_fall"));
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
        let count = memory.writes.len();
        let decorator = super::super::worldgen_api_v1::TreeDecorator::new(|_| panic!("unsupported tree must not invoke decorators"));
        assert!(!memory.place_tree(&serde_json::Value::Null, pos, &[decorator]));
        assert_eq!(memory.writes.len(), count);
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

    fn survival_facts(classes: &[&str]) -> PlacementFacts {
        PlacementFacts { classes: classes.iter().map(|c| c.to_string()).collect(), full_collision: false, solid_render: false }
    }

    fn survival_view() -> MemoryBlockView {
        let mut view = MemoryBlockView::default();
        view.placement_facts.insert("minecraft:frosted_ice".to_owned(), survival_facts(&["FrostedIceBlock", "IceBlock", "HalfTransparentBlock", "Block"]));
        view.placement_facts.insert("minecraft:torch".to_owned(), survival_facts(&["TorchBlock", "BaseTorchBlock", "Block"]));
        view.placement_facts.insert("minecraft:wall_torch".to_owned(), survival_facts(&["WallTorchBlock", "TorchBlock", "BaseTorchBlock", "Block"]));
        view.placement_facts.insert("minecraft:wheat".to_owned(), survival_facts(&["CropBlock", "BushBlock", "Block"]));
        view.placement_facts.insert("minecraft:brown_mushroom".to_owned(), survival_facts(&["MushroomBlock", "BushBlock", "Block"]));
        view.placement_facts.insert("minecraft:white_carpet".to_owned(), survival_facts(&["WoolCarpetBlock", "CarpetBlock", "Block"]));
        view.placement_facts.insert("minecraft:oak_door".to_owned(), survival_facts(&["DoorBlock", "Block"]));
        view.placement_facts.insert("minecraft:fire".to_owned(), survival_facts(&["FireBlock", "BaseFireBlock", "Block"]));
        view.placement_facts.insert("minecraft:scaffolding".to_owned(), survival_facts(&["ScaffoldingBlock", "Block"]));
        view.placement_facts.insert("minecraft:sugar_cane".to_owned(), survival_facts(&["SugarCaneBlock", "Block"]));
        view.placement_facts.insert("minecraft:chorus_flower".to_owned(), survival_facts(&["ChorusFlowerBlock", "Block"]));
        view.placement_facts.insert("minecraft:piston_head".to_owned(), survival_facts(&["PistonHeadBlock", "Block"]));
        view.placement_facts.insert("minecraft:snow".to_owned(), survival_facts(&["SnowLayerBlock", "Block"]));
        view.placement_facts.insert("minecraft:dirt".to_owned(), PlacementFacts { classes: vec!["Block".to_owned()], full_collision: true, solid_render: true });
        view
    }

    #[test]
    fn survival_answers_the_chain_and_never_guesses_what_the_facts_lack() {
        let view = survival_view();
        let at = |state: &str| can_survive(&BlockState::parse(state).unwrap(), [0, 64, 0], &view);
        // Frosted ice has no surviving arm in the host's chain: it survives anywhere.
        assert_eq!(at("minecraft:frosted_ice"), Some(true));
        // A standing torch needs the CENTER sturdy bit, which the published facts do not carry.
        assert_eq!(at("minecraft:torch"), None);
        // A wall torch reads the sturdy face of the wall it faces.
        assert_eq!(at("minecraft:wall_torch[facing=east]"), Some(false), "no wall to the west");
        // A block with no placement facts at all cannot be judged.
        assert_eq!(can_survive(&BlockState::parse("minecraft:stone").unwrap(), [0, 64, 0], &view), None);
    }

    #[test]
    fn survival_reads_sturdy_faces_light_and_the_ground_below() {
        let mut view = survival_view();
        view.blocks.insert([0, 64, 0], BlockState::parse("minecraft:stone").unwrap());
        view.blocks.insert([-1, 64, 0], BlockState::parse("minecraft:stone").unwrap());
        view.blocks.insert([0, 63, 0], BlockState::parse("minecraft:dirt").unwrap());
        let at = |state: &str, view: &MemoryBlockView| can_survive(&BlockState::parse(state).unwrap(), [0, 64, 0], view);
        assert_eq!(at("minecraft:wall_torch[facing=east]", &view), Some(true), "the stone to the west holds it");
        view.weak.insert([-1, 64, 0]);
        assert_eq!(at("minecraft:wall_torch[facing=east]", &view), Some(false), "a weak wall does not");
        view.weak.remove(&[-1, 64, 0]);
        // A crop needs light 8 or brighter, read without sky darkening.
        view.light.insert([0, 64, 0], 5);
        assert_eq!(at("minecraft:wheat", &view), Some(false), "too dark for a crop");
        view.light.insert([0, 64, 0], 15);
        assert_eq!(at("minecraft:wheat", &view), Some(true));
        // A mushroom needs light below 13 and solid ground under it.
        view.light.insert([0, 64, 0], 15);
        assert_eq!(at("minecraft:brown_mushroom", &view), Some(false), "too bright for a mushroom");
        view.light.insert([0, 64, 0], 10);
        assert_eq!(at("minecraft:brown_mushroom", &view), Some(true), "dim light over solid dirt");
        // A carpet needs anything below it that is not air.
        assert_eq!(at("minecraft:white_carpet", &view), Some(true), "dirt below");
        view.blocks.remove(&[0, 63, 0]);
        assert_eq!(at("minecraft:white_carpet", &view), Some(false), "air below");
    }

    #[test]
    fn survival_follows_halves_and_heads() {
        let mut view = survival_view();
        view.blocks.insert([0, 63, 0], BlockState::parse("minecraft:oak_door[half=lower]").unwrap());
        let at = |state: &str, view: &MemoryBlockView| can_survive(&BlockState::parse(state).unwrap(), [0, 64, 0], view);
        assert_eq!(at("minecraft:oak_door[half=lower]", &view), Some(true), "sturdy dirt below the lower half");
        assert_eq!(at("minecraft:oak_door[half=upper]", &view), Some(true), "the lower half below the upper");
        view.blocks.insert([0, 63, 0], BlockState::parse("minecraft:air").unwrap());
        assert_eq!(at("minecraft:oak_door[half=upper]", &view), Some(false), "no lower half below the upper");
        // A piston head needs its extended base behind it, facing the same way.
        view.blocks.insert([0, 64, 1], BlockState::parse("minecraft:piston[facing=north,extended=true]").unwrap());
        assert_eq!(at("minecraft:piston_head[facing=north]", &view), Some(true));
        view.blocks.insert([0, 64, 1], BlockState::parse("minecraft:piston[facing=south,extended=true]").unwrap());
        assert_eq!(at("minecraft:piston_head[facing=north]", &view), Some(false), "the base faces the other way");
    }

    #[test]
    fn survival_lets_a_registered_behaviour_answer_first() {
        let mut view = survival_view();
        view.placement_facts.insert("example:stubborn".to_owned(), survival_facts(&["Block"]));
        BlockBehaviors::register(BlockBehavior::new(id("example:stubborn")).can_survive(|_| false));
        assert_eq!(can_survive(&BlockState::parse("example:stubborn").unwrap(), [0, 64, 0], &view), Some(false));
        BlockBehaviors::register(BlockBehavior::new(id("example:stubborn")).can_survive(|_| true));
        assert_eq!(can_survive(&BlockState::parse("example:stubborn").unwrap(), [0, 64, 0], &view), Some(true));
    }

    #[test]
    fn survival_reports_unknown_rather_than_refusing_fire_scaffolding_and_snow() {
        let mut view = survival_view();
        view.blocks.insert([0, 63, 0], BlockState::parse("minecraft:stone").unwrap());
        let at = |state: &str, view: &MemoryBlockView| can_survive(&BlockState::parse(state).unwrap(), [0, 64, 0], view);
        assert_eq!(at("minecraft:fire", &view), Some(true), "a sturdy face below lights it");
        view.blocks.remove(&[0, 63, 0]);
        assert_eq!(at("minecraft:fire", &view), None, "without the face the fire odds would decide");
        assert_eq!(at("minecraft:scaffolding", &view), Some(false), "nothing below and nothing beside it");
        view.blocks.insert([0, 63, 0], BlockState::parse("minecraft:stone").unwrap());
        assert_eq!(at("minecraft:scaffolding", &view), Some(true), "sturdy ground holds it at distance 0");
        assert_eq!(at("minecraft:snow", &view), None, "a snow layer needs the below block's full up face");
        view.state_tags.insert("minecraft:stone".to_owned(), ["minecraft:cannot_support_snow_layer".to_owned()].into_iter().collect());
        assert_eq!(at("minecraft:snow", &view), Some(false), "the tag refuses it outright");
    }

    #[test]
    fn survival_checks_chorus_and_cane_by_their_neighbours() {
        let mut view = survival_view();
        view.blocks.insert([0, 63, 0], BlockState::parse("minecraft:end_stone").unwrap());
        let at = |state: &str, view: &MemoryBlockView| can_survive(&BlockState::parse(state).unwrap(), [0, 64, 0], view);
        assert_eq!(at("minecraft:chorus_flower", &view), Some(false), "end stone below is neither chorus nor empty");
        view.blocks.insert([0, 63, 0], BlockState::parse("minecraft:chorus_plant").unwrap());
        assert_eq!(at("minecraft:chorus_flower", &view), Some(true));
        // Sugar cane on its own block survives; on sand it needs a tagged neighbour below.
        view.blocks.insert([0, 63, 0], BlockState::parse("minecraft:sugar_cane").unwrap());
        assert_eq!(at("minecraft:sugar_cane", &view), Some(true));
        view.blocks.insert([0, 63, 0], BlockState::parse("minecraft:sand").unwrap());
        view.state_tags.insert("minecraft:sand".to_owned(), ["minecraft:supports_sugar_cane".to_owned(), "minecraft:supports_sugar_cane_adjacently".to_owned()].into_iter().collect());
        assert_eq!(at("minecraft:sugar_cane", &view), None, "no tagged neighbour beside the sand yet");
        view.blocks.insert([1, 63, 0], BlockState::parse("minecraft:sand").unwrap());
        assert_eq!(at("minecraft:sugar_cane", &view), Some(true), "tagged sand beside the sand below");
    }

    fn survival_block_facts(block: &str) -> SurvivalFacts {
        match block {
            "minecraft:stone" => SurvivalFacts { solid: true, sturdy_center: 0b111111, sturdy_rigid: 0b111111, full_faces: 0b111111, nonempty_faces: 0b111111, fluid_empty: true, ..Default::default() },
            "minecraft:glass" => SurvivalFacts { fluid_empty: true, ..Default::default() },
            "minecraft:air" => SurvivalFacts { fluid_empty: true, ..Default::default() },
            "minecraft:oak_log" => SurvivalFacts { solid: true, sturdy_center: 0b111111, sturdy_rigid: 0b111111, full_faces: 0b111111, nonempty_faces: 0b111111, ignite_odds: 5, fluid_empty: true, ..Default::default() },
            "minecraft:water" => SurvivalFacts { liquid: true, fluid_supports_lily_pad: true, fluid_supports_frogspawn: true, fluid_supports_sugar_cane_adjacently: true, ..Default::default() },
            "minecraft:oak_slab" => SurvivalFacts { full_faces: 0b000001, nonempty_faces: 0b111111, fluid_empty: true, ..Default::default() },
            "minecraft:melon_stem" => SurvivalFacts { support_blocks: vec!["minecraft:farmland".to_owned()], ..Default::default() },
            _ => SurvivalFacts::default(),
        }
    }

    fn survival_view_with_facts() -> MemoryBlockView {
        let mut view = survival_view();
        for block in ["minecraft:stone", "minecraft:glass", "minecraft:air", "minecraft:oak_log", "minecraft:water", "minecraft:oak_slab", "minecraft:melon_stem"] {
            view.survival_facts.insert(block.to_owned(), survival_block_facts(block));
        }
        view.placement_facts.insert("minecraft:white_banner".to_owned(), survival_facts(&["BannerBlock", "Block"]));
        view.placement_facts.insert("minecraft:oak_pressure_plate".to_owned(), survival_facts(&["PressurePlateBlock", "BasePressurePlateBlock", "Block"]));
        view.placement_facts.insert("minecraft:rail".to_owned(), survival_facts(&["RailBlock", "BaseRailBlock", "Block"]));
        view.placement_facts.insert("minecraft:vine".to_owned(), survival_facts(&["VineBlock", "Block"]));
        view.placement_facts.insert("minecraft:lily_pad".to_owned(), survival_facts(&["LilyPadBlock", "Block"]));
        view.placement_facts.insert("minecraft:melon_stem".to_owned(), survival_facts(&["StemBlock", "BushBlock", "Block"]));
        view.placement_facts.insert("minecraft:sea_pickle".to_owned(), survival_facts(&["SeaPickleBlock", "BushBlock", "Block"]));
        view
    }

    #[test]
    fn survival_reads_solidity_and_the_center_and_rigid_bits() {
        let mut view = survival_view_with_facts();
        view.blocks.insert([0, 63, 0], BlockState::parse("minecraft:stone").unwrap());
        let at = |state: &str, view: &MemoryBlockView| can_survive(&BlockState::parse(state).unwrap(), [0, 64, 0], view);
        // A standing torch is held by the CENTER bit of the stone below.
        assert_eq!(at("minecraft:torch", &view), Some(true));
        // A banner stands on a solid block.
        assert_eq!(at("minecraft:white_banner", &view), Some(true));
        // A pressure plate takes RIGID or CENTER support; a rail takes RIGID only.
        assert_eq!(at("minecraft:oak_pressure_plate", &view), Some(true));
        assert_eq!(at("minecraft:rail", &view), Some(true));
        view.blocks.insert([0, 63, 0], BlockState::parse("minecraft:glass").unwrap());
        assert_eq!(at("minecraft:torch", &view), Some(false), "glass centres nothing");
        assert_eq!(at("minecraft:white_banner", &view), Some(false), "glass is not solid");
        assert_eq!(at("minecraft:oak_pressure_plate", &view), Some(false));
        assert_eq!(at("minecraft:rail", &view), Some(false));
        // A moving piston's support lives in its block entity: unknown, not the table's no.
        view.blocks.insert([0, 63, 0], BlockState::parse("minecraft:moving_piston[facing=up]").unwrap());
        assert_eq!(at("minecraft:torch", &view), None);
        view.blocks.insert([-1, 64, 0], BlockState::parse("minecraft:moving_piston[facing=east]").unwrap());
        assert_eq!(at("minecraft:wall_torch[facing=east]", &view), None);
    }

    #[test]
    fn survival_reads_shape_faces_fire_odds_and_the_attach_table() {
        let mut view = survival_view_with_facts();
        view.blocks.insert([0, 63, 0], BlockState::parse("minecraft:stone").unwrap());
        view.state_tags.insert("minecraft:stone".to_owned(), BTreeSet::new());
        let at = |state: &str, view: &MemoryBlockView| can_survive(&BlockState::parse(state).unwrap(), [0, 64, 0], view);
        // A snow layer is held by the stone's full up face.
        assert_eq!(at("minecraft:snow[layers=1]", &view), Some(true));
        view.blocks.insert([0, 63, 0], BlockState::parse("minecraft:oak_slab[type=bottom]").unwrap());
        view.state_tags.insert("minecraft:oak_slab".to_owned(), BTreeSet::new());
        assert_eq!(at("minecraft:snow[layers=1]", &view), Some(false), "a bottom slab's up face is not full");
        view.blocks.remove(&[0, 63, 0]);
        // Fire with nothing below survives beside a block that can burn, and only there.
        assert_eq!(at("minecraft:fire", &view), Some(false), "air on every side burns nothing");
        view.blocks.insert([1, 64, 0], BlockState::parse("minecraft:oak_log").unwrap());
        assert_eq!(at("minecraft:fire", &view), Some(true), "the log's ignite odds hold the fire");
        view.blocks.insert([1, 64, 0], BlockState::parse("minecraft:oak_log[waterlogged=true]").unwrap());
        assert_eq!(at("minecraft:fire", &view), Some(false), "a waterlogged log never burns");
        view.blocks.remove(&[1, 64, 0]);
        // A vine keeps the face that the block above still holds.
        view.blocks.insert([0, 65, 0], BlockState::parse("minecraft:stone").unwrap());
        assert_eq!(at("minecraft:vine[up=true]", &view), Some(true));
        view.blocks.remove(&[0, 65, 0]);
        assert_eq!(at("minecraft:vine[up=true]", &view), Some(false), "nothing above holds the face");
    }

    #[test]
    fn survival_reads_fluid_tags_and_support_sets() {
        let mut view = survival_view_with_facts();
        let ground = [0, 63, 0];
        // A lily pad floats on a fluid in its fluid tag, under empty fluid.
        view.blocks.insert(ground, BlockState::parse("minecraft:water[level=0]").unwrap());
        let pad = BlockState::parse("minecraft:lily_pad").unwrap();
        assert_eq!(vegetation_may_place_on(&pad, &BlockState::parse("minecraft:water[level=0]").unwrap(), ground, &view), Some(true));
        view.blocks.insert([0, 64, 0], BlockState::parse("minecraft:water[level=0]").unwrap());
        assert_eq!(vegetation_may_place_on(&pad, &BlockState::parse("minecraft:water[level=0]").unwrap(), ground, &view), Some(false), "fluid above the pad refuses it");
        view.blocks.remove(&[0, 64, 0]);
        // A stem stands only on the blocks of its own support set.
        let stem = BlockState::parse("minecraft:melon_stem").unwrap();
        assert_eq!(vegetation_may_place_on(&stem, &BlockState::parse("minecraft:farmland").unwrap(), ground, &view), Some(true));
        assert_eq!(vegetation_may_place_on(&stem, &BlockState::parse("minecraft:stone").unwrap(), ground, &view), Some(false));
        // A sea pickle takes a ground whose up face is not empty, sturdy or not.
        view.blocks.insert(ground, BlockState::parse("minecraft:oak_slab[type=bottom]").unwrap());
        view.weak.insert(ground);
        let pickle = BlockState::parse("minecraft:sea_pickle").unwrap();
        assert_eq!(vegetation_may_place_on(&pickle, &BlockState::parse("minecraft:oak_slab[type=bottom]").unwrap(), ground, &view), Some(true));
    }

    #[test]
    fn survival_matches_a_piston_head_to_its_own_base() {
        let mut view = survival_view();
        let at = |state: &str, view: &MemoryBlockView| can_survive(&BlockState::parse(state).unwrap(), [0, 64, 0], view);
        view.blocks.insert([0, 64, 1], BlockState::parse("minecraft:sticky_piston[facing=north,extended=true]").unwrap());
        assert_eq!(at("minecraft:piston_head[facing=north,type=sticky]", &view), Some(true), "a sticky head on its sticky base");
        assert_eq!(at("minecraft:piston_head[facing=north]", &view), Some(false), "a plain head does not fit a sticky base");
    }

    #[test]
    fn vegetation_placement_reads_the_support_tags() {
        let mut view = survival_view();
        view.placement_facts.insert("minecraft:wheat".to_owned(), survival_facts(&["CropBlock", "BushBlock", "Block"]));
        let farmland = BlockState::parse("minecraft:farmland").unwrap();
        let ground = [0, 63, 0];
        assert_eq!(vegetation_may_place_on(&BlockState::parse("minecraft:wheat").unwrap(), &farmland, ground, &view), None, "no registry answer for the ground");
        view.state_tags.insert("minecraft:farmland".to_owned(), ["minecraft:supports_crops".to_owned()].into_iter().collect());
        assert_eq!(vegetation_may_place_on(&BlockState::parse("minecraft:wheat").unwrap(), &farmland, ground, &view), Some(true));
        view.state_tags.insert("minecraft:farmland".to_owned(), BTreeSet::new());
        assert_eq!(vegetation_may_place_on(&BlockState::parse("minecraft:wheat").unwrap(), &farmland, ground, &view), Some(false));
        assert_eq!(vegetation_may_place_on(&BlockState::parse("minecraft:stone").unwrap(), &farmland, ground, &view), None, "a stone is no vegetation");
    }
}
