//! The platforms (PlatformBlock; the weathering and waxed copper variants extend it): a frame of four
//! corner posts with a bottom and a top plate, each plate present exactly when the platform does NOT
//! connect that way, and a thin side plate on each connected horizontal side. The six connection
//! booleans are the block's state, computed at placement and kept by neighbour updates, and the
//! declaration's shapes follow them box for box (the port previously declared no shapes at all, so the
//! host collided the frame as a full cube and a player walked into its face — finding #17; the frame is
//! not vanilla scaffolding, it just reads as one once its own collision is declared).
//!
//! Two parts of the original have no seam here and are stated rather than faked:
//! - `getBlockSupportShape` is a full block (things stand on a platform as on a cube); the content
//!   schema has no support-shape table, so support reads the collision shape instead.
//! - the water tick a waterlogged platform schedules on a neighbour change is the host's own business
//!   for a waterlogged block, the same as for the punji, so it is not scheduled here.
use pluto_lib::{
    BlockBehavior, BlockBehaviors, BlockState, BlockView, Direction, Identifier, Placement, PlacementContext,
    ShapeUpdateContext,
};

/// The eleven platforms (TinkerShared / the copper family): the plain metals, the weathering stages
/// and their waxed twins. Each connects vertically only to its own kind (`state.is(this)`).
const PLATFORMS: [&str; 11] = [
    "iron_platform", "gold_platform", "cobalt_platform",
    "copper_platform", "exposed_copper_platform", "weathered_copper_platform", "oxidized_copper_platform",
    "waxed_copper_platform", "waxed_exposed_copper_platform", "waxed_weathered_copper_platform", "waxed_oxidized_copper_platform",
];

/// The tag a neighbour must be in for a platform to plate its side toward it (TinkerTags.Blocks.PLATFORM_CONNECTIONS).
const CONNECTIONS_TAG: &str = "tconstruct:platform_connections";

fn id(name: &str) -> Identifier {
    Identifier::new("tconstruct", name).expect("fixed block id")
}

