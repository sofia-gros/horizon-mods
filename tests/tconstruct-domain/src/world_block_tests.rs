//! Behaviour tests for the R4-world U1/U2 slice: TinkerWorld's sticky slime
//! predicates and CongealedSlimeBlock's landing/contact rules, dispatched
//! through the registered behaviours the way the host dispatches them.

use pluto_lib::{AfterFallContext, BlockBehaviors, BlockState, Direction, EntityInsideMotionContext, FallOnContext, MemoryBlockView, PathNodeType, PathNodeTypeContext, PistonBehavior};

const SLIMES: [&str; 3] = ["sky_slime", "ichor_slime", "ender_slime"];
const CONGEALED: [&str; 4] = ["earth_congealed_slime", "sky_congealed_slime", "ichor_congealed_slime", "ender_congealed_slime"];

fn state(id: &str) -> BlockState {
    BlockState::parse(id).unwrap()
}

fn slime(name: &str) -> BlockState {
    state(&format!("tconstruct:{name}"))
}

fn registered() {
    crate::world_blocks::register();
}

#[test]
fn sticky_slime_flags_and_stick_to_truth_table() {
    registered();
    for name in SLIMES {
        assert_eq!(pluto_lib::fixtures::slime_flags(&slime(name)), (Some(true), Some(true)), "{name} is a slime block and a sticky block");
    }
    // Nothing is registered for a plain block.
    assert_eq!(pluto_lib::fixtures::slime_flags(&state("minecraft:stone")), (None, None));
    assert_eq!(pluto_lib::fixtures::stick_to(&state("minecraft:stone"), &state("minecraft:stone")), None);

    // TinkerWorld's predicates: sky sticks to anything, ichor to anything but
    // itself, ender only to itself. Targets: itself, another slime type, a
    // plain block, the honey block.
    let table: [(&str, &str, [bool; 4]); 3] = [
        ("sky_slime", "ichor_slime", [true, true, true, true]),
        ("ichor_slime", "sky_slime", [false, true, true, true]),
        ("ender_slime", "sky_slime", [true, false, false, false]),
    ];
    for (name, other_slime, expected) in table {
        let this = slime(name);
        let targets = [slime(name), slime(other_slime), state("minecraft:stone"), state("minecraft:honey_block")];
        for (target, want) in targets.iter().zip(expected) {
            assert_eq!(pluto_lib::fixtures::stick_to(&this, target), Some(want), "{name} sticking to {}", target.block);
        }
    }
}

#[test]
fn congealed_slime_zeroes_fall_damage() {
    registered();
    for name in CONGEALED {
        let mut c = FallOnContext { state: slime(name), pos: [0, 64, 0], entity: 1, kind: "minecraft:player".to_string(), distance: 12.0, damage: 9.0 };
        pluto_lib::fixtures::landing_damage(&mut c);
        assert_eq!(c.damage, 0.0, "{name} deals no fall damage");
    }
    // The sticky slime blocks register no fall_on handler; damage passes through.
    let mut c = FallOnContext { state: slime("sky_slime"), pos: [0, 64, 0], entity: 1, kind: "minecraft:player".to_string(), distance: 12.0, damage: 9.0 };
    pluto_lib::fixtures::landing_damage(&mut c);
    assert_eq!(c.damage, 9.0);
}

fn after_fall(block: &str, living: bool, item: bool, suppressing_bounce: bool, vy: f64) -> AfterFallContext {
    let mut c = AfterFallContext {
        state: slime(block), pos: [0, 64, 0], entity: 1, kind: "minecraft:zombie".to_string(),
        living, item, suppressing_bounce, velocity: [0.25, vy, -0.5], fall_distance: 7.0, on_ground: true,
    };
    pluto_lib::fixtures::after_fall(&mut c);
    c
}

