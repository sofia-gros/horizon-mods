//! The gadgets: the punji sticks. Ported from the original `PunjiBlock` (the block's look, shapes, loot and recipe are content files).
use pluto_lib::{BlockBehavior, BlockBehaviors, BlockState, BlockView, Direction, Identifier, Placement};

const PUNJI: &str = "punji";

/// The direction that is "north" on a punji stuck to the side `facing`.
fn local_north(facing: Direction) -> Direction {
    match facing {
        Direction::Down => Direction::North,
        Direction::Up => Direction::South,
        _ => Direction::Up,
    }
}

/// The direction that is "east" on a punji stuck to the side `facing`.
fn local_east(facing: Direction) -> Direction {
    match facing {
        Direction::Down | Direction::Up => Direction::East,
        side => side.clockwise(),
    }
}

fn facing_of(state: &BlockState) -> Option<Direction> {
    Direction::parse(state.property("facing")?)
}

/// Whether a punji stuck to the same side stands at `target`.
fn is_connected(view: &dyn BlockView, facing: Direction, target: [i32; 3]) -> bool {
    view.block_state(target).is_some_and(|s| s.block.namespace() == "tconstruct" && s.block.path() == PUNJI && facing_of(&s) == Some(facing))
}

fn flag(on: bool) -> &'static str {
    if on { "true" } else { "false" }
}

/// The state with `facing` and all four connections worked out around `pos`.
fn connected(view: &dyn BlockView, state: BlockState, facing: Direction, pos: [i32; 3]) -> BlockState {
    let (north, east) = (local_north(facing), local_east(facing));
    let north_pos = north.relative(pos);
    state
        .with("facing", facing.name())
        .with("north", flag(is_connected(view, facing, north_pos)))
        .with("east", flag(is_connected(view, facing, east.relative(pos))))
        .with("northeast", flag(is_connected(view, facing, east.relative(north_pos))))
        .with("northwest", flag(is_connected(view, facing, east.opposite().relative(north_pos))))
}

/// A punji stays if the block it is stuck to is solid on that face.
fn survives(view: &dyn BlockView, facing: Direction, pos: [i32; 3]) -> bool {
    view.is_face_sturdy(facing.relative(pos), facing.opposite())
}

pub fn register() {
    BlockBehaviors::register(
        BlockBehavior::new(Identifier::new("tconstruct", PUNJI).expect("valid id"))
            .can_survive(|c| facing_of(&c.state).is_some_and(|facing| survives(c.view, facing, c.pos)))
            .state_for_placement(|c| {
                let wanted = c.face.opposite();
                // if the space is invalid, try the other sides
                let side = std::iter::once(wanted)
                    .chain(Direction::ALL.into_iter().filter(|d| *d != wanted))
                    .find(|d| survives(c.view, *d, c.pos));
                match side {
                    Some(facing) => Placement::State(connected(c.view, c.state.clone(), facing, c.pos)),
                    None => Placement::Refuse,
                }
            })
            .update_shape(|c| {
                let facing = facing_of(&c.state)?;
                let updated = connected(c.view, c.state.clone(), facing, c.pos);
                (updated != c.state).then_some(updated)
            }),
    );
}
