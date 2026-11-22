//! TinkerWorld's growth: the random-tick and bonemeal behaviour of the slime grass, the saplings, the fungi,
//! the ender propagule and its leaves, the slime vines and the budding crystals, plus the slime grass seeds'
//! use on dirt and vines (SlimeKnights, MIT). Each function is the pinned original's method of the same name:
//! SlimeGrassBlock (spreading, decay, the shared growGrass bonemeal walk), SlimeNyliumBlock (decay and its
//! growGrass call), SaplingBlock with the SlimeTree grower (advanceTree), SlimeFungusBlock, SlimePropaguleBlock,
//! SlimePropaguleLeavesBlock, SlimeVineBlock, BuddingCrystalBlock and SlimeGrassSeedItem.
//!
//! Two translations are worth spelling out. The original's light test (LightEngine.getLightBlockInto of the
//! block above, compared against the max light level) is PlutoLib's `light_passes_up`, whose own documentation
//! names this use ("what lets grass live under a cover"). And the configured features the growers place
//! (`tconstruct:earth_slime_tree` and its kin, TinkerStructures) are worldgen data owned by the worldgen unit:
//! `place_feature` answers false while they are absent, and advanceTree then restores the sapling, exactly as
//! the original does when a feature fails to place.
//!
//! SlimeDirtBlock.canSustainPlant has no PlutoLib counterpart: the API has no ground-side sustain callback
//! (no "sustain" anywhere in pluto-lib's API sources). The original answers true for the SLIME plant type
//! (TinkerWorld.SLIME_PLANT_TYPE, returned by SlimeSaplingBlock's and SlimeTallGrassBlock's getPlantType) and
//! for PLAINS. The port's stand-in is data: the host's vegetation support tag `minecraft:supports_vegetation`
//! lists exactly the four slime dirts in the content, which is what the host consults when vegetation checks
//! its ground; the flora's own survival sets are implemented below (plant_can_survive) from the original's
//! mayPlaceOn overrides instead. SlimePropaguleBlock.updateShape is likewise not registered: its waterlogged
//! tick scheduling is the mangrove propagule template's own fluid handling in the host, and its upward-facing
//! removal is the host applying the can_survive answer registered here when the block above changes.
use pluto_lib::{ActionResult, BlockBehavior, BlockBehaviors, BlockEditor, BlockPos, BlockState, BlockView, BonemealContext, Direction, Identifier, Placement, PlacementContext, ShapeUpdateContext, StackData, SurviveContext, TickContext, UseItemOnBlockCallback, UseItemOnBlockContext, World};

/// DirtType.values(), in the original's order.
const DIRT_TYPES: [&str; 5] = ["earth", "sky", "ichor", "ender", "vanilla"];
/// FoliageType.values(), in the original's order.
const FOLIAGE_TYPES: [&str; 5] = ["earth", "sky", "ichor", "ender", "blood"];
/// FoliageType.isNether is false: the grass is SlimeGrassBlock and bonemeals without spreading.
const OVERWORLD_FOLIAGE: [&str; 3] = ["earth", "sky", "ender"];
/// FoliageType.isNether is true: the grass is SlimeNyliumBlock and the "sapling" is SlimeFungusBlock.
const NETHER_FOLIAGE: [&str; 2] = ["ichor", "blood"];
/// The geode types of TinkerWorld.registerGeode.
const GEODES: [&str; 4] = ["earth", "sky", "ichor", "ender"];
/// Direction.Plane.HORIZONTAL in the game's order.
const HORIZONTAL: [Direction; 4] = [Direction::North, Direction::East, Direction::South, Direction::West];

fn above(pos: BlockPos) -> BlockPos { [pos[0], pos[1] + 1, pos[2]] }
fn below(pos: BlockPos) -> BlockPos { [pos[0], pos[1] - 1, pos[2]] }
fn bool_str(value: bool) -> &'static str { if value { "true" } else { "false" } }

/// A tconstruct block's default state.
fn block_state(name: &str) -> BlockState {
    BlockState::parse(&format!("tconstruct:{name}")).expect("a fixed block id")
}

