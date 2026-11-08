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
}

/// A [`BlockView`] for tests: the blocks set are sturdy on every face, everything else is air.
#[derive(Default, Debug, Clone)]
pub struct MemoryBlockView {
    /// The blocks set so far.
    pub blocks: BTreeMap<BlockPos, BlockState>,
    /// Positions that hold a block but not a sturdy one (a slab's top, glass).
    pub weak: BTreeSet<BlockPos>,
    /// The tags each position's block is in.
    pub tags: BTreeMap<BlockPos, BTreeSet<String>>,
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
    /// The blocks around.
    pub view: &'a dyn BlockView,
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

/// What one block of the mod does. Build it, then give it to [`BlockBehaviors::register`].
#[derive(Clone)]
pub struct BlockBehavior {
    block: Identifier,
    can_survive: Option<SurviveFn>,
    state_for_placement: Option<PlacementFn>,
    update_shape: Option<ShapeFn>,
    entity_inside: Option<InsideFn>,
}

impl BlockBehavior {
    /// A behaviour for `block` that changes nothing yet.
    pub fn new(block: Identifier) -> Self {
        Self { block, can_survive: None, state_for_placement: None, update_shape: None, entity_inside: None }
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

    /// How a neighbour's change turns the block's state: `Some(state)` becomes the block's state, `None` leaves it. The game asks often
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
    fn directions_turn_and_step_as_the_game_does() {
        assert_eq!(Direction::North.clockwise(), Direction::East);
        assert_eq!(Direction::Up.clockwise(), Direction::Up);
        assert_eq!(Direction::South.relative([1, 1, 1]), [1, 1, 2]);
        assert_eq!(Direction::from_offset(Direction::West.offset()), Some(Direction::West));
        assert!(Direction::ALL.iter().all(|d| d.opposite().opposite() == *d && Direction::parse(d.name()) == Some(*d)));
    }
}
