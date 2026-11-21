//! World generation of the original's `world/worldgen` packages (SlimeKnights, MIT): the slime tree and slime
//! fungus feature types, the ender trees' leave-vine decorator and extra-root-variants root placer, and the slime
//! island structure with its template piece. The algorithms follow the pinned Java (SlimeTreeFeature,
//! SlimeFungusFeature over the vanilla HugeFungusFeature it extends, IslandStructure / IslandPiece /
//! IslandPlacement, LeaveVineDecorator, ExtraRootVariantPlacer). The geode features are vanilla `minecraft:geode`
//! data and the two ender trees are vanilla `minecraft:tree` data; neither needs a codec here, only the two
//! Tinkers tree components below.
//!
//! Registration is all this module does at load: PlutoLib's worldgen registries connect the codecs to the host's
//! catalog and generation paths. A codec accepts either catalog shape for its configuration: the value itself
//! (26.3's flattened form, which the importer writes) or a `config` object holding it (the vanilla form).

use std::collections::BTreeSet;

use pluto_lib::{BlockEditor, BlockState, ConfiguredFeature, ConfiguredStructure, ConfiguredStructurePiece, Direction, FeatureContext, FeatureTypeRegistry, Identifier, RootPlacement, RootPlacementTypeRegistry, StructureContext, StructureMirror, StructurePiece, StructurePieceTemplate, StructurePieceTypeRegistry, StructureRotation, StructureStart, StructureTypeRegistry, TreeDecorator, TreeDecoratorTypeRegistry, WorldgenBindings};
use pluto_lib::{BlockPos, StructureMarkerContext, StructurePieceWorld};
use serde_json::Value;

fn id(name: &str) -> Identifier {
    Identifier::new("tconstruct", name).unwrap()
}

/// The configuration object of a codec value, in either catalog shape.
fn config_of(value: &Value) -> &Value {
    value.get("config").filter(|config| config.is_object()).unwrap_or(value)
}

fn int_of(config: &Value, key: &str, default: i32) -> Result<i32, String> {
    match config.get(key) {
        None => Ok(default),
        Some(value) => value.as_i64().and_then(|n| i32::try_from(n).ok()).ok_or_else(|| format!("{key} must be an integer")),
    }
}

fn bool_of(config: &Value, key: &str, default: bool) -> bool {
    config.get(key).and_then(Value::as_bool).unwrap_or(default)
}

/// A block state as the data writes it: a plain name, or a `Name`/`Properties` object.
fn state_of(value: &Value) -> Result<BlockState, String> {
    match value {
        Value::String(name) => BlockState::parse(name).map_err(|error| format!("bad block state {name}: {error:?}")),
        Value::Object(_) => {
            let name = value.get("Name").and_then(Value::as_str).ok_or("block state needs a Name")?;
            let mut state = BlockState::parse(name).map_err(|error| format!("bad block state {name}: {error:?}"))?;
            if let Some(properties) = value.get("Properties").and_then(Value::as_object) {
                for (key, property) in properties {
                    let property = property.as_str().ok_or("block state properties must be strings")?;
                    state.properties.insert(key.clone(), property.to_owned());
                }
            }
            Ok(state)
        }
        _ => Err("block state must be a name or an object".to_owned()),
    }
}

fn air() -> BlockState {
    BlockState::parse("minecraft:air").unwrap()
}

fn shift(pos: BlockPos, offset: [i32; 3]) -> BlockPos {
    [pos[0] + offset[0], pos[1] + offset[1], pos[2] + offset[2]]
}

/* State providers (the two kinds the tree and fungus configurations use) */

#[derive(Clone)]
enum StateProvider {
    Simple(BlockState),
    Weighted(Vec<(BlockState, i32)>),
}

fn provider_of(value: &Value) -> Result<StateProvider, String> {
    let kind = value.get("type").and_then(Value::as_str).ok_or("state provider needs a type")?;
    match kind {
        "minecraft:simple_state_provider" => Ok(StateProvider::Simple(state_of(value.get("state").ok_or("simple provider needs a state")?)?)),
        "minecraft:weighted_state_provider" => {
            let entries = value.get("entries").and_then(Value::as_array).ok_or("weighted provider needs entries")?;
            let mut out = Vec::new();
            for entry in entries {
                let weight = entry.get("weight").and_then(Value::as_i64).ok_or("weighted entry needs a weight")?;
                let weight = i32::try_from(weight).map_err(|_| "weighted entry weight out of range".to_owned())?;
                out.push((state_of(entry.get("state").ok_or("weighted entry needs a state")?)?, weight));
            }
            if out.is_empty() {
                return Err("weighted provider is empty".to_owned());
            }
            Ok(StateProvider::Weighted(out))
        }
        other => Err(format!("unsupported state provider {other}")),
    }
}

impl StateProvider {
    fn get_state(&self, world: &mut dyn BlockEditor) -> BlockState {
        match self {
            StateProvider::Simple(state) => state.clone(),
            StateProvider::Weighted(entries) => {
                let total: i32 = entries.iter().map(|(_, weight)| *weight).sum();
                if total <= 0 {
                    return entries[0].0.clone();
                }
                let mut roll = world.random_int(total);
                for (state, weight) in entries {
                    roll -= weight;
                    if roll < 0 {
                        return state.clone();
                    }
                }
                entries[entries.len() - 1].0.clone()
            }
        }
    }
}