/// TinkerWorld.allDirt: the block a dirt type is.
fn dirt_state_of(dirt: &str) -> BlockState {
    if dirt == "vanilla" { BlockState::parse("minecraft:dirt").expect("a fixed block id") } else { block_state(&format!("{dirt}_slime_dirt")) }
}

/// TinkerWorld.slimeGrass: the grass block of a dirt and a foliage type.
fn grass_state_of(foliage: &str, dirt: &str) -> BlockState { block_state(&format!("{foliage}_{dirt}_slime_grass")) }

/// The (foliage, dirt) of a slime grass block id, if it is one.
fn grass_parts(block: &Identifier) -> Option<(&'static str, &'static str)> {
    let stem = block.path().strip_suffix("_slime_grass")?;
    let (foliage, dirt) = stem.split_once('_')?;
    let foliage = FOLIAGE_TYPES.iter().find(|name| **name == foliage)?;
    let dirt = DIRT_TYPES.iter().find(|name| **name == dirt)?;
    Some((*foliage, *dirt))
}

/// SlimeGrassBlock.getDirtState: the dirt under a grass state, vanilla dirt for anything else.
pub(crate) fn dirt_state(state: &BlockState) -> BlockState {
    match grass_parts(&state.block) {
        Some((_, dirt)) => dirt_state_of(dirt),
        None => dirt_state_of("vanilla"),
    }
}

/// SlimeGrassBlock.getStateFromDirt: the grass of the given foliage on a dirt state, if it is a dirt.
pub(crate) fn state_from_dirt(dirt: &BlockState, foliage: &str) -> Option<BlockState> {
    let dirt = DIRT_TYPES.iter().find(|name| dirt_state_of(name).block == dirt.block)?;
    Some(grass_state_of(foliage, dirt))
}

/// Whether a state is one of TinkerWorld.allDirt's blocks.
fn is_all_dirt(state: &BlockState) -> bool {
    DIRT_TYPES.iter().any(|name| dirt_state_of(name).block == state.block)
}

/// The ground SlimeSaplingBlock.mayPlaceOn and SlimeTallGrassBlock.mayPlaceOn accept: the four slime dirts
/// (TinkerWorld.slimeDirt) and every slime grass block (the five per-dirt EnumObjects).
fn is_slime_ground(block: &Identifier) -> bool {
    if grass_parts(block).is_some() { return true; }
    DIRT_TYPES[..4].iter().any(|name| dirt_state_of(name).block == *block)
}

/// SlimeGrassBlock.isValidPos: a single snow layer above is fine, a full fluid above is not, otherwise the
/// cover must hold the light back (see the module note on light_passes_up).
pub(crate) fn valid_pos(view: &dyn BlockView, pos: BlockPos) -> bool {
    let above_pos = above(pos);
    if let Some(above_state) = view.block_state(above_pos) {
        if above_state.block.to_string() == "minecraft:snow" && above_state.property("layers") == Some("1") {
            return true;
        }
    }
    if view.is_fluid_full(above_pos) {
        return false;
    }
    view.light_passes_up(pos)
}

/// SlimeGrassBlock.canSpread.
fn can_spread(view: &dyn BlockView, pos: BlockPos) -> bool {
    valid_pos(view, pos) && !view.is_water(above(pos))
}

/// SlimeGrassBlock.randomTick.
pub(crate) fn grass_random_tick(c: &mut TickContext<'_>) {
    let Some((foliage, _)) = grass_parts(&c.state.block) else { return };
    if !c.world.is_area_loaded(c.pos, 3) {
        return;
    }
    if !valid_pos(c.world, c.pos) {
        let dirt = dirt_state(&c.state);
        c.world.set_block_state(c.pos, &dirt);
    } else if c.world.brightness(above(c.pos)) >= 9 {
        for _ in 0..4 {
            let target = [c.pos[0] + c.world.random_int(3) - 1, c.pos[1] + c.world.random_int(5) - 3, c.pos[2] + c.world.random_int(3) - 1];
            let Some(target_state) = c.world.block_state(target) else { continue };
            if let Some(new_state) = state_from_dirt(&target_state, foliage) {
                if can_spread(c.world, target) {
                    let snowy = c.world.block_state(above(target)).is_some_and(|state| state.block.to_string() == "minecraft:snow");
                    let new_state = new_state.with("snowy", bool_str(snowy));
                    c.world.set_block_state(target, &new_state);
                }
            }
        }
    }
}

