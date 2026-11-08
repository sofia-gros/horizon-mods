//! Shared blocks: the platforms. Ported from the original `PlatformBlock`; the shapes, models and loot are content files.
use pluto_lib::{BlockBehavior, BlockBehaviors, BlockState, BlockView, Direction, Identifier, Placement};

/// Every platform and the family its vertical neighbours must belong to (the copper ones connect across weathering and waxing).
fn platforms() -> Vec<(String, &'static str)> {
    let mut all: Vec<(String, &'static str)> = ["iron", "gold", "cobalt"].iter().map(|m| (format!("{m}_platform"), *m)).collect();
    for prefix in ["", "exposed_", "weathered_", "oxidized_"] {
        all.push((format!("{prefix}copper_platform"), "copper"));
        all.push((format!("waxed_{prefix}copper_platform"), "copper"));
    }
    all
}

fn family_of(block: &str) -> Option<&'static str> {
    platforms().into_iter().find(|(name, _)| name == block).map(|(_, family)| family)
}

/// Whether the platform `family` continues at `pos` (the original's `verticalConnect`).
fn vertical_connect(view: &dyn BlockView, family: &str, pos: [i32; 3], exact: Option<&str>) -> bool {
    view.block_state(pos).is_some_and(|s| {
        s.block.namespace() == "tconstruct"
            && match exact {
                Some(name) => s.block.path() == name,
                None => family_of(s.block.path()) == Some(family),
            }
    })
}

fn flag(on: bool) -> &'static str {
    if on { "true" } else { "false" }
}

pub fn register() {
    let sides = ["false", "true"];
    for (name, family) in platforms() {
        let exact = (family != "copper").then(|| name.clone());
        let behavior = {
            let mut b = BlockBehavior::new(Identifier::new("tconstruct", &name).expect("valid id")).sturdy(false);
            for property in ["down", "up", "north", "south", "west", "east"] {
                b = b.property(property, &sides);
            }
            b
        };
        let on_place = (family, exact.clone());
        let on_update = (family, exact);
        BlockBehaviors::register(
            behavior
                .state_for_placement(move |c| {
                    let (family, exact) = &on_place;
                    let above = Direction::Up.relative(c.pos);
                    let below = Direction::Down.relative(c.pos);
                    Placement::State(
                        c.state
                            .clone()
                            .with("up", flag(vertical_connect(c.view, family, above, exact.as_deref())))
                            .with("down", flag(vertical_connect(c.view, family, below, exact.as_deref()) || c.view.is_face_sturdy(below, Direction::Up))),
                    )
                })
                .update_shape(move |c| {
                    let (family, exact) = &on_update;
                    let connected = |state: &BlockState, key: &str, on: bool| (state.property(key) != Some(flag(on))).then(|| state.clone().with(key, flag(on)));
                    match c.direction {
                        Direction::Up => connected(&c.state, "up", vertical_connect(c.view, family, c.neighbour_pos, exact.as_deref())),
                        Direction::Down => connected(
                            &c.state,
                            "down",
                            vertical_connect(c.view, family, c.neighbour_pos, exact.as_deref()) || c.view.is_face_sturdy(c.neighbour_pos, Direction::Up),
                        ),
                        _ => None,
                    }
                }),
        );
    }
}