/* SlimeTreeFeature */

#[derive(Clone)]
struct SlimeTree {
    trunk: StateProvider,
    leaves: StateProvider,
    vines: StateProvider,
    base_height: i32,
    random_height: i32,
    can_double_height: bool,
    has_vines: bool,
}

fn slime_tree(value: &Value) -> Result<ConfiguredFeature, String> {
    let config = config_of(value);
    let tree = SlimeTree {
        trunk: provider_of(config.get("trunk_provider").ok_or("slime_tree needs trunk_provider")?)?,
        leaves: provider_of(config.get("leaves_provider").ok_or("slime_tree needs leaves_provider")?)?,
        vines: provider_of(config.get("vines_provider").ok_or("slime_tree needs vines_provider")?)?,
        base_height: int_of(config, "base_height", 0)?,
        random_height: int_of(config, "random_height", 0)?,
        can_double_height: bool_of(config, "can_double_height", false),
        has_vines: bool_of(config, "has_vines", false),
        // `planted` is decoded by the original's codec but unused by SlimeTreeFeature.place; it is not read here.
    };
    Ok(ConfiguredFeature::new(move |context| place_tree(&tree, context)))
}

fn air_or_leaves(world: &dyn BlockEditor, pos: BlockPos) -> bool {
    world.is_air(pos) || world.is_in_tag(pos, "minecraft:leaves")
}

fn set_log(world: &mut dyn BlockEditor, tree: &SlimeTree, pos: BlockPos, placed: &mut BTreeSet<BlockPos>) {
    if air_or_leaves(world, pos) {
        let state = tree.trunk.get_state(world);
        world.set_block_state(pos, &state);
        placed.insert(pos);
    }
}

fn set_leaf(world: &mut dyn BlockEditor, tree: &SlimeTree, pos: BlockPos, placed: &mut BTreeSet<BlockPos>) {
    if air_or_leaves(world, pos) {
        let state = tree.leaves.get_state(world);
        world.set_block_state(pos, &state);
        placed.insert(pos);
    }
}

fn place_air(world: &mut dyn BlockEditor, pos: BlockPos, placed: &mut BTreeSet<BlockPos>) {
    if air_or_leaves(world, pos) {
        world.set_block_state(pos, &air());
        placed.insert(pos);
    }
}

fn place_vine(world: &mut dyn BlockEditor, pos: BlockPos, state: BlockState, placed: &mut BTreeSet<BlockPos>) {
    if air_or_leaves(world, pos) {
        world.set_block_state(pos, &state);
        placed.insert(pos);
    }
}

/// SlimeTreeFeature.getRandomizedVine: the provider's state with its faces cleared, then one to three random
/// faces set (draws may repeat a face).
fn randomized_vine(world: &mut dyn BlockEditor, tree: &SlimeTree) -> BlockState {
    let mut state = tree.vines.get_state(world);
    const SIDES: [&str; 4] = ["north", "east", "south", "west"];
    for side in SIDES {
        state.properties.insert(side.to_owned(), "false".to_owned());
    }
    for _ in 0..world.random_int(3) + 1 {
        let side = SIDES[world.random_int(4) as usize];
        state.properties.insert(side.to_owned(), "true".to_owned());
    }
    state
}

fn diamond_layer(world: &mut dyn BlockEditor, tree: &SlimeTree, range: i32, center: BlockPos, placed: &mut BTreeSet<BlockPos>) {
    for x in -range..=range {
        for z in -range..=range {
            if x.abs() + z.abs() <= range {
                set_leaf(world, tree, [center[0] + x, center[1], center[2] + z], placed);
            }
        }
    }
}

fn place_canopy(world: &mut dyn BlockEditor, tree: &SlimeTree, origin: BlockPos, height: i32, placed: &mut BTreeSet<BlockPos>) {
    let top = [origin[0], origin[1] + height, origin[2]];
    for i in 0..4 {
        diamond_layer(world, tree, i + 1, [top[0], top[1] - i, top[2]], placed);
    }
    let mut pos = [top[0], top[1] - 3, top[2]];
    for offset in [[4, 0, 0], [-4, 0, 0], [0, 0, 4], [0, 0, -4]] {
        place_air(world, shift(pos, offset), placed);
    }
    if tree.has_vines {
        for offset in [[1, 0, 1], [1, 0, -1], [-1, 0, 1], [-1, 0, -1]] {
            place_air(world, shift(pos, offset), placed);
        }
    }
    // Drippers, one block below the air ring.
    pos = [pos[0], pos[1] - 1, pos[2]];
    for offset in [[3, 0, 0], [-3, 0, 0], [0, 0, -3], [0, 0, 3]] {
        set_leaf(world, tree, shift(pos, offset), placed);
    }
    if !tree.has_vines {
        for offset in [[1, 0, 1], [-3, 0, 0], [-1, 0, 1], [-1, 0, -1]] {
            set_leaf(world, tree, shift(pos, offset), placed);
        }
    }
    if tree.has_vines {
        pos = [pos[0], pos[1] - 1, pos[2]];
        for offset in [[3, 0, 0], [-3, 0, 0], [0, 0, -3], [0, 0, 3]] {
            let vine = randomized_vine(world, tree).with("up", "true");
            place_vine(world, shift(pos, offset), vine, placed);
        }
        for (x, z) in [(2, 2), (2, -2), (-2, 2), (-2, -2)] {
            let vine = randomized_vine(world, tree);
            place_vine(world, [pos[0] + x, pos[1] + 1, pos[2] + z], vine.clone().with("up", "true"), placed);
            place_vine(world, [pos[0] + x, pos[1], pos[2] + z], vine, placed);
        }
    }
}

