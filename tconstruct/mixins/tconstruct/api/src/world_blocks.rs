//! TinkerWorld's sticky predicates and CongealedSlimeBlock's landing/contact rules (SlimeKnights, MIT).
use pluto_lib::{AfterFallContext, BlockBehavior, BlockBehaviors, EntityInsideMotionContext, Identifier, PathNodeType};
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
