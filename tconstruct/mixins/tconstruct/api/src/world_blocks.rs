//! TinkerWorld's sticky predicates and CongealedSlimeBlock's landing/contact rules (SlimeKnights, MIT).
//! Below them, the geodes and heads: CrystalClusterBlock placement and survival, the budding blocks' piston rule,
//! SkullBlock rotation and the wall heads' pick. Not expressible yet: budding growth (a later unit; the tick is
//! declared in the content), the crystal chime on projectile hit (no projectile-hit hook), the head item placing
//! the wall variant (the content schema's wall blocks are signs), skull rendering and animation (the host has no
//! skull concept for content blocks) and the enderman head counting as a mask (disguises are modifier-driven).
use pluto_lib::{AfterFallContext, BlockBehavior, BlockBehaviors, BlockState, BlockView, Direction, EntityInsideMotionContext, Identifier, ItemStack, PathNodeType, PistonBehavior, Placement};
use pluto_lib::{ItemTooltipCallback, TooltipLine};

pub(crate) fn register() {
    static TOOLTIPS: std::sync::Once = std::sync::Once::new();
    TOOLTIPS.call_once(|| {
        ItemTooltipCallback::EVENT.register(|tip| {
            if let Some(name @ ("cobalt_shard" | "steel_shard" | "knightmetal_shard")) = tip.item.strip_prefix("tconstruct:") {
                tip.added.borrow_mut().push(TooltipLine::translatable(format!("item.tconstruct.{name}.tooltip")));
            }
        });
    });
    let id = |name| Identifier::new("tconstruct", name).unwrap();
    for name in ["sky_slime", "ichor_slime", "ender_slime"] {
        BlockBehaviors::register(BlockBehavior::new(id(name))
            .is_slime_block(|_| true)
            .is_sticky_block(|_| true)
            .can_stick_to(move |state, other| match name {
                "ichor_slime" => state.block != other.block,
                "ender_slime" => state.block == other.block,
                _ => true,
            }));
    }
    for name in ["earth_congealed_slime", "sky_congealed_slime", "ichor_congealed_slime", "ender_congealed_slime"] {
        BlockBehaviors::register(BlockBehavior::new(id(name))
            .fall_on(|c| c.damage = 0.0)
            .update_entity_after_fall_on(after_fall)
            .entity_inside_motion(inside)
            .path_node_type(|_| Some(PathNodeType::StickyHoney)));
    }
    for name in CLUSTERS {
        BlockBehaviors::register(BlockBehavior::new(id(name))
            .state_for_placement(|c| Placement::State(cluster_placement(c.state.clone(), c.face, c.in_water)))
            .can_survive(|c| cluster_survives(&c.state, c.pos, c.view))
            .piston_behavior(PistonBehavior::Destroy));
    }
    // BuddingCrystalBlock is destroyed by pistons too; its random tick is declared in the content, and the growth
    // it drives is a later unit.
    for name in BUDDING {
        BlockBehaviors::register(BlockBehavior::new(id(name)).piston_behavior(PistonBehavior::Destroy));
    }
    for name in HEADS {
        BlockBehaviors::register(BlockBehavior::new(id(name))
            .state_for_placement(|c| Placement::State(head_placement(c.state.clone(), c.yaw)))
            .piston_behavior(PistonBehavior::Destroy));
    }
    // The wall heads carry no item of their own: picking one gives the standing head, the item the original's
    // StandingAndWallBlockItem is registered for.
    for name in WALL_HEADS {
        BlockBehaviors::register(BlockBehavior::new(id(name)).pick_stack(|c| wall_head_pick(&c.state)));
    }
}

/// The CrystalClusterBlocks: each geode's cluster and three buds, then the three metal clusters (TinkerWorld).
pub(crate) const CLUSTERS: [&str; 19] = [
    "earth_slime_crystal_cluster", "small_earth_slime_crystal_bud", "medium_earth_slime_crystal_bud", "large_earth_slime_crystal_bud",
    "sky_slime_crystal_cluster", "small_sky_slime_crystal_bud", "medium_sky_slime_crystal_bud", "large_sky_slime_crystal_bud",
    "ichor_slime_crystal_cluster", "small_ichor_slime_crystal_bud", "medium_ichor_slime_crystal_bud", "large_ichor_slime_crystal_bud",
    "ender_slime_crystal_cluster", "small_ender_slime_crystal_bud", "medium_ender_slime_crystal_bud", "large_ender_slime_crystal_bud",
    "steel_cluster", "cobalt_cluster", "knightmetal_cluster",
];

