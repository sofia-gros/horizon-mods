fn pluto_block_fall_dispatch(world: &crate::motion::MotionWorld<'_, '_, '_>, state: sunrise_world::registry::BlockStateId, position: sunrise_core::position::BlockPos, distance: f64, damage: f32) -> std::ops::ControlFlow<f32> {
    match pluto_block_fall::dispatch(world, state, position, distance, damage) {
        Some(damage) => std::ops::ControlFlow::Break(damage),
        None => std::ops::ControlFlow::Continue(()),
    }
}
