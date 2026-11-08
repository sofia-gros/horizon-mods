//! The world: the slime vines. Ported from the original `SlimeVineBlock`; the block's look, loot and states are content files.
use pluto_lib::{ActionResult, BlockBehavior, BlockBehaviors, BlockEditor, BlockState, BlockView, Direction, Identifier, Placement, UseItemOnBlockCallback, World};

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
                    Some(if has_sides(&updated) { updated } else { BlockState::parse("minecraft:air").ok()? })
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

const LOGS: &str = "tconstruct:slimy_logs";
const LEAVES_BLOCKS: [&str; 3] = ["earth_slime_leaves", "sky_slime_leaves", "ender_slime_leaves"];

/// How far a neighbour is from a log: 0 for a slime log, its own distance for slime leaves, 7 for anything else (`getDistance`).
fn leaf_distance<V: BlockView + ?Sized>(view: &V, pos: [i32; 3]) -> i32 {
    if view.is_in_tag(pos, LOGS) {
        return 0;
    }
    match view.block_state(pos) {
        Some(s) if s.block.namespace() == "tconstruct" && LEAVES_BLOCKS.contains(&s.block.path()) => s.property("distance").and_then(|d| d.parse().ok()).unwrap_or(7),
        _ => 7,
    }
}

/// The leaves with the distance their six neighbours give (`updateDistance`).
fn update_distance<V: BlockView + ?Sized>(view: &V, state: BlockState, pos: [i32; 3]) -> BlockState {
    let mut distance = 7;
    for direction in Direction::ALL {
        distance = distance.min(leaf_distance(view, direction.relative(pos)) + 1);
        if distance == 1 {
            break;
        }
    }
    state.with("distance", &distance.to_string())
}

pub fn register_leaves() {
    for name in LEAVES_BLOCKS {
        BlockBehaviors::register(
            BlockBehavior::new(Identifier::new("tconstruct", name).expect("valid id"))
                // the original schedules a tick that sets the distance; the distance is set right away, which gives the same result
                .update_shape(|c| Some(update_distance(c.view, c.state.clone(), c.pos)))
                .state_for_placement(|c| Placement::State(update_distance(c.view, c.state.clone().with("persistent", "true"), c.pos))),
        );
    }
}

// ---- slime grass, nylium and the short plants

const FOLIAGE: [&str; 5] = ["earth", "sky", "ichor", "ender", "blood"];
const SOILS: [&str; 5] = ["earth", "sky", "ichor", "ender", "vanilla"];

/// Nether foliage grows on nylium (`FoliageType.isNether`), the rest on grass.
fn is_nether(foliage: &str) -> bool {
    matches!(foliage, "ichor" | "blood")
}

/// The dirt a slime grass block turns back into (`getDirtState`): its own soil.
fn dirt_of(grass: &BlockState) -> Option<BlockState> {
    let (_, rest) = grass.block.path().split_once('_')?;
    let soil = rest.strip_suffix("_slime_grass")?;
    BlockState::parse(&if soil == "vanilla" { "minecraft:dirt".to_owned() } else { format!("tconstruct:{soil}_slime_dirt") }).ok()
}

/// The grass of `foliage` on top of the given dirt, if it is a dirt that grass can grow on (`getStateFromDirt`).
fn grass_on(dirt: &BlockState, foliage: &str) -> Option<BlockState> {
    let soil = match (dirt.block.namespace(), dirt.block.path()) {
        ("minecraft", "dirt") => "vanilla".to_owned(),
        ("tconstruct", path) => path.strip_suffix("_slime_dirt")?.to_owned(),
        _ => return None,
    };
    if !SOILS.contains(&soil.as_str()) {
        return None;
    }
    BlockState::parse(&format!("tconstruct:{foliage}_{soil}_slime_grass")).ok()
}

/// What the short plants (fern, tall grass, sapling) may stand on: the four slime dirts and every slime grass.
fn supports(ground: &BlockState) -> bool {
    if ground.block.namespace() != "tconstruct" {
        return false;
    }
    let path = ground.block.path();
    if let Some(soil) = path.strip_suffix("_slime_dirt") {
        return matches!(soil, "earth" | "sky" | "ichor" | "ender");
    }
    path.strip_suffix("_slime_grass")
        .and_then(|name| name.split_once('_'))
        .is_some_and(|(foliage, soil)| FOLIAGE.contains(&foliage) && SOILS.contains(&soil))
}

