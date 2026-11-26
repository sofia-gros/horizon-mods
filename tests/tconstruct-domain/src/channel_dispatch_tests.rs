//! #1161 through the real registries: `channel::register` wires the channel's block behaviour,
//! and the dispatch the game's seams call (`pluto_lib::fixtures::placement` / `shape_update`,
//! the same functions the native hooks call) must answer the connected state.
//!
//! The wave-3 fix moved the channel's "is a fluid handler" classification onto
//! `BlockEntityTypeRegistry::is_fluid_handler`; the unit tests in `channel.rs` call the pure
//! functions directly, so a broken registry seam (behaviour not wired, classification source
//! changed) would pass them while the game never connects. These tests go through the seam.
//! The probe tank is registered the way `tank::register` registers the real seared tanks:
//! filler/drainer/reader hooks and no declared tank list, which must still classify as a
//! fluid handler.
use pluto_lib::{
    BlockEntityType, BlockEntityTypeRegistry, BlockState, Direction, Identifier, MemoryBlockView, Placement, PlacementContext, ShapeUpdateContext,
};

fn channel() -> BlockState { BlockState::parse("tconstruct:seared_channel").unwrap() }
fn probe_tank() -> BlockState { BlockState::parse("tconstruct:handler_probe").unwrap() }

fn register_probe_tank() {
    BlockEntityTypeRegistry::register(
        BlockEntityType::new(Identifier::parse("tconstruct:handler_probe").unwrap()).fluid_filler(|_| 0),
    );
}

fn placement_context(state: BlockState, pos: [i32; 3], face: Direction, sneaking: bool, view: &MemoryBlockView) -> PlacementContext<'_> {
    PlacementContext {
        state,
        pos,
        face,
        yaw: 0.0,
        pitch: 0.0,
        sneaking,
        in_water: false,
        item: "tconstruct:seared_channel".to_string(),
        replace_clicked: false,
        view,
    }
}

#[test]
fn placing_a_channel_against_a_tank_through_the_registry_connects_out_to_it() {
    crate::channel::register();
    register_probe_tank();
    assert!(
        BlockEntityTypeRegistry::is_fluid_handler("tconstruct:handler_probe"),
        "a type with a filler hook is a fluid handler even with no declared tanks"
    );
    assert!(
        !BlockEntityTypeRegistry::is_fluid_handler("tconstruct:no_such_block"),
        "an unregistered block is not a fluid handler"
    );

    let mut view = MemoryBlockView::default();
    view.blocks.insert([5, 64, -1], probe_tank());
    // the tank stands north of the new channel; the player clicked its south face
    let placed = pluto_lib::fixtures::placement(
        "tconstruct:seared_channel",
        &placement_context(channel(), [5, 64, 0], Direction::South, false, &view),
    );
    let Placement::State(state) = placed else { panic!("the channel behaviour answers placement with a state") };
    assert_eq!(state.property("north"), Some("out"), "placed against a tank: out toward it (the #1161 failure)");
}

#[test]
fn placing_a_channel_against_a_channel_and_on_a_tank_through_the_registry() {
    crate::channel::register();
    register_probe_tank();
    let mut view = MemoryBlockView::default();
    view.blocks.insert([1, 64, 0], channel());
    view.blocks.insert([0, 63, 0], probe_tank());
    let placed = pluto_lib::fixtures::placement(
        "tconstruct:seared_channel",
        &placement_context(channel(), [0, 64, 0], Direction::West, false, &view),
    );
    let Placement::State(state) = placed else { panic!("the channel behaviour answers placement with a state") };
    assert_eq!(state.property("east"), Some("out"), "placed against a channel: out");
    let placed = pluto_lib::fixtures::placement(
        "tconstruct:seared_channel",
        &placement_context(channel(), [0, 64, 0], Direction::West, true, &view),
    );
    let Placement::State(state) = placed else { panic!("the channel behaviour answers placement with a state") };
    assert_eq!(state.property("east"), Some("in"), "placed against a channel while sneaking: in");
    let placed = pluto_lib::fixtures::placement(
        "tconstruct:seared_channel",
        &placement_context(channel(), [0, 64, 0], Direction::Up, false, &view),
    );
    let Placement::State(state) = placed else { panic!("the channel behaviour answers placement with a state") };
    assert_eq!(state.property("down"), Some("true"), "placed on top of a tank: down connects");
}

#[test]
fn a_neighbour_channel_mirrors_through_the_registry_and_air_drops_the_side() {
    crate::channel::register();
    let view = MemoryBlockView::default();
    let neighbour = channel().with("west", "out");
    let context = ShapeUpdateContext::new(channel(), [1, 64, 0], Direction::East, [2, 64, 0], Some(neighbour), &view);
    let updated = pluto_lib::fixtures::shape_update("tconstruct:seared_channel", &context).expect("the channel behaviour answers shape updates");
    assert_eq!(updated.property("east"), Some("in"), "the neighbour's out is mirrored as our in");

    let state = channel().with("north", "out");
    let air = BlockState::parse("minecraft:air").unwrap();
    let context = ShapeUpdateContext::new(state, [1, 64, 0], Direction::North, [1, 64, -1], Some(air), &view);
    let updated = pluto_lib::fixtures::shape_update("tconstruct:seared_channel", &context).expect("the channel behaviour answers shape updates");
    assert_eq!(updated.property("north"), Some("none"), "a side facing air drops to none");
}