/// The BuddingCrystalBlocks (the growth on their declared random tick is a later unit).
pub(crate) const BUDDING: [&str; 4] = ["budding_earth_slime_crystal", "budding_sky_slime_crystal", "budding_ichor_slime_crystal", "budding_ender_slime_crystal"];

/// TinkerHeadType's standing heads, in its order.
pub(crate) const HEADS: [&str; 9] = ["blaze_head", "enderman_head", "stray_head", "husk_head", "drowned_head", "spider_head", "cave_spider_head", "piglin_brute_head", "zombified_piglin_head"];

/// The wall variants of HEADS, in the same order.
pub(crate) const WALL_HEADS: [&str; 9] = ["blaze_wall_head", "enderman_wall_head", "stray_wall_head", "husk_wall_head", "drowned_wall_head", "spider_wall_head", "cave_spider_wall_head", "piglin_brute_wall_head", "zombified_piglin_wall_head"];

/// AmethystClusterBlock.getStateForPlacement: the cluster points along the clicked face and keeps the water it
/// was placed in.
pub(crate) fn cluster_placement(state: BlockState, face: Direction, in_water: bool) -> BlockState {
    state.with("facing", face.name()).with("waterlogged", if in_water { "true" } else { "false" })
}

/// AmethystClusterBlock.canSurvive: the block behind the point must support it. The original asks for centre
/// support specifically; the host's view answers the one sturdy-face question it has.
pub(crate) fn cluster_survives(state: &BlockState, pos: [i32; 3], view: &dyn BlockView) -> bool {
    let Some(facing) = state.property("facing").and_then(Direction::parse) else { return false };
    view.is_face_sturdy(facing.opposite().relative(pos), facing)
}

/// RotationSegment.convertToSegment: a segment every 22.5 degrees, centred on the segment's angle.
pub(crate) fn rotation_segment(yaw: f32) -> i32 { ((f64::from(yaw) / 22.5) + 0.5).floor() as i32 & 15 }

/// SkullBlock.getStateForPlacement: the floor head takes the placer's yaw segment.
pub(crate) fn head_placement(state: BlockState, yaw: f32) -> BlockState {
    let segment = rotation_segment(yaw).to_string();
    state.with("rotation", &segment)
}

/// A wall head picks as its standing head.
pub(crate) fn wall_head_pick(state: &BlockState) -> Option<ItemStack> {
    let head = state.block.path().strip_suffix("_wall_head")?;
    Some(ItemStack::new(Identifier::new("tconstruct", &format!("{head}_head")).ok()?, 1))
}

fn after_fall(c: &mut AfterFallContext) {
    if !c.suppressing_bounce && (c.living || c.item) && c.velocity[1] < 0.0 {
        c.velocity[1] *= if c.living { -1.0 } else { -0.8 };
        c.fall_distance = 0.0;
        if c.item { c.on_ground = false; }
    }
}

fn inside(c: &mut EntityInsideMotionContext) {
    let y = c.position[1] - f64::from(c.pos[1]);
    if !c.server || c.suppressing_bounce || !(-0.0625..0.9365).contains(&y) { return; }
    let x = c.position[0] - (f64::from(c.pos[0]) + 0.5);
    let z = c.position[2] - (f64::from(c.pos[2]) + 0.5);
    let (axis, direction, across) = if x.abs() > z.abs() { (0, x, z) } else { (2, z, x) };
    if across.abs() >= 0.495 { return; }
    let speed = c.position.iter().zip(c.previous_position).map(|(now, old)| (now - old).powi(2)).sum::<f64>().sqrt() * 0.95;
    // Java Math.signum(0) is zero; Rust's signum(0) is one.
    c.velocity[axis] = if direction == 0.0 { 0.0 } else { speed.copysign(direction) };
    c.sync_velocity = true;
    if speed > 0.1 { c.step_sound = Some([1.0, 1.0]); }
}