/// SlimeNyliumBlock.randomTick: no spreading and no area check, just the light test (its isDarkEnough is the
/// same LightEngine expression as isValidPos's fallback).
pub(crate) fn nylium_random_tick(c: &mut TickContext<'_>) {
    if !c.world.light_passes_up(c.pos) {
        let dirt = dirt_state(&c.state);
        c.world.set_block_state(c.pos, &dirt);
    }
}

/// SlimeGrassBlock.growGrass, shared by the grass (validBase `tconstruct:slimy_grass`, no sapling, no
/// spreading) and the nylium (`tconstruct:slimy_nylium`, sapling, spreading).
pub(crate) fn grow_grass(world: &mut dyn BlockEditor, pos: BlockPos, foliage: &str, valid_base: &str, include_sapling: bool, spread: bool) {
    let up = above(pos);
    'main: for i in 0..128 {
        let mut target = up;
        for _ in 0..(i / 16) {
            let dx = world.random_int(3) - 1;
            let dy = (world.random_int(3) - 1) * world.random_int(3) / 2;
            let dz = world.random_int(3) - 1;
            target = [target[0] + dx, target[1] + dy, target[2] + dz];
            let below_pos = below(target);
            if world.is_full_block(target) {
                continue 'main;
            }
            if spread {
                if let Some(below_state) = world.block_state(below_pos) {
                    if is_all_dirt(&below_state) {
                        if let Some(grass) = state_from_dirt(&below_state, foliage) {
                            world.set_block_state(below_pos, &grass);
                        }
                        continue 'main;
                    }
                }
            }
            if !world.is_in_tag(below_pos, valid_base) {
                continue 'main;
            }
        }
        if world.is_air(target) {
            let roll = world.random_int(32);
            let plant = if roll == 0 && include_sapling {
                block_state(&format!("{foliage}_slime_sapling"))
            } else if roll < 6 {
                block_state(&format!("{foliage}_slime_fern"))
            } else {
                block_state(&format!("{foliage}_slime_tall_grass"))
            };
            if plant_survives(&plant, world, target) {
                world.set_block_state(target, &plant);
            }
        }
    }
}

/// The original's plantState.canSurvive in growGrass: the fungi answer with the slimy soil tag
/// (SlimeFungusBlock.mayPlaceOn), the saplings and flora with the slime ground set.
fn plant_survives(plant: &BlockState, view: &dyn BlockView, target: BlockPos) -> bool {
    let below_pos = below(target);
    if NETHER_FOLIAGE.iter().any(|name| plant.block.to_string() == format!("tconstruct:{name}_slime_sapling")) {
        return view.is_in_tag(below_pos, "tconstruct:slimy_soil");
    }
    view.block_state(below_pos).is_some_and(|state| is_slime_ground(&state.block))
}

/// SlimeTree.getConfiguredFeature: the feature a foliage's grower places (TinkerStructures' keys).
fn tree_feature(world: &mut dyn BlockEditor, foliage: &str) -> &'static str {
    match foliage {
        "earth" => "tconstruct:earth_slime_tree",
        "sky" => "tconstruct:sky_slime_tree",
        "ender" => if world.random_float() < 0.85 { "tconstruct:ender_slime_tree_tall" } else { "tconstruct:ender_slime_tree" },
        "blood" => "tconstruct:blood_slime_fungus",
        _ => "tconstruct:ichor_slime_fungus",
    }
}