#[test]
fn congealed_after_fall_bounce_matches_the_original() {
    registered();
    for name in CONGEALED {
        // Living entities reflect at full strength and land without fall distance.
        let c = after_fall(name, true, false, false, -2.0);
        assert_eq!(c.velocity, [0.25, 2.0, -0.5], "{name}: living bounce");
        assert_eq!(c.fall_distance, 0.0, "{name}: living bounce clears fall distance");
        assert!(c.on_ground, "{name}: living bounce keeps on-ground");

        // Item entities reflect at 0.8 and are lifted off the ground.
        let c = after_fall(name, false, true, false, -2.0);
        assert!((c.velocity[1] - 1.6).abs() < 1e-12, "{name}: item bounce is 0.8, got {}", c.velocity[1]);
        assert_eq!(c.fall_distance, 0.0, "{name}: item bounce clears fall distance");
        assert!(!c.on_ground, "{name}: item bounce clears on-ground");

        // Anything else (the original calls out XP orbs) falls through untouched.
        let c = after_fall(name, false, false, false, -2.0);
        assert_eq!(c.velocity, [0.25, -2.0, -0.5], "{name}: other kinds do not bounce");
        assert_eq!(c.fall_distance, 7.0, "{name}: other kinds keep fall distance");

        // Suppressing the bounce (sneaking) disables it even for living entities.
        let c = after_fall(name, true, false, true, -2.0);
        assert_eq!(c.velocity, [0.25, -2.0, -0.5], "{name}: suppressed bounce does nothing");
        assert_eq!(c.fall_distance, 7.0);

        // Upward motion is not a landing.
        let c = after_fall(name, true, false, false, 1.5);
        assert_eq!(c.velocity, [0.25, 1.5, -0.5], "{name}: upward velocity untouched");
        assert_eq!(c.fall_distance, 7.0);
    }
}

fn inside(block: &str, position: [f64; 3], previous_position: [f64; 3], velocity: [f64; 3], server: bool, suppressing_bounce: bool) -> EntityInsideMotionContext {
    let mut c = EntityInsideMotionContext {
        state: slime(block), pos: [10, 64, 10], entity: 1, kind: "minecraft:zombie".to_string(),
        server, suppressing_bounce, position, previous_position, velocity, sync_velocity: false, step_sound: None,
    };
    pluto_lib::fixtures::inside_motion(&mut c);
    c
}

#[test]
fn congealed_inside_motion_side_bounce() {
    registered();
    for name in CONGEALED {
        // Moving +X into the block's east side: bounce along X at 0.95 of the
        // distance moved since last tick, marked for sync, step sound played.
        let c = inside(name, [10.9, 64.5, 10.5], [10.7, 64.5, 10.5], [0.0; 3], true, false);
        assert!((c.velocity[0] - 0.19).abs() < 1e-9, "{name}: +X bounce speed, got {}", c.velocity[0]);
        assert_eq!(c.velocity[2], 0.0, "{name}: no Z change");
        assert!(c.sync_velocity, "{name}: bounce marks velocity for sync");
        assert_eq!(c.step_sound, Some([1.0, 1.0]), "{name}: step sound above speed 0.1");

        // Moving -X: the bounce follows the direction's sign.
        let c = inside(name, [10.1, 64.5, 10.5], [10.3, 64.5, 10.5], [0.0; 3], true, false);
        assert!((c.velocity[0] + 0.19).abs() < 1e-9, "{name}: -X bounce speed, got {}", c.velocity[0]);

        // The larger axis wins: here Z.
        let c = inside(name, [10.5, 64.5, 10.9], [10.5, 64.5, 10.7], [0.0; 3], true, false);
        assert_eq!(c.velocity[0], 0.0, "{name}: no X change");
        assert!((c.velocity[2] - 0.19).abs() < 1e-9, "{name}: +Z bounce speed, got {}", c.velocity[2]);

        // A slow contact still bounces but plays no sound.
        let c = inside(name, [10.9, 64.5, 10.5], [10.8, 64.5, 10.5], [0.0; 3], true, false);
        assert!((c.velocity[0] - 0.095).abs() < 1e-9, "{name}: slow bounce speed, got {}", c.velocity[0]);
        assert!(c.sync_velocity);
        assert_eq!(c.step_sound, None, "{name}: no step sound at speed 0.095");

        // Exactly on the block centre the chosen axis' direction is zero, and
        // Java's signum(0) makes the bounce zero rather than full speed.
        let c = inside(name, [10.5, 64.5, 10.5], [10.5, 64.9, 10.5], [0.0, 0.0, 3.0], true, false);
        assert_eq!(c.velocity[2], 0.0, "{name}: zero direction means zero bounce");
        assert!(c.sync_velocity);
    }
}

