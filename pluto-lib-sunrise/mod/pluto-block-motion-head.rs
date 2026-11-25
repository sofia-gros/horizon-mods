fn pluto_after_fall_dispatch(world: &mut crate::motion::MotionWorld<'_, '_, '_>, state: sunrise_world::registry::BlockStateId, x: bool, z: bool, resolved: sunrise_core::geometry::Vec3) -> std::ops::ControlFlow<Result<()>> {
    match pluto_block_motion::dispatch(world, state, x, z, resolved) {
        Some(result) => std::ops::ControlFlow::Break(result),
        None => std::ops::ControlFlow::Continue(()),
    }
}
