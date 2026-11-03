fn tconstruct_bouncy_factor(value: f32, world: &mut crate::motion::MotionWorld<'_, '_, '_>, contact: &Option<(horizon_world::registry::BlockStateId, horizon_core::position::BlockPos, f64)>) -> f32 {
    crate::motion::tconstruct_bouncy_motion::multiplier(world, value, contact.is_some())
}