fn bool_text(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

pub(crate) fn register() {
    for name in PLATFORMS {
        let own = id(name);
        let (for_place, for_update) = (own.clone(), own.clone());
        BlockBehaviors::register(
            BlockBehavior::new(own)
                .state_for_placement(move |context| platform_placement(&for_place, context))
                .update_shape(move |context| platform_update(&for_update, context)),
        );
    }
}

/// PlatformBlock.verticalConnect: the block there is this very platform.
fn vertical_connect(state: &BlockState, own: &Identifier) -> bool {
    state.block == *own
}

/// PlatformBlock.connected, for the neighbour in `direction`: the neighbour must be in the connections
/// tag; a neighbour carrying the side property for the side facing this platform (another platform,
/// a punji) answers with that property; a lever-style block attached to anything but a wall does not
/// connect; a block with a facing connects only when it faces this way.
fn connected(view: &dyn BlockView, pos: [i32; 3], direction: Direction) -> bool {
    let neighbour_pos = direction.relative(pos);
    if !view.is_in_tag(neighbour_pos, CONNECTIONS_TAG) {
        return false;
    }
    let Some(state) = view.block_state(neighbour_pos) else { return false };
    if let Some(side) = state.property(direction.opposite().name()) {
        return side == "true";
    }
    if let Some(face) = state.property("face") {
        if face != "wall" {
            return false;
        }
    }
    state.property("facing").map_or(true, |facing| facing == direction.name())
}

/// Whether the platform at `pos` connects downward: a platform below, or any block whose top face is sturdy.
fn connects_down(view: &dyn BlockView, pos: [i32; 3], own: &Identifier) -> bool {
    let below = Direction::Down.relative(pos);
    view.block_state(below).is_some_and(|state| vertical_connect(&state, own)) || view.is_face_sturdy(below, Direction::Up)
}

/// PlatformBlock.getStateForPlacement: the connections read from the world as it stands, and the
/// waterlogging from the water at the clicked position.
pub(crate) fn platform_placement(own: &Identifier, context: &PlacementContext<'_>) -> Placement {
    let view = context.view;
    let above = Direction::Up.relative(context.pos);
    let mut state = context
        .state
        .clone()
        .with("waterlogged", bool_text(context.in_water))
        .with("up", bool_text(view.block_state(above).is_some_and(|state| vertical_connect(&state, own))))
        .with("down", bool_text(connects_down(view, context.pos, own)));
    for direction in [Direction::North, Direction::East, Direction::South, Direction::West] {
        state = state.with(direction.name(), bool_text(connected(view, context.pos, direction)));
    }
    Placement::State(state)
}

/// PlatformBlock.updateShape: the side that changed is recomputed, every other side keeps its value.
pub(crate) fn platform_update(own: &Identifier, context: &ShapeUpdateContext<'_>) -> Option<BlockState> {
    let mut state = context.state.clone();
    state = match context.direction {
        Direction::Up => {
            let neighbour = context.view.block_state(context.neighbour_pos);
            state.with("up", bool_text(neighbour.is_some_and(|state| vertical_connect(&state, own))))
        }
        Direction::Down => state.with("down", bool_text(connects_down(context.view, context.pos, own))),
        direction => state.with(direction.name(), bool_text(connected(context.view, context.pos, direction))),
    };
    Some(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::MemoryBlockView;

    fn platform(name: &str) -> BlockState {
        BlockState::new(id(name))
    }

    fn placed(view: &MemoryBlockView, in_water: bool) -> BlockState {
        let context = PlacementContext {
            state: platform("copper_platform"),
            pos: [0, 1, 0],
            face: Direction::Up,
            yaw: 0.0,
            pitch: 0.0,
            sneaking: false,
            in_water,
            item: "tconstruct:copper_platform".to_owned(),
            replace_clicked: false,
            view,
        };
        match platform_placement(&id("copper_platform"), &context) {
            Placement::State(state) => state,
            other => panic!("a platform always places, got {other:?}"),
        }
    }

    #[test]
    fn the_platforms_are_the_eleven_metals_and_copper_stages() {
        assert_eq!(PLATFORMS.len(), 11);
        let mut names = PLATFORMS.to_vec();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 11, "no platform is registered twice");
        for name in PLATFORMS {
            assert!(name.ends_with("_platform"), "{name}");
        }
    }

    #[test]
    fn placement_reads_the_neighbours_it_connects_to() {
        let mut view = MemoryBlockView::default();
        // A sturdy block below: down connects (so no bottom plate is drawn or collided).
        view.blocks.insert([0, 0, 0], BlockState::new(Identifier::new("minecraft", "stone").unwrap()));
        // A ladder to the north: in the connections tag, with no side or facing property to refuse.
        let ladder = [0, 1, -1];
        view.blocks.insert(ladder, BlockState::new(Identifier::new("minecraft", "ladder").unwrap()));
        view.tags.insert(ladder, [CONNECTIONS_TAG.to_owned()].into_iter().collect());
        // Another platform to the east whose own west side is off: its side property answers for it.
        let east = [1, 1, 0];
        view.blocks.insert(east, platform("copper_platform").with("west", "false"));
        view.tags.insert(east, [CONNECTIONS_TAG.to_owned()].into_iter().collect());
        let state = placed(&view, true);
        assert_eq!(state.property("down"), Some("true"));
        assert_eq!(state.property("up"), Some("false"), "nothing above to connect to");
        assert_eq!(state.property("north"), Some("true"));
        assert_eq!(state.property("east"), Some("false"), "the neighbour's own side property decides");
        assert_eq!(state.property("south"), Some("false"));
        assert_eq!(state.property("west"), Some("false"));
        assert_eq!(state.property("waterlogged"), Some("true"), "placed in water");
    }

    #[test]
    fn a_platform_above_connects_only_to_its_own_kind() {
        let mut view = MemoryBlockView::default();
        view.blocks.insert([0, 2, 0], platform("copper_platform"));
        assert_eq!(placed(&view, false).property("up"), Some("true"));
        view.blocks.insert([0, 2, 0], platform("iron_platform"));
        assert_eq!(placed(&view, false).property("up"), Some("false"), "verticalConnect is state.is(this)");
    }

    #[test]
    fn an_update_recomputes_only_the_side_that_changed() {
        let start = platform("copper_platform")
            .with("up", "false")
            .with("down", "false")
            .with("north", "false")
            .with("east", "false")
            .with("south", "false")
            .with("west", "false")
            .with("waterlogged", "false");
        // The block below becomes a platform: a change from below sets down, nothing else moves.
        let mut view = MemoryBlockView::default();
        view.blocks.insert([0, 0, 0], platform("copper_platform"));
        let context = ShapeUpdateContext::new(start.clone(), [0, 1, 0], Direction::Down, [0, 0, 0], None, &view);
        let next = platform_update(&id("copper_platform"), &context).expect("an update answers");
        assert_eq!(next.property("down"), Some("true"));
        assert_eq!(next.property("north"), Some("false"));
        // A tagged neighbour appears to the north: a change from there sets north only.
        let ladder = [0, 1, -1];
        view.blocks.insert(ladder, BlockState::new(Identifier::new("minecraft", "ladder").unwrap()));
        view.tags.insert(ladder, [CONNECTIONS_TAG.to_owned()].into_iter().collect());
        let context = ShapeUpdateContext::new(start.clone(), [0, 1, 0], Direction::North, ladder, None, &view);
        let next = platform_update(&id("copper_platform"), &context).expect("an update answers");
        assert_eq!(next.property("north"), Some("true"));
        assert_eq!(next.property("down"), Some("false"), "the down side was not recomputed");
    }
}