/// SaplingBlock.advanceTree with AbstractTreeGrower.growTree: stage 0 advances, otherwise the position is
/// left holding its fluid (water when waterlogged, air otherwise) while the feature tries to place, and
/// restored when it cannot.
pub(crate) fn advance_tree(world: &mut dyn BlockEditor, state: &BlockState, pos: BlockPos, foliage: &str) {
    if state.property("stage") != Some("1") {
        world.set_block_state_with_flags(pos, &state.clone().with("stage", "1"), 4);
        return;
    }
    let feature = tree_feature(world, foliage);
    let fluid_block = if state.property("waterlogged") == Some("true") {
        BlockState::parse("minecraft:water").expect("a fixed block id")
    } else {
        BlockState::parse("minecraft:air").expect("a fixed block id")
    };
    world.set_block_state_with_flags(pos, &fluid_block, 4);
    if !world.place_feature(feature, pos) {
        world.set_block_state_with_flags(pos, state, 4);
    }
}

/// SlimePropaguleBlock.randomTick: a standing propagule grows like a sapling (without the light check), a
/// hanging one ages until it is mature.
pub(crate) fn propagule_random_tick(c: &mut TickContext<'_>) {
    if c.state.property("hanging") == Some("true") {
        if c.state.property("age") != Some("4") {
            let next = cycle_age(&c.state);
            c.world.set_block_state_with_flags(c.pos, &next, 2);
        }
    } else if c.world.random_int(7) == 0 {
        let state = c.state.clone();
        advance_tree(c.world, &state, c.pos, "ender");
    }
}

/// MangrovePropaguleBlock's AGE cycle: 0 to 4, then back to 0 (the callers only cycle below 4).
fn cycle_age(state: &BlockState) -> BlockState {
    let age: i32 = state.property("age").and_then(|value| value.parse().ok()).unwrap_or(0);
    state.clone().with("age", &((age + 1) % 5).to_string())
}

/// SlimePropaguleBlock.canSurvive: hanging propagules hold to slimy leaves above; standing ones use
/// SlimeSaplingBlock's ground (the slime ground set, through BushBlock.canSurvive).
pub(crate) fn propagule_survives(c: &SurviveContext<'_>) -> bool {
    if c.state.property("hanging") == Some("true") {
        return c.view.is_in_tag(above(c.pos), "tconstruct:slimy_leaves");
    }
    c.view.block_state(below(c.pos)).is_some_and(|state| is_slime_ground(&state.block))
}

/// SlimePropaguleBlock.getStateForPlacement: placed propagules are mature and take the water they are in.
pub(crate) fn propagule_placement(c: &PlacementContext<'_>) -> Placement {
    Placement::State(c.state.clone().with("waterlogged", bool_str(c.in_water)).with("age", "4"))
}

/// SlimePropaguleBlock.performBonemeal (and its isBonemealSuccess split): hanging propagules age, standing
/// ones grow as saplings do.
pub(crate) fn propagule_bonemeal_perform(c: &mut TickContext<'_>) {
    if c.state.property("hanging") == Some("true") && c.state.property("age") != Some("4") {
        let next = cycle_age(&c.state);
        c.world.set_block_state_with_flags(c.pos, &next, 2);
    } else {
        let state = c.state.clone();
        advance_tree(c.world, &state, c.pos, "ender");
    }
}

/// SlimePropaguleLeavesBlock.performBonemeal: a young hanging propagule appears below the leaves, flags 2.
pub(crate) fn propagule_leaves_bonemeal(c: &mut TickContext<'_>) {
    let propagule = block_state("ender_slime_sapling").with("hanging", "true").with("age", "0");
    c.world.set_block_state_with_flags(below(c.pos), &propagule, 2);
}

/// VineBlock.isAcceptableNeighbour as PlutoLib phrases it: the neighbour's far face must be sturdy.
fn is_acceptable_neighbour(view: &dyn BlockView, pos: BlockPos, face: Direction) -> bool {
    view.is_face_sturdy(pos, face.opposite())
}

/// Whether a vine state carries the given face (VineBlock's boolean face properties).
fn vine_face(state: &BlockState, face: Direction) -> bool {
    state.property(face.name()) == Some("true")
}

/// SlimeVineBlock.hasSides: any of the five face properties (down has none).
fn has_sides(state: &BlockState) -> bool {
    vine_face(state, Direction::Up) || HORIZONTAL.iter().any(|face| vine_face(state, *face))
}