/// SlimeTreeFeature.updateLeaves: leaf `distance` values flood out from every placed position (the original's
/// set is shared by the trunk, canopy, air and vine writes, and so is this one).
fn update_leaf_distances(world: &mut dyn BlockEditor, placed: &BTreeSet<BlockPos>) {
    let mut levels: Vec<BTreeSet<BlockPos>> = (0..6).map(|_| BTreeSet::new()).collect();
    for &log in placed {
        for direction in Direction::ALL {
            let neighbour = direction.relative(log);
            if placed.contains(&neighbour) {
                continue;
            }
            if let Some(state) = world.block_state(neighbour) {
                if state.property("distance").is_some() {
                    world.set_block_state(neighbour, &state.with("distance", "1"));
                    levels[0].insert(neighbour);
                }
            }
        }
    }
    for distance in 1..6i32 {
        let current: Vec<BlockPos> = levels[(distance - 1) as usize].iter().copied().collect();
        for pos in current {
            for direction in Direction::ALL {
                let neighbour = direction.relative(pos);
                if levels[(distance - 1) as usize].contains(&neighbour) || levels[distance as usize].contains(&neighbour) {
                    continue;
                }
                if let Some(state) = world.block_state(neighbour) {
                    let Some(found) = state.property("distance").and_then(|value| value.parse::<i32>().ok()) else { continue };
                    if found > distance + 1 {
                        world.set_block_state(neighbour, &state.with("distance", &(distance + 1).to_string()));
                        levels[distance as usize].insert(neighbour);
                    }
                }
            }
        }
    }
}

fn place_tree(tree: &SlimeTree, context: &mut FeatureContext<'_>) -> bool {
    let world = &mut *context.world;
    let origin = context.origin;
    // The original draws nextInt(random_height) unconditionally; a zero random height throws there, so the
    // shipped configurations (all with a positive random height) are the only ones that can occur.
    let mut height = tree.base_height + if tree.random_height > 0 { world.random_int(tree.random_height) } else { 0 };
    if tree.can_double_height && world.random_int(10) == 0 {
        height *= 2;
    }
    let below = [origin[0], origin[1] - 1, origin[2]];
    let [min_y, max_y] = world.build_height();
    if origin[1] < min_y + 1 || origin[1] + height + 1 > max_y || !world.is_in_tag(below, "tconstruct:slimy_soil") {
        return false;
    }
    // setDirtAt: a soil in the vanilla dirt tag becomes plain dirt under the trunk.
    if world.is_in_tag(below, "minecraft:dirt") {
        world.set_block_state(below, &BlockState::parse("minecraft:dirt").unwrap());
    }
    let mut placed = BTreeSet::new();
    for i in 0..height {
        set_log(world, tree, [origin[0], origin[1] + i, origin[2]], &mut placed);
    }
    place_canopy(world, tree, origin, height, &mut placed);
    if placed.is_empty() {
        return false;
    }
    update_leaf_distances(world, &placed);
    true
}

/* SlimeFungusFeature (over the vanilla HugeFungusFeature) */

#[derive(Clone)]
enum BlockPredicate {
    True,
    MatchingTag(String),
    MatchingBlocks(BTreeSet<String>),
    AnyOf(Vec<BlockPredicate>),
    AllOf(Vec<BlockPredicate>),
    Not(Box<BlockPredicate>),
}

fn predicate_of(value: &Value) -> Result<BlockPredicate, String> {
    let kind = value.get("type").and_then(Value::as_str).ok_or("block predicate needs a type")?;
    match kind {
        "minecraft:true" => Ok(BlockPredicate::True),
        "minecraft:matching_block_tag" => {
            let tag = value.get("tag").and_then(Value::as_str).ok_or("matching_block_tag needs a tag")?;
            Ok(BlockPredicate::MatchingTag(tag.to_owned()))
        }
        "minecraft:matching_blocks" => {
            let blocks = value.get("blocks").and_then(Value::as_array).ok_or("matching_blocks needs blocks")?;
            let mut out = BTreeSet::new();
            for block in blocks {
                out.insert(block.as_str().ok_or("matching_blocks entries must be block ids")?.to_owned());
            }
            Ok(BlockPredicate::MatchingBlocks(out))
        }
        "minecraft:any_of" | "minecraft:all_of" => {
            let list = value.get("predicates").and_then(Value::as_array).ok_or("combining predicate needs predicates")?;
            let mut out = Vec::new();
            for entry in list {
                out.push(predicate_of(entry)?);
            }
            Ok(if kind == "minecraft:any_of" { BlockPredicate::AnyOf(out) } else { BlockPredicate::AllOf(out) })
        }
        "minecraft:not" => Ok(BlockPredicate::Not(Box::new(predicate_of(value.get("predicate").ok_or("not needs a predicate")?)?))),
        other => Err(format!("unsupported block predicate {other}")),
    }
}