#[test]
fn congealed_inside_motion_guards() {
    registered();
    let name = "earth_congealed_slime";
    // The y window is [-0.0625, 0.9365) of the block's own height: the top
    // edge is excluded, the bottom edge included.
    let c = inside(name, [10.9, 64.94, 10.5], [10.7, 64.94, 10.5], [0.0; 3], true, false);
    assert_eq!(c.velocity, [0.0; 3], "above the window there is no bounce");
    assert!(!c.sync_velocity);
    // Exactly 0.9365 (measured from a block at y=0 so the subtraction is
    // exact) is outside the window.
    let mut c = EntityInsideMotionContext {
        state: slime(name), pos: [10, 0, 10], entity: 1, kind: "minecraft:zombie".to_string(),
        server: true, suppressing_bounce: false, position: [10.9, 0.9365, 10.5], previous_position: [10.7, 0.9365, 10.5],
        velocity: [0.0; 3], sync_velocity: false, step_sound: None,
    };
    pluto_lib::fixtures::inside_motion(&mut c);
    assert_eq!(c.velocity, [0.0; 3], "at the top edge there is no bounce");
    assert!(!c.sync_velocity);
    let c = inside(name, [10.9, 63.9, 10.5], [10.7, 63.9, 10.5], [0.0; 3], true, false);
    assert_eq!(c.velocity, [0.0; 3], "below the window there is no bounce");
    assert!(!c.sync_velocity);
    let c = inside(name, [10.9, 63.9375, 10.5], [10.7, 63.9375, 10.5], [0.0; 3], true, false);
    assert!((c.velocity[0] - 0.19).abs() < 1e-9, "the bottom edge still bounces, got {}", c.velocity[0]);

    // No double bounce: past the halfway point on the other axis, no bounce.
    let c = inside(name, [11.2, 64.5, 11.0], [11.0, 64.5, 11.0], [0.0; 3], true, false);
    assert_eq!(c.velocity, [0.0; 3], "across offset 0.5 blocks the X bounce");
    assert!(!c.sync_velocity);
    let c = inside(name, [11.2, 64.5, 10.99], [11.0, 64.5, 10.99], [0.0; 3], true, false);
    assert!((c.velocity[0] - 0.19).abs() < 1e-9, "across offset 0.49 still bounces, got {}", c.velocity[0]);

    // Client side and suppressing entities never bounce.
    let c = inside(name, [10.9, 64.5, 10.5], [10.7, 64.5, 10.5], [0.0; 3], false, false);
    assert_eq!(c.velocity, [0.0; 3], "no client-side bounce");
    assert!(!c.sync_velocity);
    let c = inside(name, [10.9, 64.5, 10.5], [10.7, 64.5, 10.5], [0.0; 3], true, true);
    assert_eq!(c.velocity, [0.0; 3], "no suppressed bounce");
    assert!(!c.sync_velocity);
}

#[test]
fn congealed_path_node_is_sticky_honey_and_all_seven_blocks_register() {
    registered();
    let view = MemoryBlockView::default();
    for name in CONGEALED {
        let c = PathNodeTypeContext { state: slime(name), pos: [0, 64, 0], view: &view, entity: None, kind: None };
        assert_eq!(pluto_lib::fixtures::path_node(&c), Some(PathNodeType::StickyHoney), "{name} paths as sticky honey");
    }
    // The sticky slime blocks register no path handler.
    let c = PathNodeTypeContext { state: slime("sky_slime"), pos: [0, 64, 0], view: &view, entity: None, kind: None };
    assert_eq!(pluto_lib::fixtures::path_node(&c), None);
    // Registration of the seven blocks is proven by dispatch: slime flags and
    // the stick table cover the three slimes; landing damage, the after-fall
    // and inside-motion fixtures (which unwrap their handlers) and the path
    // node cover the four congealed blocks.
}

