//! The slime trees that grow from saplings. Ported from the original `SlimeTreeFeature` and `SlimeSaplingBlock`; the growth uses only
//! PlutoLib's `BlockEditor`, so it also works for any caller that can hand over one.
use std::collections::BTreeSet;

use pluto_lib::{BlockBehavior, BlockBehaviors, BlockEditor, BlockState, Direction, Identifier};

type Pos = [i32; 3];

/// How a tree looks and how tall it grows (`SlimeTreeConfig`).
pub struct TreeConfig {
    pub trunk: &'static str,
    pub leaves: &'static str,
    /// The vine hung from the canopy, if the tree has vines.
    pub vines: Option<&'static str>,
    pub base_height: i32,
    pub random_height: i32,
    pub can_double_height: bool,
}

/// The tree an earth sapling grows into (`earthSlimeTree`).
pub const EARTH_TREE: TreeConfig = TreeConfig { trunk: "tconstruct:greenheart_log", leaves: "tconstruct:earth_slime_leaves", vines: None, base_height: 4, random_height: 3, can_double_height: false };
/// The tree a sky sapling grows into (`skySlimeTree`).
pub const SKY_TREE: TreeConfig = TreeConfig { trunk: "tconstruct:skyroot_log", leaves: "tconstruct:sky_slime_leaves", vines: None, base_height: 5, random_height: 4, can_double_height: true };
/// The sky tree of the islands (`skySlimeIslandTree`): the same, with vines.
pub const SKY_ISLAND_TREE: TreeConfig =
    TreeConfig { trunk: "tconstruct:skyroot_log", leaves: "tconstruct:sky_slime_leaves", vines: Some("tconstruct:sky_slime_vine[stage=middle]"), base_height: 5, random_height: 4, can_double_height: true };

fn state(text: &str) -> BlockState {
    BlockState::parse(text).expect("a fixed block id")
}

fn offset(pos: Pos, dx: i32, dy: i32, dz: i32) -> Pos {
    [pos[0] + dx, pos[1] + dy, pos[2] + dz]
}

fn air_or_leaves(world: &dyn BlockEditor, pos: Pos) -> bool {
    world.is_air(pos) || world.is_in_tag(pos, "minecraft:leaves")
}

fn place(world: &mut dyn BlockEditor, pos: Pos, what: &BlockState, touched: &mut BTreeSet<Pos>) -> bool {
    if !air_or_leaves(&*world, pos) {
        return false;
    }
    world.set_block_state(pos, what);
    touched.insert(pos);
    true
}

fn diamond_layer(world: &mut dyn BlockEditor, range: i32, center: Pos, leaves: &BlockState, touched: &mut BTreeSet<Pos>) {
    for x in -range..=range {
        for z in -range..=range {
            if x.abs() + z.abs() <= range {
                place(world, offset(center, x, 0, z), leaves, touched);
            }
        }
    }
}

/// A vine with one to three random sides (`getRandomizedVine`).
fn random_vine(world: &mut dyn BlockEditor, vine: &BlockState) -> BlockState {
    let sides = ["north", "east", "south", "west"];
    let mut result = vine.clone();
    for side in sides {
        result = result.with(side, "false");
    }
    for _ in 0..(world.random_int(3) + 1) {
        let side = sides[world.random_int(4) as usize];
        result = result.with(side, "true");
    }
    result
}

/// Grows the tree of `config` with its trunk base at `origin` (`SlimeTreeFeature.place`); false if there is no room or no slime soil.
pub fn grow_tree(world: &mut dyn BlockEditor, origin: Pos, config: &TreeConfig) -> bool {
    let mut height = world.random_int(config.random_height) + config.base_height;
    if config.can_double_height && world.random_int(10) == 0 {
        height *= 2;
    }
    let [bottom, top] = world.build_height();
    let below = offset(origin, 0, -1, 0);
    if !(origin[1] >= bottom + 1 && origin[1] + height + 1 <= top && world.is_in_tag(below, "tconstruct:slimy_soil")) {
        return false;
    }
    // the canopy and the distance of the leaves reach ten blocks sideways: never touch a chunk that is not loaded
    if !world.is_area_loaded(origin, 10) {
        return false;
    }
    if world.is_in_tag(below, "minecraft:dirt") {
        world.set_block_state(below, &state("minecraft:dirt"));
    }
    let trunk = state(config.trunk);
    let leaves = state(config.leaves);
    let air = state("minecraft:air");
    let mut logs = BTreeSet::new();
    let mut touched = BTreeSet::new();
    for dy in 0..height {
        place(world, offset(origin, 0, dy, 0), &trunk, &mut logs);
    }
    let crown = offset(origin, 0, height, 0);
    for i in 0..4 {
        diamond_layer(world, i + 1, offset(crown, 0, -i, 0), &leaves, &mut touched);
    }
    let layer = offset(crown, 0, -3, 0);
    for (dx, dz) in [(4, 0), (-4, 0), (0, 4), (0, -4)] {
        place(world, offset(layer, dx, 0, dz), &air, &mut touched);
    }
    if config.vines.is_some() {
        for (dx, dz) in [(1, 1), (1, -1), (-1, 1), (-1, -1)] {
            place(world, offset(layer, dx, 0, dz), &air, &mut touched);
        }
    }
    // drippers: one block down only, because of the decay distance of leaves
    let drip = offset(layer, 0, -1, 0);
    for (dx, dz) in [(3, 0), (-3, 0), (0, -3), (0, 3)] {
        place(world, offset(drip, dx, 0, dz), &leaves, &mut touched);
    }
    if config.vines.is_none() {
        // the original repeats (-3, 0) here instead of (1, -1); kept as it is
        for (dx, dz) in [(1, 1), (-3, 0), (-1, 1), (-1, -1)] {
            place(world, offset(drip, dx, 0, dz), &leaves, &mut touched);
        }
    }
    if let Some(vine) = config.vines {
        let vine = state(vine);
        let low = offset(drip, 0, -1, 0);
        for (dx, dz) in [(3, 0), (-3, 0), (0, -3), (0, 3)] {
            let v = random_vine(world, &vine).with("up", "true");
            place(world, offset(low, dx, 0, dz), &v, &mut touched);
        }
        for (dx, dz) in [(2, 2), (2, -2), (-2, 2), (-2, -2)] {
            let v = random_vine(world, &vine);
            place(world, offset(low, dx, 1, dz), &v.clone().with("up", "true"), &mut touched);
            place(world, offset(low, dx, 0, dz), &v, &mut touched);
        }
    }
    if logs.is_empty() {
        return false;
    }
    update_leaves(world, &logs);
    true
}