impl BlockPredicate {
    fn test(&self, world: &dyn BlockEditor, pos: BlockPos) -> bool {
        match self {
            BlockPredicate::True => true,
            BlockPredicate::MatchingTag(tag) => world.is_in_tag(pos, tag),
            BlockPredicate::MatchingBlocks(blocks) => world.block_state(pos).map(|state| blocks.contains(&state.block.to_string())).unwrap_or(false),
            BlockPredicate::AnyOf(list) => list.iter().any(|predicate| predicate.test(world, pos)),
            BlockPredicate::AllOf(list) => list.iter().all(|predicate| predicate.test(world, pos)),
            BlockPredicate::Not(predicate) => !predicate.test(world, pos),
        }
    }
}

#[derive(Clone)]
struct SlimeFungus {
    ground_tag: String,
    stem: BlockState,
    hat: BlockState,
    decor: BlockState,
    replaceable: BlockPredicate,
    planted: bool,
}

fn slime_fungus(value: &Value) -> Result<ConfiguredFeature, String> {
    let config = config_of(value);
    let fungus = SlimeFungus {
        ground_tag: config.get("valid_base").and_then(Value::as_str).ok_or("slime_fungus needs valid_base")?.trim_start_matches('#').to_owned(),
        stem: state_of(config.get("stem_state").ok_or("slime_fungus needs stem_state")?)?,
        hat: state_of(config.get("hat_state").ok_or("slime_fungus needs hat_state")?)?,
        decor: state_of(config.get("decor_state").ok_or("slime_fungus needs decor_state")?)?,
        replaceable: predicate_of(config.get("replaceable_blocks").ok_or("slime_fungus needs replaceable_blocks")?)?,
        planted: bool_of(config, "planted", false),
    };
    Ok(ConfiguredFeature::new(move |context| place_fungus(&fungus, context)))
}

/// HugeFungusFeature.isReplaceable: the native replaceability of the state, or (for stems only) the
/// configuration's replaceable-blocks predicate.
fn fungus_replaceable(world: &dyn BlockEditor, fungus: &SlimeFungus, pos: BlockPos, stem: bool) -> bool {
    world.is_replaceable(pos) || (stem && fungus.replaceable.test(world, pos))
}

fn place_stem(world: &mut dyn BlockEditor, fungus: &SlimeFungus, origin: BlockPos, height: i32, wide: bool) {
    let radius: i32 = if wide { 1 } else { 0 };
    for x in -radius..=radius {
        for z in -radius..=radius {
            // A wide stem is 3x3; its corners are stray stems, placed one time in ten.
            let corner = wide && x.abs() == radius && z.abs() == radius;
            for y in 0..height {
                let pos = [origin[0] + x, origin[1] + y, origin[2] + z];
                if !fungus_replaceable(world, fungus, pos, true) {
                    continue;
                }
                if fungus.planted {
                    if !world.is_air(pos) {
                        world.remove_block(pos);
                    }
                    world.set_block_state(pos, &fungus.stem);
                } else if corner {
                    if world.random_float() < 0.1 {
                        world.set_block_state(pos, &fungus.stem);
                    }
                } else {
                    world.set_block_state(pos, &fungus.stem);
                }
            }
        }
    }
}

fn place_hat_block(world: &mut dyn BlockEditor, fungus: &SlimeFungus, pos: BlockPos, decor_chance: f32, hat_chance: f32, vine_chance: f32) {
    if world.random_float() < decor_chance {
        world.set_block_state(pos, &fungus.decor);
    } else if world.random_float() < hat_chance {
        world.set_block_state(pos, &fungus.hat);
        if world.random_float() < vine_chance {
            // HugeFungusFeature.tryPlaceWeepingVines: reachable only for nether-wart hats (vine_chance is 0
            // otherwise); the shipped slime fungi use leaf hats, so this is never taken with shipped data.
        }
    }
}

fn place_hat_drop_block(world: &mut dyn BlockEditor, fungus: &SlimeFungus, pos: BlockPos, wart: bool) {
    let below = [pos[0], pos[1] - 1, pos[2]];
    let below_is_hat = world.block_state(below).map(|state| state.block == fungus.hat.block).unwrap_or(false);
    if below_is_hat {
        world.set_block_state(pos, &fungus.hat);
    } else if world.random_float() < 0.15 {
        world.set_block_state(pos, &fungus.hat);
        if wart && world.random_int(11) == 0 {
            // tryPlaceWeepingVines, as in place_hat_block: nether-wart hats only, never with shipped data.
        }
    }
}