const CLUSTERS: [&str; 19] = [
    "earth_slime_crystal_cluster", "small_earth_slime_crystal_bud", "medium_earth_slime_crystal_bud", "large_earth_slime_crystal_bud",
    "sky_slime_crystal_cluster", "small_sky_slime_crystal_bud", "medium_sky_slime_crystal_bud", "large_sky_slime_crystal_bud",
    "ichor_slime_crystal_cluster", "small_ichor_slime_crystal_bud", "medium_ichor_slime_crystal_bud", "large_ichor_slime_crystal_bud",
    "ender_slime_crystal_cluster", "small_ender_slime_crystal_bud", "medium_ender_slime_crystal_bud", "large_ender_slime_crystal_bud",
    "steel_cluster", "cobalt_cluster", "knightmetal_cluster",
];
const BUDDING: [&str; 4] = ["budding_earth_slime_crystal", "budding_sky_slime_crystal", "budding_ichor_slime_crystal", "budding_ender_slime_crystal"];
const HEADS: [&str; 9] = ["blaze_head", "enderman_head", "stray_head", "husk_head", "drowned_head", "spider_head", "cave_spider_head", "piglin_brute_head", "zombified_piglin_head"];
const WALL_HEADS: [&str; 9] = ["blaze_wall_head", "enderman_wall_head", "stray_wall_head", "husk_wall_head", "drowned_wall_head", "spider_wall_head", "cave_spider_wall_head", "piglin_brute_wall_head", "zombified_piglin_wall_head"];

#[test]
fn clusters_point_along_the_clicked_face_and_keep_their_water() {
    for (face, name) in [(Direction::Down, "down"), (Direction::Up, "up"), (Direction::North, "north"),
                         (Direction::South, "south"), (Direction::West, "west"), (Direction::East, "east")] {
        let placed = crate::world_blocks::cluster_placement(slime("steel_cluster"), face, false);
        assert_eq!(placed.property("facing"), Some(name), "{name}");
        assert_eq!(placed.property("waterlogged"), Some("false"), "{name} placed dry");
        let wet = crate::world_blocks::cluster_placement(slime("steel_cluster"), face, true);
        assert_eq!(wet.property("waterlogged"), Some("true"), "{name} placed in water");
    }
}

#[test]
fn clusters_survive_only_behind_a_sturdy_block() {
    let up = crate::world_blocks::cluster_placement(slime("steel_cluster"), Direction::Up, false);
    let mut view = MemoryBlockView::default();
    assert!(!crate::world_blocks::cluster_survives(&up, [10, 64, 10], &view), "no support below");
    view.blocks.insert([10, 63, 10], state("minecraft:stone"));
    assert!(crate::world_blocks::cluster_survives(&up, [10, 64, 10], &view), "the stone below supports an up cluster");
    let north = crate::world_blocks::cluster_placement(slime("steel_cluster"), Direction::North, false);
    assert!(!crate::world_blocks::cluster_survives(&north, [10, 64, 10], &view), "the stone below does not support a north cluster");
    view.blocks.insert([10, 64, 11], state("minecraft:stone"));
    assert!(crate::world_blocks::cluster_survives(&north, [10, 64, 10], &view), "the stone behind supports a north cluster");
}

#[test]
fn heads_take_the_placers_yaw_segment() {
    for (yaw, segment) in [(0.0, "0"), (11.0, "0"), (12.0, "1"), (90.0, "4"), (180.0, "8"), (270.0, "12"), (-90.0, "12"), (360.0, "0")] {
        let placed = crate::world_blocks::head_placement(slime("blaze_head"), yaw);
        assert_eq!(placed.property("rotation"), Some(segment), "yaw {yaw}");
    }
}

#[test]
fn wall_heads_pick_the_standing_head() {
    for (wall, head) in WALL_HEADS.iter().zip(HEADS) {
        let pick = crate::world_blocks::wall_head_pick(&slime(wall)).expect("a wall head picks");
        assert_eq!(pick.item.to_string(), format!("tconstruct:{head}"), "{wall}");
        assert_eq!(pick.count, 1, "{wall}");
    }
    assert!(crate::world_blocks::wall_head_pick(&slime("blaze_head")).is_none(), "a standing head is not a wall head");
}

#[test]
fn geode_and_head_piston_rules_and_registration() {
    registered();
    for name in CLUSTERS.iter().chain(&BUDDING).chain(&HEADS) {
        let behaviour = BlockBehaviors::get(&format!("tconstruct:{name}")).unwrap_or_else(|| panic!("{name} registers a behaviour"));
        assert_eq!(behaviour.piston_behavior_override(), Some(PistonBehavior::Destroy), "{name} is destroyed by pistons");
    }
    // The wall heads register only their pick: a piston moves them normally, as the original leaves them.
    for name in WALL_HEADS {
        let behaviour = BlockBehaviors::get(&format!("tconstruct:{name}")).unwrap_or_else(|| panic!("{name} registers a behaviour"));
        assert_eq!(behaviour.piston_behavior_override(), None, "{name} keeps the normal piston behaviour");
    }
}