/// Whether the position can be slime grass: light must reach it, unless it lies under one layer of snow (`isValidPos`).
fn valid_grass_position<V: BlockView + ?Sized>(view: &V, pos: [i32; 3]) -> bool {
    let above = Direction::Up.relative(pos);
    if let Some(top) = view.block_state(above) {
        if top.block.to_string() == "minecraft:snow" && top.property("layers") == Some("1") {
            return true;
        }
    }
    if view.is_fluid_full(above) {
        return false;
    }
    view.light_passes_up(pos)
}

pub(crate) fn plant_can_stand<V: BlockView + ?Sized>(view: &V, pos: [i32; 3]) -> bool {
    view.block_state(Direction::Down.relative(pos)).is_some_and(|ground| supports(&ground))
}

/// Shared growth from bone meal (`SlimeGrassBlock.growGrass`): scatters ferns, tall grass and perhaps a sapling around `pos`.
fn grow_grass(world: &mut dyn BlockEditor, pos: [i32; 3], valid_base: &str, foliage: &str, include_sapling: bool, spread: bool) {
    let up = Direction::Up.relative(pos);
    'main: for i in 0..128 {
        let mut target = up;
        for _ in 0..(i / 16) {
            let dx = world.random_int(3) - 1;
            let dy = (world.random_int(3) - 1) * world.random_int(3) / 2;
            let dz = world.random_int(3) - 1;
            target = [target[0] + dx, target[1] + dy, target[2] + dz];
            let below = Direction::Down.relative(target);
            // stop under an opaque block
            if world.is_full_block(target) {
                continue 'main;
            }
            // spread the foliage onto dirt, if asked to
            if spread {
                if let Some(dirt) = world.block_state(below) {
                    if let Some(grass) = grass_on(&dirt, foliage) {
                        world.set_block_state(below, &grass);
                        continue 'main;
                    }
                }
            }
            if !world.is_in_tag(below, valid_base) {
                continue 'main;
            }
        }
        if world.is_air(target) {
            let roll = world.random_int(32);
            let plant = if roll == 0 && include_sapling {
                "slime_sapling"
            } else if roll < 6 {
                "slime_fern"
            } else {
                "slime_tall_grass"
            };
            if let Ok(state) = BlockState::parse(&format!("tconstruct:{foliage}_{plant}")) {
                if plant_can_stand(&*world, target) {
                    world.set_block_state(target, &state);
                }
            }
        }
    }
}

pub fn register_grass() {
    for foliage in FOLIAGE {
        let nether = is_nether(foliage);
        for soil in SOILS {
            let name = format!("{foliage}_{soil}_slime_grass");
            let (valid_base, include_sapling, spread) = if nether { ("tconstruct:slimy_nylium", true, true) } else { ("tconstruct:slimy_grass", false, false) };
            BlockBehaviors::register(
                BlockBehavior::new(Identifier::new("tconstruct", &name).expect("valid id"))
                    .random_tick(move |c| {
                        if nether {
                            // nylium only dies when covered
                            if !c.world.light_passes_up(c.pos) {
                                if let Some(dirt) = dirt_of(&c.state) {
                                    c.world.set_block_state(c.pos, &dirt);
                                }
                            }
                            return;
                        }
                        // never load a chunk by touching its edge
                        if !c.world.is_area_loaded(c.pos, 3) {
                            return;
                        }
                        if !valid_grass_position(&*c.world, c.pos) {
                            if let Some(dirt) = dirt_of(&c.state) {
                                c.world.set_block_state(c.pos, &dirt);
                            }
                        } else if c.world.brightness(Direction::Up.relative(c.pos)) >= 9 {
                            for _ in 0..4 {
                                let dx = c.world.random_int(3) - 1;
                                let dy = c.world.random_int(5) - 3;
                                let dz = c.world.random_int(3) - 1;
                                let spot = [c.pos[0] + dx, c.pos[1] + dy, c.pos[2] + dz];
                                let Some(dirt) = c.world.block_state(spot) else { continue };
                                let Some(grass) = grass_on(&dirt, foliage) else { continue };
                                let above = Direction::Up.relative(spot);
                                if valid_grass_position(&*c.world, spot) && !c.world.is_water(above) {
                                    let snowy = c.world.block_state(above).is_some_and(|s| s.block.to_string() == "minecraft:snow");
                                    c.world.set_block_state(spot, &grass.with("snowy", if snowy { "true" } else { "false" }));
                                }
                            }
                        }
                    })
                    .can_bonemeal(|c| c.view.is_air(Direction::Up.relative(c.pos)))
                    .bonemeal_success(|_| true)
                    .on_bonemeal(move |c| grow_grass(&mut *c.world, c.pos, valid_base, foliage, include_sapling, spread)),
            );
        }
        for plant in ["slime_fern", "slime_tall_grass"] {
            BlockBehaviors::register(
                BlockBehavior::new(Identifier::new("tconstruct", &format!("{foliage}_{plant}")).expect("valid id")).can_survive(|c| plant_can_stand(c.view, c.pos)),
            );
        }
    }
}