fn place_hat(world: &mut dyn BlockEditor, fungus: &SlimeFungus, origin: BlockPos, height: i32, wide: bool) {
    let wart = fungus.hat.block.to_string() == "minecraft:nether_wart_block";
    let layers = (world.random_int(1 + height / 3) + 5).min(height);
    let base = height - layers;
    for k in base..=height {
        let mut radius = if k < height - world.random_int(3) { 2 } else { 1 };
        if layers > 8 && k < base + 4 {
            radius = 3;
        }
        if wide {
            radius += 1;
        }
        for m in -radius..=radius {
            for n in -radius..=radius {
                let m_edge = m == -radius || m == radius;
                let n_edge = n == -radius || n == radius;
                let inner = !m_edge && !n_edge && k != height;
                let corner = m_edge && n_edge;
                let lower = k < base + 3;
                let pos = [origin[0] + m, origin[1] + k, origin[2] + n];
                if !fungus_replaceable(world, fungus, pos, false) {
                    continue;
                }
                if fungus.planted && !world.is_air(pos) {
                    world.remove_block(pos);
                }
                if lower {
                    if !inner {
                        place_hat_drop_block(world, fungus, pos, wart);
                    }
                } else if inner {
                    place_hat_block(world, fungus, pos, 0.1, 0.2, if wart { 0.1 } else { 0.0 });
                } else if corner {
                    place_hat_block(world, fungus, pos, 0.01, 0.7, if wart { 0.083 } else { 0.0 });
                } else {
                    place_hat_block(world, fungus, pos, 5.0e-4, 0.98, if wart { 0.07 } else { 0.0 });
                }
            }
        }
    }
}

fn place_fungus(fungus: &SlimeFungus, context: &mut FeatureContext<'_>) -> bool {
    let world = &mut *context.world;
    let origin = context.origin;
    let below = [origin[0], origin[1] - 1, origin[2]];
    // SlimeFungusFeature replaces the vanilla valid-base state with its ground tag.
    if !world.is_in_tag(below, &fungus.ground_tag) {
        return false;
    }
    let mut height = 4 + world.random_int(10);
    if world.random_int(12) == 0 {
        height *= 2;
    }
    if !fungus.planted {
        // SlimeFungusFeature measures the ceiling against the chunk generator's depth; an editor with no
        // generator context reports none, and the check cannot be made there.
        if let Some(depth) = world.generation_depth() {
            if origin[1] + height + 1 >= depth {
                return false;
            }
        }
    }
    let wide = !fungus.planted && world.random_float() < 0.06;
    world.set_block_state(origin, &air());
    place_stem(world, fungus, origin, height, wide);
    place_hat(world, fungus, origin, height, wide);
    true
}

/* LeaveVineDecorator (the ender trees' vine decorator) */

fn leave_vines(value: &Value) -> Result<TreeDecorator, String> {
    let config = config_of(value);
    let vines = config.get("vines").and_then(Value::as_str).ok_or("leave_vines needs a vines block")?;
    let vines = Identifier::parse(vines).map_err(|error| format!("bad leave_vines block {vines}: {error:?}"))?;
    let probability = config.get("probability").and_then(Value::as_f64).ok_or("leave_vines needs a probability")? as f32;
    if !(0.0..=1.0).contains(&probability) {
        return Err("leave_vines probability out of range".to_owned());
    }
    Ok(TreeDecorator::new(move |context| {
        // Direction.Plane.HORIZONTAL iterates south, west, north, east in the original.
        for &leaf in context.leaves {
            for direction in [Direction::South, Direction::West, Direction::North, Direction::East] {
                if context.world.random_float() < probability {
                    let offset = direction.relative(leaf);
                    if context.world.is_air(offset) {
                        hang_vine(context.world, &vines, offset, direction.opposite());
                    }
                }
            }
        }
        Ok(())
    }))
}

/// LeaveVineDecorator.addHangingVine: the vine on the leaf's face, then up to four more below it while air.
fn hang_vine(world: &mut dyn BlockEditor, vines: &Identifier, pos: BlockPos, face: Direction) {
    let state = BlockState::new(vines.clone()).with(face.name(), "true");
    world.set_block_state(pos, &state);
    let mut target = [pos[0], pos[1] - 1, pos[2]];
    let mut remaining = 4;
    while remaining > 0 && world.is_air(target) {
        world.set_block_state(target, &state);
        target = [target[0], target[1] - 1, target[2]];
        remaining -= 1;
    }
}

/* ExtraRootVariantPlacer (the ender trees' root placer) */

