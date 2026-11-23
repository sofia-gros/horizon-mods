// The per-state gameplay facts of the game's own block table (gameplay-facts.json, decoded by
// horizon-items into BlockFacts), published by the game when its resource tables load, so the
// API crate's dig report can name the block's hardness, its correct-tool requirement and its
// explosion resistance. The three are constant across a block's states in the game's data, so
// the table answers by block name and the first state of a block wins.
use std::sync::RwLock;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DigFacts {
    pub hardness: f32,
    pub requires_tool: bool,
    pub explosion_resistance: f32,
}

static FACTS: RwLock<Vec<(String, DigFacts)>> = RwLock::new(Vec::new());

pub fn publish(states: Vec<(String, DigFacts)>) {
    if let Ok(mut facts) = FACTS.write() { *facts = states; }
}

pub fn for_block(block: &str) -> Option<DigFacts> {
    FACTS.read().ok()?.iter().find(|(name, _)| name == block).map(|(_, facts)| *facts)
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
