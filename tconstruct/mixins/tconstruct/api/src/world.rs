//! The world: the slime vines. Ported from the original `SlimeVineBlock`; the block's look, loot and states are content files.
use pluto_lib::{BlockBehavior, BlockBehaviors, BlockEditor, BlockState, BlockView, Direction, Identifier, Placement};

const HORIZONTAL: [Direction; 4] = [Direction::North, Direction::East, Direction::South, Direction::West];
const LEAVES: &str = "tconstruct:slimy_leaves";

fn side(direction: Direction) -> &'static str {
    direction.name()
}

fn on(state: &BlockState, key: &str) -> bool {
    state.property(key) == Some("true")
}

fn flag(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

/// Whether the block at `pos` can hold a vine on its `direction`-facing side (vanilla's `isAcceptableNeighbour`).
fn acceptable<V: BlockView + ?Sized>(view: &V, pos: [i32; 3], direction: Direction) -> bool {
    view.is_face_sturdy(pos, direction.opposite())
}

fn has_sides(state: &BlockState) -> bool {
    ["up", "north", "east", "south", "west"].iter().any(|key| on(state, key))
}

fn has_no_horizontal_sides(state: &BlockState) -> bool {
    !HORIZONTAL.iter().any(|d| on(state, side(*d)))
}

/// Whether the vine can stay on its `direction` side (`hasAttachment`).
fn has_attachment<V: BlockView + ?Sized>(view: &V, vine: &str, pos: [i32; 3], direction: Direction) -> bool {
    if direction == Direction::Down {
        return false;
    }
    if acceptable(view, direction.relative(pos), direction) {
        return true;
    }
    let above = Direction::Up.relative(pos);
    if view.is_in_tag(above, LEAVES) {
        return true;
    }
    // otherwise a vine above that holds the same side, and is not the last stage, carries it
    direction != Direction::Up
        && view
            .block_state(above)
            .is_some_and(|up| up.block.to_string() == vine && on(&up, side(direction)) && up.property("stage") != Some("end"))
}

/// The sides that can no longer be held are dropped (`updateConnections`).
fn update_connections<V: BlockView + ?Sized>(view: &V, vine: &str, mut state: BlockState, pos: [i32; 3]) -> BlockState {
    if on(&state, "up") {
        state = state.with("up", flag(acceptable(view, Direction::Up.relative(pos), Direction::Up)));
    }
    for direction in HORIZONTAL {
        if on(&state, side(direction)) {
            state = state.with(side(direction), flag(has_attachment(view, vine, pos, direction)));
        }
    }
    state
}

/// Whether no side of the vine touches a block that holds it (`freeFloating`).
fn free_floating<V: BlockView + ?Sized>(view: &V, state: &BlockState, pos: [i32; 3]) -> bool {
    !HORIZONTAL.iter().any(|d| on(state, side(*d)) && acceptable(view, d.relative(pos), *d))
}

fn next_stage(stage: &str) -> &'static str {
    match stage {
        "start" => "middle",
        "middle" => "end",
        _ => "start",
    }
}

/// Grows the vine one stage downwards (`grow`).
fn grow(world: &mut dyn BlockEditor, vine: &str, pos: [i32; 3], mut state: BlockState) {
    if has_no_horizontal_sides(&state) || state.property("stage") == Some("end") {
        return;
    }
    let below = Direction::Down.relative(pos);
    if !world.is_air(below) {
        return;
    }
    if free_floating(&*world, &state, pos) {
        // force a transition after three vines of the same stage
        let stage = state.property("stage").map(str::to_owned);
        let mut count = 1;
        while count < 3 {
            let above = [pos[0], pos[1] + count, pos[2]];
            let same = world.block_state(above).is_some_and(|s| s.block.to_string() == vine && s.property("stage").map(str::to_owned) == stage);
            if !same {
                break;
            }
            count += 1;
        }
        if count > 2 || world.random_int(2) == 0 {
            state = state.with("stage", next_stage(stage.as_deref().unwrap_or("start")));
        }
    }
    world.set_block_state(below, &state.with("up", "false"));
}

pub fn register() {
    for kind in ["sky", "ender"] {
        let name = format!("{kind}_slime_vine");
        let id = format!("tconstruct:{name}");
        let (survive_id, place_id, update_id, tick_id) = (id.clone(), id.clone(), id.clone(), id.clone());
        BlockBehaviors::register(
            BlockBehavior::new(Identifier::new("tconstruct", &name).expect("valid id"))
                .can_survive(move |c| has_sides(&update_connections(c.view, &survive_id, c.state.clone(), c.pos)))
                .state_for_placement(move |c| {
                    let existing = c.view.block_state(c.pos).filter(|s| s.block.to_string() == place_id);
                    let vine = existing.clone().unwrap_or_else(|| c.state.clone());
                    // try each direction, see if the vine can be placed on that side
                    for direction in c.nearest_looking_directions() {
                        if direction == Direction::Down {
                            continue;
                        }
                        let key = side(direction);
                        let taken = existing.as_ref().is_some_and(|e| on(e, key));
                        if !taken && has_attachment(c.view, &place_id, c.pos, direction) {
                            return Placement::State(vine.with(key, "true"));
                        }
                    }
                    // no side worked: fine only where a vine is already
                    if existing.is_some() { Placement::State(vine) } else { Placement::Refuse }
                })
                .update_shape(move |c| {
                    if c.direction == Direction::Down {
                        return None;
                    }
                    let updated = update_connections(c.view, &update_id, c.state.clone(), c.pos);
                    let result = if has_sides(&updated) { updated } else { BlockState::parse("minecraft:air").ok()? };
                    (result != c.state).then_some(result)
                })
                .random_tick(move |c| {
                    // only a top piece starts by growing down a side
                    if has_no_horizontal_sides(&c.state) {
                        let on_leaves = c.world.is_in_tag(Direction::Up.relative(c.pos), LEAVES);
                        let mut state = c.state.clone();
                        for direction in HORIZONTAL {
                            if (on_leaves || acceptable(&*c.world, direction.relative(c.pos), direction)) && c.world.random_int(6) == 0 {
                                state = state.with(side(direction), "true");
                            }
                        }
                        if state != c.state {
                            c.world.set_block_state(c.pos, &state);
                        }
                    } else if c.world.random_int(4) == 0 {
                        grow(&mut *c.world, &tick_id, c.pos, c.state.clone());
                    }
                }),
        );
    }
}