fn extra_root_variants(value: &Value) -> Result<RootPlacement, String> {
    let config = config_of(value);
    let variants = config.get("root_variants").and_then(Value::as_array).ok_or("extra_root_variants needs root_variants")?;
    if variants.is_empty() {
        return Err("extra_root_variants needs at least one root variant".to_owned());
    }
    // The variant match sets and state providers, in configuration order, are the placement's bindings; the
    // native mangrove simulation (which owns the trunk offset and the muddy-roots fallback of
    // mangrove_root_placement) calls back per root position, and the first matching variant wins, exactly as
    // ExtraRootVariantPlacer.placeRoot checks its variants before delegating to the mangrove placer.
    let mut bindings = WorldgenBindings::default();
    for variant in variants {
        bindings.block_sets.push(variant.get("matches").cloned().ok_or("root variant needs matches")?);
        bindings.state_providers.push(variant.get("state_provider").cloned().ok_or("root variant needs a state_provider")?);
    }
    let count = variants.len();
    Ok(RootPlacement::new(bindings, move |context| {
        for index in 0..count {
            if context.world.matches_block_set(index, context.pos)? {
                return Ok(Some(context.world.sample_state_provider(index, context.pos)?));
            }
        }
        Ok(None)
    }))
}

/* The slime islands: IslandStructure, IslandPlacement and IslandPiece */

/// A weighted entry list as both the structure data and the piece configuration write it.
fn weighted_entries(value: Option<&Value>, key: &str) -> Result<Vec<(String, i32)>, String> {
    let entries = value.and_then(Value::as_array).ok_or_else(|| format!("island data needs {key}"))?;
    let mut out = Vec::new();
    for entry in entries {
        let data = entry.get("data").and_then(Value::as_str).ok_or_else(|| format!("{key} entry needs data"))?;
        let weight = entry.get("weight").and_then(Value::as_i64).ok_or_else(|| format!("{key} entry needs a weight"))?;
        let weight = i32::try_from(weight).map_err(|_| format!("{key} entry weight out of range"))?;
        out.push((data.to_owned(), weight));
    }
    Ok(out)
}

/// SimpleWeightedRandomList.getRandomValue: one draw over the total weight, walking the entries in order.
fn pick_weighted<T: Clone>(entries: &[(T, i32)], random: &mut dyn FnMut(i32) -> Result<i32, String>) -> Result<Option<T>, String> {
    let total: i32 = entries.iter().map(|(_, weight)| *weight).sum();
    if total <= 0 {
        return Ok(None);
    }
    let mut roll = random(total)?;
    for (entry, weight) in entries {
        roll -= weight;
        if roll < 0 {
            return Ok(Some(entry.clone()));
        }
    }
    Ok(entries.last().map(|(entry, _)| entry.clone()))
}

#[derive(Clone, Copy, PartialEq)]
enum IslandPlacementKind {
    Sky,
    Sea,
}

#[derive(Clone)]
struct Island {
    placement: IslandPlacementKind,
    templates: Vec<(String, i32)>,
    trees: Vec<(String, i32)>,
    vines: Option<String>,
    grasses: Vec<(String, i32)>,
}

fn rotation_name(rotation: StructureRotation) -> &'static str {
    match rotation {
        StructureRotation::None => "none",
        StructureRotation::Clockwise90 => "clockwise_90",
        StructureRotation::Clockwise180 => "clockwise_180",
        StructureRotation::Counterclockwise90 => "counterclockwise_90",
    }
}

fn mirror_name(mirror: StructureMirror) -> &'static str {
    match mirror {
        StructureMirror::None => "none",
        StructureMirror::LeftRight => "left_right",
        StructureMirror::FrontBack => "front_back",
    }
}

fn island_structure(value: &Value) -> Result<ConfiguredStructure, String> {
    let config = config_of(value);
    let placement = match config.get("placement").and_then(Value::as_str) {
        Some("sky") => IslandPlacementKind::Sky,
        Some("sea") => IslandPlacementKind::Sea,
        other => return Err(format!("island placement must be sky or sea, not {other:?}")),
    };
    // The vanilla structure settings in the same object (biomes, step, spawn overrides) are the host's own
    // decode; the codec reads only the island fields, as IslandStructure.CODEC adds them to the settings codec.
    let island = Island {
        placement,
        templates: weighted_entries(config.get("templates"), "templates")?,
        trees: weighted_entries(config.get("trees"), "trees")?,
        vines: config.get("vines").and_then(Value::as_str).map(str::to_owned),
        grasses: weighted_entries(config.get("grasses"), "grasses")?,
    };
    Ok(ConfiguredStructure::new(move |context| plan_island(&island, context)))
}