/// The seeds (`SlimeGrassSeedItem`): used on dirt they make it grass of their foliage, used on vines they make slime vines (sky and ender).
pub fn register_seeds() {
    UseItemOnBlockCallback::EVENT.register(|used| {
        let Some(foliage) = used.item.strip_prefix("tconstruct:").and_then(|name| name.strip_suffix("_slime_grass_seeds")) else {
            return ActionResult::Pass;
        };
        if !FOLIAGE.contains(&foliage) {
            return ActionResult::Pass;
        }
        let world = World::new(&used.dimension);
        let Some(old) = world.get_block_state(used.pos) else { return ActionResult::Pass };
        // try vines first
        let mut new_state = None;
        let mut sound = "minecraft:block.slime_block.place";
        if old.block.to_string() == "minecraft:vine" && matches!(foliage, "sky" | "ender") {
            if let Ok(mut vine) = BlockState::parse(&format!("tconstruct:{foliage}_slime_vine[stage=start]")) {
                for side in ["up", "north", "east", "south", "west"] {
                    if old.property(side) == Some("true") {
                        vine = vine.with(side, "true");
                    }
                }
                new_state = Some(vine);
                sound = "minecraft:block.grass.place";
            }
        }
        // if vines did not succeed, try grass
        let Some(new_state) = new_state.or_else(|| grass_on(&old, foliage)) else { return ActionResult::Pass };
        if !world.set_block_state(used.pos, &new_state) {
            return ActionResult::Pass;
        }
        world.play_sound(sound, [f64::from(used.pos[0]) + 0.5, f64::from(used.pos[1]) + 0.5, f64::from(used.pos[2]) + 0.5], 1.0, 1.0);
        if !used.creative {
            used.player.take_item(&used.item, 1);
        }
        ActionResult::Success
    });
}

/// The budding slime crystals (`BuddingCrystalBlock`): now and then they start a small bud on a side or grow the bud that is there.
pub fn register_crystals() {
    for kind in ["earth", "sky", "ichor", "ender"] {
        BlockBehaviors::register(
            BlockBehavior::new(Identifier::new("tconstruct", &format!("budding_{kind}_slime_crystal")).expect("valid id")).random_tick(move |c| {
                if c.world.random_int(5) != 0 {
                    return;
                }
                let direction = Direction::ALL[c.world.random_int(6) as usize];
                let target = direction.relative(c.pos);
                if !c.world.is_area_loaded(target, 0) {
                    return;
                }
                let Some(old) = c.world.block_state(target) else { return };
                // an empty space or a full block of water takes a small bud (`canClusterGrowAtState`)
                let name = if c.world.is_air(target) || (old.block.to_string() == "minecraft:water" && c.world.is_fluid_full(target)) {
                    Some(format!("tconstruct:small_{kind}_slime_crystal_bud"))
                } else {
                    [("small", "medium"), ("medium", "large"), ("large", "")].iter().find_map(|(from, to)| {
                        (old.block.to_string() == format!("tconstruct:{from}_{kind}_slime_crystal_bud"))
                            .then(|| if to.is_empty() { format!("tconstruct:{kind}_slime_crystal_cluster") } else { format!("tconstruct:{to}_{kind}_slime_crystal_bud") })
                    })
                };
                let Some(Ok(bud)) = name.map(|n| BlockState::parse(&n)) else { return };
                let water = c.world.is_water(target);
                c.world.set_block_state(target, &bud.with("facing", direction.name()).with("waterlogged", if water { "true" } else { "false" }));
            }),
        );
    }
}