/// SlimeVineBlock.hasNoHorizontalSides.
fn has_no_horizontal_sides(state: &BlockState) -> bool {
    !HORIZONTAL.iter().any(|face| vine_face(state, *face))
}

/// SlimeVineBlock.hasAttachment: a side holds when its neighbour is acceptable, or when the block above is
/// slimy leaves, or when the vine above carries the same side and has not ended.
fn has_attachment(view: &dyn BlockView, pos: BlockPos, vine: &Identifier, side: Direction) -> bool {
    if side == Direction::Down {
        return false;
    }
    if is_acceptable_neighbour(view, side.relative(pos), side) {
        return true;
    }
    let Some(up_state) = view.block_state(above(pos)) else { return false };
    if view.is_in_tag(above(pos), "tconstruct:slimy_leaves") {
        return true;
    }
    side != Direction::Up && up_state.block == *vine && vine_face(&up_state, side) && up_state.property("stage") != Some("end")
}

/// SlimeVineBlock.updateConnections: each carried face is rechecked against its support.
pub(crate) fn vine_update_connections(state: &BlockState, view: &dyn BlockView, pos: BlockPos) -> BlockState {
    let mut updated = state.clone();
    if vine_face(&updated, Direction::Up) {
        let holds = is_acceptable_neighbour(view, above(pos), Direction::Up);
        updated = updated.with("up", bool_str(holds));
    }
    for face in HORIZONTAL {
        if vine_face(&updated, face) {
            let holds = has_attachment(view, pos, &state.block, face);
            updated = updated.with(face.name(), bool_str(holds));
        }
    }
    updated
}

/// SlimeVineBlock.canSurvive.
pub(crate) fn vine_survives(c: &SurviveContext<'_>) -> bool {
    has_sides(&vine_update_connections(&c.state, c.view, c.pos))
}

/// SlimeVineBlock.updateShape: downward updates pass through untouched (the superclass answer), the rest
/// recompute their connections and vanish when nothing holds them.
pub(crate) fn vine_update_shape(c: &ShapeUpdateContext<'_>) -> Option<BlockState> {
    if c.direction == Direction::Down {
        return Some(c.state.clone());
    }
    let updated = vine_update_connections(&c.state, c.view, c.pos);
    if has_sides(&updated) {
        Some(updated)
    } else {
        Some(BlockState::parse("minecraft:air").expect("a fixed block id"))
    }
}

/// SlimeVineBlock.getStateForPlacement: the first looking direction (never down) whose side can hold and
/// is not already carried becomes the placed face; an existing vine keeps its state, anything else refuses.
pub(crate) fn vine_placement(c: &PlacementContext<'_>) -> Placement {
    let current = c.view.block_state(c.pos);
    let is_vine = current.as_ref().is_some_and(|state| state.block == c.state.block);
    let mut vine_state = if is_vine { current.expect("checked above") } else { c.state.clone() };
    for direction in c.nearest_looking_directions() {
        if direction == Direction::Down {
            continue;
        }
        if is_vine && vine_face(&vine_state, direction) {
            continue;
        }
        if has_attachment(c.view, c.pos, &c.state.block, direction) {
            vine_state = vine_state.with(direction.name(), "true");
            return Placement::State(vine_state);
        }
    }
    if is_vine { Placement::State(vine_state) } else { Placement::Refuse }
}

/// SlimeVineBlock.freeFloating: no carried side has an acceptable neighbour.
fn free_floating(view: &dyn BlockView, pos: BlockPos, state: &BlockState) -> bool {
    !HORIZONTAL.iter().any(|face| vine_face(state, *face) && is_acceptable_neighbour(view, face.relative(pos), *face))
}

/// VineStage.cycle.
fn next_stage(stage: &str) -> &'static str {
    match stage {
        "start" => "middle",
        "middle" => "end",
        _ => "start",
    }
}