fn plan_island(island: &Island, context: &mut StructureContext<'_>) -> Result<Option<StructureStart>, String> {
    let world = &mut *context.world;
    let Some(center) = world.top_of_chunk_center(context.chunk)? else { return Ok(None) };
    if !world.is_valid_biome(center)? {
        return Ok(None);
    }
    let mut pieces = Vec::new();
    if let Some(template) = pick_weighted(&island.templates, &mut |bound| world.random_int(bound))? {
        // Rotation.getRandom, then the placement height (a sky island draws its +0..49 inside), then the
        // mirror, then the tree: IslandStructure.generatePieces' draw order.
        let rotation = match world.random_int(4)? {
            0 => StructureRotation::None,
            1 => StructureRotation::Clockwise90,
            2 => StructureRotation::Clockwise180,
            _ => StructureRotation::Counterclockwise90,
        };
        let height = match island.placement {
            IslandPlacementKind::Sea => (world.sea_level() - 7).max(0),
            IslandPlacementKind::Sky => {
                let (x_offset, z_offset) = match rotation {
                    StructureRotation::Clockwise90 => (-5, 5),
                    StructureRotation::Clockwise180 => (-5, -5),
                    StructureRotation::Counterclockwise90 => (5, -5),
                    StructureRotation::None => (5, 5),
                };
                let x = context.chunk[0] * 16 + 7;
                let z = context.chunk[1] * 16 + 7;
                let mut lowest = i32::MAX;
                for (corner_x, corner_z) in [(x, z), (x, z + z_offset), (x + x_offset, z), (x + x_offset, z + z_offset)] {
                    lowest = lowest.min(world.first_occupied_height(corner_x, corner_z)?);
                }
                (lowest + 60 + world.random_int(50)?).min(world.generation_depth() - 20)
            }
        };
        let origin = [context.chunk[0] * 16 + 8, height, context.chunk[1] * 16 + 8];
        let mirror = match world.random_int(3)? {
            0 => StructureMirror::None,
            1 => StructureMirror::LeftRight,
            _ => StructureMirror::FrontBack,
        };
        let tree = pick_weighted(&island.trees, &mut |bound| world.random_int(bound))?;
        pieces.push(StructurePiece {
            kind: id("island"),
            configuration: serde_json::json!({
                "template": template,
                "origin": origin,
                "rotation": rotation_name(rotation),
                "mirror": mirror_name(mirror),
                "tree": tree,
                "vines": island.vines,
                "grasses": island.grasses.iter().map(|(data, weight)| serde_json::json!({"data": data, "weight": weight})).collect::<Vec<_>>(),
            }),
        });
    }
    Ok(Some(StructureStart { position: center, pieces }))
}

#[derive(Clone)]
struct IslandPieceData {
    tree: Option<String>,
    vines: Option<String>,
    grasses: Vec<(String, i32)>,
}

fn pos_of(value: Option<&Value>) -> Result<BlockPos, String> {
    let array = value.and_then(Value::as_array).ok_or("island piece needs an origin")?;
    if array.len() != 3 {
        return Err("island piece origin needs three coordinates".to_owned());
    }
    let mut pos = [0; 3];
    for (index, coordinate) in array.iter().enumerate() {
        pos[index] = coordinate.as_i64().and_then(|n| i32::try_from(n).ok()).ok_or("island piece origin out of range")?;
    }
    Ok(pos)
}

fn rotation_of(value: Option<&Value>) -> Result<StructureRotation, String> {
    match value.and_then(Value::as_str) {
        Some("none") => Ok(StructureRotation::None),
        Some("clockwise_90") => Ok(StructureRotation::Clockwise90),
        Some("clockwise_180") => Ok(StructureRotation::Clockwise180),
        Some("counterclockwise_90") => Ok(StructureRotation::Counterclockwise90),
        other => Err(format!("bad island piece rotation {other:?}")),
    }
}

fn mirror_of(value: Option<&Value>) -> Result<StructureMirror, String> {
    match value.and_then(Value::as_str) {
        Some("none") => Ok(StructureMirror::None),
        Some("left_right") => Ok(StructureMirror::LeftRight),
        Some("front_back") => Ok(StructureMirror::FrontBack),
        other => Err(format!("bad island piece mirror {other:?}")),
    }
}

fn island_piece(value: &Value) -> Result<ConfiguredStructurePiece, String> {
    let template = value.get("template").and_then(Value::as_str).ok_or("island piece needs a template")?;
    let template = Identifier::parse(template).map_err(|error| format!("bad island template {template}: {error:?}"))?;
    let piece = IslandPieceData {
        tree: value.get("tree").and_then(Value::as_str).map(str::to_owned),
        vines: value.get("vines").and_then(Value::as_str).map(str::to_owned),
        grasses: weighted_entries(value.get("grasses"), "grasses")?,
    };
    // A restored piece carries its count in the configuration; a fresh one starts at zero.
    let placed = value.get("trees_placed").and_then(Value::as_u64).unwrap_or(0);
    Ok(ConfiguredStructurePiece::new(
        StructurePieceTemplate { template, origin: pos_of(value.get("origin"))?, rotation: rotation_of(value.get("rotation"))?, mirror: mirror_of(value.get("mirror"))? },
        serde_json::json!({ "trees_placed": placed }),
        move |context| island_marker(&piece, context),
    )
    .with_state_validator(|state| {
        if state.get("trees_placed").and_then(Value::as_u64).is_some() {
            Ok(())
        } else {
            Err("island piece state needs trees_placed".to_owned())
        }
    }))
}