/// Sets the distance of the leaves around the logs, as the feature does when it is done (`updateLeaves`).
fn update_leaves(world: &mut dyn BlockEditor, logs: &BTreeSet<Pos>) {
    let mut current: BTreeSet<Pos> = BTreeSet::new();
    for log in logs {
        for direction in Direction::ALL {
            let pos = direction.relative(*log);
            if logs.contains(&pos) {
                continue;
            }
            if let Some(leaf) = world.block_state(pos) {
                if leaf.property("distance").is_some() {
                    current.insert(pos);
                    world.set_block_state(pos, &leaf.with("distance", "1"));
                }
            }
        }
    }
    for distance in 1..6 {
        let mut next = BTreeSet::new();
        for pos in &current {
            for direction in Direction::ALL {
                let around = direction.relative(*pos);
                if current.contains(&around) || next.contains(&around) {
                    continue;
                }
                if let Some(leaf) = world.block_state(around) {
                    if let Some(value) = leaf.property("distance").and_then(|d| d.parse::<i32>().ok()) {
                        if value > distance + 1 {
                            world.set_block_state(around, &leaf.with("distance", &(distance + 1).to_string()));
                            next.insert(around);
                        }
                    }
                }
            }
        }
        current = next;
    }
}

/// A sapling advances: the first time it only changes its stage, the second time it becomes a tree (`SaplingBlock.advanceTree`).
fn advance(world: &mut dyn BlockEditor, pos: Pos, sapling: &BlockState, config: &'static TreeConfig) {
    if sapling.property("stage") == Some("0") {
        world.set_block_state(pos, &sapling.clone().with("stage", "1"));
    } else {
        // the grower removes the sapling before it tries, and puts it back if there was no room
        world.remove_block(pos);
        if !grow_tree(world, pos, config) {
            world.set_block_state(pos, sapling);
        }
    }
}

pub fn register() {
    for (kind, config) in [("earth", &EARTH_TREE), ("sky", &SKY_TREE)] {
        BlockBehaviors::register(
            BlockBehavior::new(Identifier::new("tconstruct", &format!("{kind}_slime_sapling")).expect("valid id"))
                .can_survive(|c| crate::world::plant_can_stand(c.view, c.pos))
                .random_tick(move |c| {
                    if c.world.brightness(Direction::Up.relative(c.pos)) >= 9 && c.world.random_int(7) == 0 {
                        advance(&mut *c.world, c.pos, &c.state, config);
                    }
                })
                .can_bonemeal(|_| true)
                .bonemeal_success(|c| c.world.random_float() < 0.45)
                .on_bonemeal(move |c| advance(&mut *c.world, c.pos, &c.state, config)),
        );
    }
}

/// The blood and ichor fungi (`SlimeFungusBlock`): they grow on slime soil and a bone meal places the huge fungus of their ground.
pub fn register_fungus() {
    for kind in ["blood", "ichor"] {
        BlockBehaviors::register(
            BlockBehavior::new(Identifier::new("tconstruct", &format!("{kind}_slime_sapling")).expect("valid id"))
                .can_survive(|c| c.view.is_in_tag(Direction::Down.relative(c.pos), "tconstruct:slimy_soil"))
                .can_bonemeal(|c| c.view.is_in_tag(Direction::Down.relative(c.pos), "tconstruct:slimy_soil"))
                .bonemeal_success(|c| c.world.random_float() < 0.4)
                .on_bonemeal(move |c| {
                    let ground = Direction::Down.relative(c.pos);
                    if !c.world.is_in_tag(ground, "tconstruct:slimy_soil") {
                        return;
                    }
                    // a huge cap reaches four blocks sideways: never change a chunk that is not loaded
                    if !c.world.is_area_loaded(c.pos, 5) {
                        return;
                    }
                    // Horizon's huge fungus takes one base block, so each ground has its own configured feature
                    let Some(ground) = c.world.block_state(ground) else { return };
                    c.world.place_feature(&format!("tconstruct:slime_fungus/{kind}_{}", ground.block.path()), c.pos);
                }),
        );
    }
}