/// SlimeVineBlock.grow: the vine extends one block down into air, advancing its stage when it hangs free
/// for two of its own above or on a coin flip.
pub(crate) fn vine_grow(world: &mut dyn BlockEditor, pos: BlockPos, state: &BlockState) {
    if has_no_horizontal_sides(state) || state.property("stage") == Some("end") {
        return;
    }
    let below_pos = below(pos);
    if !world.is_air(below_pos) {
        return;
    }
    let mut new_state = state.clone();
    if free_floating(world, pos, state) {
        let stage = state.property("stage").unwrap_or("start");
        let mut i = 1;
        while i < 3 {
            let above_state = world.block_state([pos[0], pos[1] + i, pos[2]]);
            match above_state {
                Some(found) if found.block == state.block && found.property("stage").unwrap_or("start") == stage => i += 1,
                _ => break,
            }
        }
        if i > 2 || world.random_int(2) == 0 {
            new_state = new_state.with("stage", next_stage(stage));
        }
    }
    let placed = new_state.with("up", "false");
    world.set_block_state(below_pos, &placed);
}

/// SlimeVineBlock.randomTick: a top piece (no horizontal sides) gains sides under leaves or against valid
/// neighbours; a sided vine grows downward one time in four.
pub(crate) fn vine_random_tick(c: &mut TickContext<'_>) {
    let state = c.state.clone();
    if has_no_horizontal_sides(&state) {
        let on_leaves = c.world.is_in_tag(above(c.pos), "tconstruct:slimy_leaves");
        let mut new_state = state.clone();
        for side in HORIZONTAL {
            if (on_leaves || is_acceptable_neighbour(c.world, side.relative(c.pos), side)) && c.world.random_int(6) == 0 {
                new_state = new_state.with(side.name(), "true");
            }
        }
        if new_state != state {
            c.world.set_block_state(c.pos, &new_state);
        }
    } else if c.world.random_int(4) == 0 {
        vine_grow(c.world, c.pos, &state);
    }
}

/// BuddingAmethystBlock.canClusterGrowAtState, as BuddingCrystalBlock uses it: air, or a full water block.
fn can_cluster_grow_at(state: &BlockState, view: &dyn BlockView, pos: BlockPos) -> bool {
    view.is_air(pos) || state.block.to_string() == "minecraft:water"
}

/// The bud a bud grows into, per geode (GeodeItemObject's buds; a cluster has no next size).
fn next_bud(block: &Identifier, geode: &str) -> Option<String> {
    let path = block.path();
    if path == format!("small_{geode}_slime_crystal_bud") {
        Some(format!("medium_{geode}_slime_crystal_bud"))
    } else if path == format!("medium_{geode}_slime_crystal_bud") {
        Some(format!("large_{geode}_slime_crystal_bud"))
    } else if path == format!("large_{geode}_slime_crystal_bud") {
        Some(format!("{geode}_slime_crystal_cluster"))
    } else {
        None
    }
}

/// BuddingCrystalBlock.randomTick: one time in five, a random direction's neighbour sprouts a small bud or
/// grows a bud one size, facing away from the budding block and keeping the water it sits in.
pub(crate) fn budding_random_tick(c: &mut TickContext<'_>, geode: &str) {
    if c.world.random_int(5) != 0 {
        return;
    }
    let direction = Direction::ALL[c.world.random_int(6) as usize];
    let target = direction.relative(c.pos);
    let Some(target_state) = c.world.block_state(target) else { return };
    let new_block = if can_cluster_grow_at(&target_state, c.world, target) {
        Some(format!("small_{geode}_slime_crystal_bud"))
    } else {
        next_bud(&target_state.block, geode)
    };
    if let Some(name) = new_block {
        let state = block_state(&name).with("facing", direction.name()).with("waterlogged", bool_str(c.world.is_water(target)));
        c.world.set_block_state(target, &state);
    }
}

/// The foliage of a slime grass seed item, if the item is one (TinkerWorld.slimeGrassSeeds).
pub(crate) fn seed_foliage(item: &str) -> Option<&'static str> {
    FOLIAGE_TYPES.iter().copied().find(|name| item == format!("tconstruct:{name}_slime_grass_seeds"))
}

