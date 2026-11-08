//! Shared blocks: the platforms. Ported from the original `PlatformBlock`; the shapes, models and loot are content files.
use pluto_lib::{ActionResult, BlockBehavior, BlockBehaviors, BlockState, BlockView, Direction, Identifier, Placement, UseItemOnBlockCallback, World};

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

/// The original's `connected`: whether the block at `pos` (next to the platform in `direction`) is one a platform joins.
fn joins(view: &dyn BlockView, pos: [i32; 3], direction: Direction) -> bool {
    if !view.is_in_tag(pos, "tconstruct:platform_connections") {
        return false;
    }
    let Some(state) = view.block_state(pos) else { return false };
    // a block with a side of its own says whether it is joined
    if let Some(side) = state.property(direction.opposite().name()) {
        return side == "true";
    }
    // fixed to a floor or ceiling, not a wall
    if state.property("face").is_some_and(|face| face != "wall") {
        return false;
    }
    // any facing it has must be this way
    state.property("facing").is_none_or(|facing| facing == direction.name())
}

fn flag(on: bool) -> &'static str {
    if on { "true" } else { "false" }
}

const AGES: [&str; 4] = ["", "exposed_", "weathered_", "oxidized_"];

/// How weathered a block that weathers is (vanilla's `WeatheringCopper`): 0 unaffected to 3 oxidized; None for a block that does not weather
/// (waxed copper, ores, other blocks). Tinkers' copper platforms weather too.
fn weathering_age(block: &Identifier) -> Option<usize> {
    let path = block.path();
    let platform = block.namespace() == "tconstruct" && path.ends_with("copper_platform");
    let vanilla = block.namespace() == "minecraft" && path.contains("copper") && !["ore", "raw", "ingot", "nugget"].iter().any(|word| path.contains(word));
    if !(platform || vanilla) || path.starts_with("waxed_") {
        return None;
    }
    Some(AGES.iter().rposition(|prefix| !prefix.is_empty() && path.starts_with(prefix)).unwrap_or(0))
}

/// The weathering of a copper platform (`WeatheringCopper.onRandomTick`): a small chance each random tick, higher when the copper around is
/// as old or older, and never while a younger piece is within four blocks.
fn weather(c: &mut pluto_lib::TickContext<'_>, age: usize) {
    if age >= 3 || c.world.random_float() >= 0.056_888_89 {
        return;
    }
    let (mut same, mut older) = (0, 0);
    for dx in -4i32..=4 {
        for dy in -4i32..=4 {
            for dz in -4i32..=4 {
                if dx == 0 && dy == 0 && dz == 0 || dx.abs() + dy.abs() + dz.abs() > 4 {
                    continue;
                }
                let at = [c.pos[0] + dx, c.pos[1] + dy, c.pos[2] + dz];
                let Some(other) = c.world.block_state(at).and_then(|s| weathering_age(&s.block)) else { continue };
                if other < age {
                    return;
                }
                if other > age {
                    older += 1;
                } else {
                    same += 1;
                }
            }
        }
    }
    let f = (older + 1) as f32 / (older + same + 1) as f32;
    let chance = f * f * if age == 0 { 0.75 } else { 1.0 };
    if c.world.random_float() < chance {
        if let Ok(next) = Identifier::new("tconstruct", &format!("{}copper_platform", AGES[age + 1])) {
            c.world.set_block_state(c.pos, &BlockState { block: next, properties: c.state.properties.clone() });
        }
    }
}

/// Honeycomb on a copper platform waxes it (`WeatheringPlatformBlock.use`).
fn register_waxing() {
    UseItemOnBlockCallback::EVENT.register(|used| {
        if used.item != "minecraft:honeycomb" {
            return ActionResult::Pass;
        }
        let world = World::new(&used.dimension);
        let Some(old) = world.get_block_state(used.pos) else { return ActionResult::Pass };
        let Some(age) = weathering_age(&old.block).filter(|_| old.block.namespace() == "tconstruct") else { return ActionResult::Pass };
        let Ok(waxed) = Identifier::new("tconstruct", &format!("waxed_{}copper_platform", AGES[age])) else { return ActionResult::Pass };
        if !world.set_block_state(used.pos, &BlockState { block: waxed, properties: old.properties }) {
            return ActionResult::Pass;
        }
        world.play_sound("minecraft:item.honeycomb.wax_on", [f64::from(used.pos[0]) + 0.5, f64::from(used.pos[1]) + 0.5, f64::from(used.pos[2]) + 0.5], 1.0, 1.0);
        if !used.creative {
            used.player.take_item(&used.item, 1);
        }
        ActionResult::Success
    });
}

pub fn register() {
    register_waxing();
    for (name, family) in platforms() {
        let exact = (family != "copper").then(|| name.clone());
        let behavior = BlockBehavior::new(Identifier::new("tconstruct", &name).expect("valid id"));
        let on_place = (family, exact.clone());
        let on_update = (family, exact);
        let age = weathering_age(&Identifier::new("tconstruct", &name).expect("valid id"));
        let mut behavior = behavior;
        if let Some(age) = age {
            behavior = behavior.random_tick(move |c| weather(c, age));
        }
        BlockBehaviors::register(
            behavior
                .state_for_placement(move |c| {
                    let (family, exact) = &on_place;
                    let above = Direction::Up.relative(c.pos);
                    let below = Direction::Down.relative(c.pos);
                    let mut state = c
                        .state
                        .clone()
                        .with("up", flag(vertical_connect(c.view, family, above, exact.as_deref())))
                        .with("down", flag(vertical_connect(c.view, family, below, exact.as_deref()) || c.view.is_face_sturdy(below, Direction::Up)));
                    for side in [Direction::North, Direction::South, Direction::West, Direction::East] {
                        state = state.with(side.name(), flag(joins(c.view, side.relative(c.pos), side)));
                    }
                    Placement::State(state)
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
                        side => connected(&c.state, side.name(), joins(c.view, c.neighbour_pos, side)),
                    }
                }),
        );
    }
}