/// IslandPiece.handleDataMarker: the template's structure blocks name one of these three markers.
fn island_marker(piece: &IslandPieceData, context: &mut StructureMarkerContext<'_>) -> Result<(), String> {
    let marker = context.marker;
    let pos = context.position;
    let world = &mut *context.world;
    let state = &mut *context.state;
    match marker {
        "tconstruct:slime_vine" => {
            let Some(vines) = &piece.vines else { return Ok(()) };
            if world.random_bool()? {
                place_island_vine(world, pos, vines)?;
            }
        }
        "tconstruct:slime_tree" => {
            let Some(tree) = &piece.tree else { return Ok(()) };
            let placed = state.get("trees_placed").and_then(Value::as_u64).ok_or("island piece state needs trees_placed")?;
            if placed < 3 && world.random_bool()? {
                let tree = Identifier::parse(tree).map_err(|error| format!("bad island tree {tree}: {error:?}"))?;
                if world.place_feature(&tree, pos)? {
                    state["trees_placed"] = Value::from(placed + 1);
                }
            }
        }
        "tconstruct:slime_tall_grass" => {
            if world.random_bool()? {
                if let Some(grass) = pick_weighted(&piece.grasses, &mut |bound| world.random_int(bound))? {
                    let grass = Identifier::parse(&grass).map_err(|error| format!("bad island grass {grass}: {error:?}"))?;
                    let grass_state = BlockState::new(grass);
                    if world.can_survive_plant(&grass_state, pos)? {
                        world.set_block_state(pos, &grass_state, 2)?;
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

const SLIME_VINES: [&str; 4] = ["tconstruct:earth_slime_vine", "tconstruct:sky_slime_vine", "tconstruct:ichor_slime_vine", "tconstruct:ender_slime_vine"];

fn has_horizontal_side(state: &BlockState) -> bool {
    ["north", "east", "south", "west"].iter().any(|side| state.property(side) == Some("true"))
}

/// VineBlock.isAcceptableNeighbour, as the piece world reports it: the neighbour's face toward the vine is
/// sturdy (a leaf's full shape makes it sturdy, which is the original's third clause).
fn acceptable_neighbour(world: &mut dyn StructurePieceWorld, pos: BlockPos, side: Direction) -> Result<bool, String> {
    world.face_sturdy(side.relative(pos), side.opposite())
}

/// IslandPiece.placeVine: face the vine at every acceptable neighbour (each write starts from the default
/// state, as the original's does, so the last acceptable face wins), then grow it down a few times.
fn place_island_vine(world: &mut dyn StructurePieceWorld, pos: BlockPos, vines: &str) -> Result<(), String> {
    let base = BlockState::new(Identifier::parse(vines).map_err(|error| format!("bad island vines {vines}: {error:?}"))?);
    for direction in Direction::ALL {
        if direction == Direction::Down {
            continue;
        }
        if acceptable_neighbour(world, pos, direction)? {
            world.set_block_state(pos, &base.clone().with(direction.name(), "true"), 2)?;
        }
    }
    let mut vine_pos = pos;
    let mut size = world.random_int(8)?;
    loop {
        let state = world.block_state(vine_pos)?;
        if !SLIME_VINES.contains(&state.block.to_string().as_str()) {
            break;
        }
        grow_vine(world, vine_pos, &state)?;
        vine_pos = [vine_pos[0], vine_pos[1] - 1, vine_pos[2]];
        if size == 0 {
            break;
        }
        size -= 1;
    }
    Ok(())
}

fn free_floating(world: &mut dyn StructurePieceWorld, pos: BlockPos, state: &BlockState) -> Result<bool, String> {
    for direction in [Direction::North, Direction::South, Direction::West, Direction::East] {
        if state.property(direction.name()) == Some("true") && acceptable_neighbour(world, pos, direction)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// SlimeVineBlock.grow: extend the vine one block down, advancing its stage when a free-floating run of
/// three same-stage vines is above, or half the time otherwise.
fn grow_vine(world: &mut dyn StructurePieceWorld, pos: BlockPos, state: &BlockState) -> Result<(), String> {
    let stage = state.property("stage").unwrap_or("start");
    if stage == "end" || !has_horizontal_side(state) {
        return Ok(());
    }
    let below = [pos[0], pos[1] - 1, pos[2]];
    if world.block_state(below)?.block.to_string() != "minecraft:air" {
        return Ok(());
    }
    let mut grown = state.clone();
    if free_floating(world, pos, state)? {
        let mut count = 1;
        while count < 3 {
            let above = world.block_state([pos[0], pos[1] + count, pos[2]])?;
            if above.block != state.block || above.property("stage").unwrap_or("start") != stage {
                break;
            }
            count += 1;
        }
        if count > 2 || world.random_int(2)? == 0 {
            grown = grown.with("stage", match stage {
                "start" => "middle",
                "middle" => "end",
                _ => "start",
            });
        }
    }
    world.set_block_state(below, &grown.with("up", "false"), 3)?;
    Ok(())
}

pub(crate) fn register() {
    FeatureTypeRegistry::register(id("slime_tree"), slime_tree);
    FeatureTypeRegistry::register(id("slime_fungus"), slime_fungus);
    TreeDecoratorTypeRegistry::register(id("leave_vines"), leave_vines);
    RootPlacementTypeRegistry::register(id("extra_root_variants"), extra_root_variants);
    StructureTypeRegistry::register(id("island"), island_structure);
    StructurePieceTypeRegistry::register(id("island"), island_piece);
}