/// SlimeGrassSeedItem.getDirtType: the dirt type a block is, through TinkerWorld.allDirt.
fn seed_dirt(block: &Identifier) -> Option<&'static str> {
    DIRT_TYPES.iter().copied().find(|name| dirt_state_of(name).block == *block)
}

/// SlimeGrassSeedItem.useOn's state choice: vines first (only the sky and ender seeds have vines, and the
/// new vine copies the clicked vine's true faces at the start stage), then grass on any dirt. The sound is
/// the new state's place sound, as the original plays it.
pub(crate) fn seed_conversion(clicked: &BlockState, foliage: &str) -> Option<(BlockState, &'static str)> {
    if clicked.block.to_string() == "minecraft:vine" {
        let vine = match foliage {
            "sky" => Some("sky_slime_vine"),
            "ender" => Some("ender_slime_vine"),
            _ => None,
        };
        if let Some(vine) = vine {
            let mut state = block_state(vine).with("stage", "start");
            for face in [Direction::Up, Direction::North, Direction::South, Direction::West, Direction::East] {
                if clicked.property(face.name()) == Some("true") {
                    state = state.with(face.name(), "true");
                }
            }
            return Some((state, "minecraft:block.grass.place"));
        }
    }
    let dirt = seed_dirt(&clicked.block)?;
    Some((grass_state_of(foliage, dirt), "minecraft:block.slime_block.place"))
}

/// Takes one seed from the stack the player used it from (the original shrinks the item in hand).
fn shrink_seed(used: &UseItemOnBlockContext) {
    for slot in 0..36 {
        let Some(stack) = used.player.stack(slot) else { continue };
        if stack.item.to_string() != used.item {
            continue;
        }
        used.player.update_stack(slot, &StackData {
            item: stack.item.clone(),
            count: stack.count - 1,
            damage: stack.damage,
            max_damage: stack.max_damage,
            custom_data: stack.custom_data.clone(),
            custom_model_data: stack.custom_model_data.clone(),
            enchantments: stack.enchantments.clone(),
        });
        return;
    }
}

