// The per-state gameplay facts of the game's own block table (gameplay-facts.json, decoded by
// horizon-items into BlockFacts), published by the game when its resource tables load, so the
// API crate's dig report can name the block's hardness, its correct-tool requirement and its
// explosion resistance. The three are constant across a block's states in the game's data, so
// the table answers by block name and the first state of a block wins.
use std::sync::{Arc, RwLock};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DigFacts {
    pub hardness: f32,
    pub requires_tool: bool,
    pub explosion_resistance: f32,
    /// `isFaceSturdy(.., SupportType.FULL)` packed by Direction ordinal (0 down, 1 up,
    /// 2 north, 3 south, 4 west, 5 east): the one fact here that can differ across a block's
    /// states, so a block answers with its first state, as above.
    pub sturdy_full: u8,
}

static FACTS: RwLock<Vec<(String, DigFacts)>> = RwLock::new(Vec::new());

pub fn publish(states: Vec<(String, DigFacts)>) {
    if let Ok(mut facts) = FACTS.write() { *facts = states; }
}

pub fn for_block(block: &str) -> Option<DigFacts> {
    FACTS.read().ok()?.iter().find(|(name, _)| name == block).map(|(_, facts)| *facts)
}

/// Whether the named block's face in `direction` (a Direction ordinal, 0..=5) is sturdy for
/// the full support type, as the game's table reports it.
pub fn face_sturdy(block: &str, direction: u8) -> Option<bool> {
    if direction > 5 { return None; }
    for_block(block).map(|facts| facts.sturdy_full & (1 << direction) != 0)
}

/// The game's own placement facts for a block: what `canSurvive` needs of a state that is not
/// in the world yet. Published with the dig facts by the same head. The class chain is a
/// property of the block class, so it is constant across a block's states; the two shape flags
/// answer with the block's first state, as the dig facts do.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacementFacts {
    /// The block's class chain (`TransparentBlock`, `LeavesBlock`, ...), most specific first.
    pub classes: Arc<[Arc<str>]>,
    /// `BlockState.isCollisionShapeFullBlock`: the shape a placement's obstruction check tests.
    pub full_collision: bool,
    /// Whether the block renders as a solid cube (`solid_render`).
    pub solid_render: bool,
}

static PLACEMENT_FACTS: RwLock<Vec<(String, PlacementFacts)>> = RwLock::new(Vec::new());

pub fn publish_placement(states: Vec<(String, PlacementFacts)>) {
    if let Ok(mut facts) = PLACEMENT_FACTS.write() { *facts = states; }
}

pub fn placement_for(block: &str) -> Option<PlacementFacts> {
    PLACEMENT_FACTS.read().ok()?.iter().find(|(name, _)| name == block).map(|(_, facts)| facts.clone())
}

static DIG_HEIGHT: RwLock<Option<[i32; 2]>> = RwLock::new(None);

/// The build range of the level the current dig is in, published at the head of every native
/// break-speed call: the dig report is worked out inside that call, so the value always names
/// the dig's own level.
pub fn publish_dig_height(height: [i32; 2]) {
    if let Ok(mut slot) = DIG_HEIGHT.write() { *slot = Some(height); }
}

pub fn dig_height() -> Option<[i32; 2]> {
    DIG_HEIGHT.read().ok().and_then(|slot| *slot)
}
