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

/// The game's own survival facts for a block: what `canSurvive` needs beyond the class
/// chain — the SOLID flag, the CENTER and RIGID sturdy bits, the collision shape's faces,
/// the fire odds, the fluid tags and the block's own support set. They are read from the
/// game's behaviour tables (horizon-world's `Behaviours`), which are only built while the
/// world rest loads, after the placement facts above are published; the creature tick
/// publishes them from the tables in hand (see `refresh_survival`). As with the dig facts,
/// the table answers by block name and the first state of a block wins.
#[derive(Clone, Debug, PartialEq)]
pub struct SurvivalFacts {
    /// `BlockState.isSolid` (the behaviour table's SOLID flag).
    pub solid: bool,
    /// `isFaceSturdy(.., SupportType.CENTER)` packed by Direction ordinal.
    pub sturdy_center: u8,
    /// `isFaceSturdy(.., SupportType.RIGID)` packed by Direction ordinal.
    pub sturdy_rigid: u8,
    /// `Block.isFaceFull(collision shape, direction)` packed by Direction ordinal;
    /// meaningless when `dynamic_shape` is set.
    pub full_faces: u8,
    /// `!collision shape.getFaceShape(direction).isEmpty()` packed by Direction ordinal;
    /// meaningless when `dynamic_shape` is set.
    pub nonempty_faces: u8,
    /// The collision shape depends on the position or a block entity, so the face bits
    /// do not describe it.
    pub dynamic_shape: bool,
    /// The state holds a fluid of its own (the behaviour table's LIQUID flag).
    pub liquid: bool,
    /// `FireBlock.getIgniteOdds` for the block.
    pub ignite_odds: i32,
    /// The state's own fluid is in the `lava` fluid tag.
    pub fluid_lava: bool,
    /// The state holds no fluid at all.
    pub fluid_empty: bool,
    /// The state's own fluid is in the fluid tag `supports_lily_pad`.
    pub fluid_supports_lily_pad: bool,
    /// The state's own fluid is in the fluid tag `supports_frogspawn`.
    pub fluid_supports_frogspawn: bool,
    /// The state's own fluid is in the fluid tag `supports_sugar_cane_adjacently`.
    pub fluid_supports_sugar_cane_adjacently: bool,
    /// The blocks this block may stand on by its own support set, by name; empty when
    /// the block has no support set.
    pub support_blocks: Arc<[Arc<str>]>,
}

static SURVIVAL_FACTS: RwLock<Vec<(String, SurvivalFacts)>> = RwLock::new(Vec::new());

/// The behaviour tables the survival facts were last read from, by address: the tables
/// are built once per game load, so a new address is the only sign of a rebuild.
static SURVIVAL_SOURCE: RwLock<usize> = RwLock::new(0);

/// Publishes the survival facts from the game's behaviour tables, zipped with the
/// gameplay facts by state: both tables are indexed by the registry's state order (the
/// facts loader refuses any other order), so a state index names the same state in each.
/// Called from the creature tick; a call with the tables already published is a no-op.
pub fn refresh_survival(behaviours: &horizon_world::level::Behaviours, facts: &horizon_client::blocks::BlockFacts) {
    let source = behaviours as *const horizon_world::level::Behaviours as usize;
    if SURVIVAL_SOURCE.read().is_ok_and(|held| *held == source) { return; }
    // The support sets are per block: resolve each one's members to names once, by
    // testing every block the facts name against the set.
    let mut block_names: Vec<&str> = Vec::new();
    for state in &facts.states {
        if block_names.last() != Some(&state.name.as_ref()) { block_names.push(state.name.as_ref()); }
    }
    let mut support_cache: Vec<(horizon_world::level::BlockId, Arc<[Arc<str>]>)> = Vec::new();
    let mut support_for = |behaviours: &horizon_world::level::Behaviours, block: horizon_world::level::BlockId| -> Arc<[Arc<str>]> {
        if let Some((_, names)) = support_cache.iter().find(|(held, _)| *held == block) { return Arc::clone(names); }
        let names: Arc<[Arc<str>]> = match &behaviours.block_data(block).support_tag {
            Some(set) => block_names.iter()
                .filter(|name| behaviours.table.block_by_name(name).is_some_and(|candidate| set.contains(candidate)))
                .map(|name| Arc::from(*name))
                .collect(),
            None => Arc::from([]),
        };
        support_cache.push((block, Arc::clone(&names)));
        names
    };
    let states = facts.states.iter().enumerate().map(|(index, state)| {
        let id = horizon_world::registry::BlockStateId(index as u32);
        let table = behaviours.facts(id);
        let shape = behaviours.shape_info(id);
        let kind = behaviours.fluid(id).kind();
        let mask = kind.mask();
        (state.name.to_string(), SurvivalFacts {
            solid: behaviours.is_solid(id),
            sturdy_center: ((table.sturdy >> 6) & 63) as u8,
            sturdy_rigid: ((table.sturdy >> 12) & 63) as u8,
            full_faces: shape.full_faces,
            nonempty_faces: shape.nonempty_faces,
            dynamic_shape: table.shape < 0,
            liquid: table.flags & horizon_world::level::data::state_flags::LIQUID != 0,
            ignite_odds: behaviours.data(id).ignite_odds,
            fluid_lava: behaviours.fluid_tags.lava & mask != 0,
            fluid_empty: kind == horizon_world::fluid::FluidKind::Empty,
            fluid_supports_lily_pad: behaviours.fluid_tags.supports_lily_pad & mask != 0,
            fluid_supports_frogspawn: behaviours.fluid_tags.supports_frogspawn & mask != 0,
            fluid_supports_sugar_cane_adjacently: behaviours.fluid_tags.supports_sugar_cane_adjacently & mask != 0,
            support_blocks: support_for(behaviours, behaviours.table.block(id)),
        })
    }).collect();
    if let Ok(mut held) = SURVIVAL_FACTS.write() { *held = states; }
    if let Ok(mut held) = SURVIVAL_SOURCE.write() { *held = source; }
}

pub fn survival_for(block: &str) -> Option<SurvivalFacts> {
    SURVIVAL_FACTS.read().ok()?.iter().find(|(name, _)| name == block).map(|(_, facts)| facts.clone())
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