/// Registers everything in this module. Called once from the product's hooks.
pub(crate) fn register() {
    let id = |name: &str| Identifier::new("tconstruct", name).expect("a fixed block id");
    // The slime grass: the overworld foliage spreads and decays (SlimeGrassBlock), the nether foliage only
    // decays (SlimeNyliumBlock); both bonemeal through growGrass with their own arguments.
    for foliage in OVERWORLD_FOLIAGE {
        for dirt in DIRT_TYPES {
            BlockBehaviors::register(BlockBehavior::new(id(&format!("{foliage}_{dirt}_slime_grass")))
                .random_tick(grass_random_tick)
                .can_bonemeal(|c: &BonemealContext<'_>| c.view.is_air(above(c.pos)))
                .bonemeal_success(|_| true)
                .on_bonemeal(move |c: &mut TickContext<'_>| grow_grass(c.world, c.pos, foliage, "tconstruct:slimy_grass", false, false)));
        }
    }
    for foliage in NETHER_FOLIAGE {
        for dirt in DIRT_TYPES {
            BlockBehaviors::register(BlockBehavior::new(id(&format!("{foliage}_{dirt}_slime_grass")))
                .random_tick(nylium_random_tick)
                .can_bonemeal(|c: &BonemealContext<'_>| c.view.is_air(above(c.pos)))
                .bonemeal_success(|_| true)
                .on_bonemeal(move |c: &mut TickContext<'_>| grow_grass(c.world, c.pos, foliage, "tconstruct:slimy_nylium", true, true)));
        }
    }
    // The saplings (SlimeSaplingBlock on the SlimeTree growers): SaplingBlock.randomTick asks for light
    // above and one chance in seven before advanceTree.
    for foliage in ["earth", "sky"] {
        BlockBehaviors::register(BlockBehavior::new(id(&format!("{foliage}_slime_sapling")))
            .random_tick(move |c: &mut TickContext<'_>| {
                if c.world.brightness(above(c.pos)) >= 9 && c.world.random_int(7) == 0 {
                    let state = c.state.clone();
                    advance_tree(c.world, &state, c.pos, foliage);
                }
            })
            .can_bonemeal(|_| true)
            .bonemeal_success(|c: &mut TickContext<'_>| c.world.random_float() < 0.45)
            .on_bonemeal(move |c: &mut TickContext<'_>| {
                let state = c.state.clone();
                advance_tree(c.world, &state, c.pos, foliage);
            }));
    }
    // The fungi (SlimeFungusBlock): no random tick, bonemeal gated on slimy soil below.
    for foliage in NETHER_FOLIAGE {
        BlockBehaviors::register(BlockBehavior::new(id(&format!("{foliage}_slime_sapling")))
            .can_bonemeal(|c: &BonemealContext<'_>| c.view.is_in_tag(below(c.pos), "tconstruct:slimy_soil"))
            .bonemeal_success(|c: &mut TickContext<'_>| c.world.random_float() < 0.4)
            .on_bonemeal(move |c: &mut TickContext<'_>| {
                let feature = tree_feature(c.world, foliage);
                c.world.place_feature(feature, c.pos);
            }));
    }
    // The ender propagule (SlimePropaguleBlock).
    BlockBehaviors::register(BlockBehavior::new(id("ender_slime_sapling"))
        .random_tick(propagule_random_tick)
        .can_survive(propagule_survives)
        .state_for_placement(|c: &PlacementContext<'_>| propagule_placement(c))
        .can_bonemeal(|c: &BonemealContext<'_>| c.state.property("hanging") != Some("true") || c.state.property("age") != Some("4"))
        .bonemeal_success(|c: &mut TickContext<'_>| {
            if c.state.property("hanging") == Some("true") {
                c.state.property("age") != Some("4")
            } else {
                c.world.random_float() < 0.45
            }
        })
        .on_bonemeal(propagule_bonemeal_perform));
    // The propagule's leaves (SlimePropaguleLeavesBlock).
    BlockBehaviors::register(BlockBehavior::new(id("ender_slime_leaves"))
        .can_bonemeal(|c: &BonemealContext<'_>| c.view.is_air(below(c.pos)))
        .bonemeal_success(|_| true)
        .on_bonemeal(propagule_leaves_bonemeal));
    // The slime vines (SlimeVineBlock).
    for name in ["sky_slime_vine", "ender_slime_vine"] {
        BlockBehaviors::register(BlockBehavior::new(id(name))
            .random_tick(vine_random_tick)
            .can_survive(vine_survives)
            .update_shape(|c: &ShapeUpdateContext<'_>| vine_update_shape(c))
            .state_for_placement(|c: &PlacementContext<'_>| vine_placement(c)));
    }
    // The budding crystals (BuddingCrystalBlock); W2 registered their piston rule on the same blocks, and
    // registering a behaviour replaces the old one, so the piston rule is restated here.
    for geode in GEODES {
        BlockBehaviors::register(BlockBehavior::new(id(&format!("budding_{geode}_slime_crystal")))
            .random_tick(move |c: &mut TickContext<'_>| budding_random_tick(c, geode))
            .piston_behavior(pluto_lib::PistonBehavior::Destroy));
    }
    // The slime grass seeds (SlimeGrassSeedItem.useOn).
    static SEED_USE: std::sync::Once = std::sync::Once::new();
    SEED_USE.call_once(|| {
        UseItemOnBlockCallback::EVENT.register(|used| {
            let Some(foliage) = seed_foliage(&used.item) else { return ActionResult::Pass };
            let world = World::new(&used.dimension);
            let Some(clicked) = world.get_block_state(used.pos) else { return ActionResult::Pass };
            let Some((new_state, sound)) = seed_conversion(&clicked, foliage) else { return ActionResult::Pass };
            world.set_block_state(used.pos, &new_state);
            world.play_sound(sound, [f64::from(used.pos[0]) + 0.5, f64::from(used.pos[1]) + 0.5, f64::from(used.pos[2]) + 0.5], 1.0, 1.0);
            if !used.creative {
                shrink_seed(used);
            }
            ActionResult::Success
        });
    });
}
