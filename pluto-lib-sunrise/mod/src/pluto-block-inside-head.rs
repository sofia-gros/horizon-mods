fn pluto_inside_motion(world: &mut crate::motion::MotionWorld<'_, '_, '_>, from: sunrise_core::geometry::Vec3, to: sunrise_core::geometry::Vec3, visited: &mut crate::fast_hash::FxHashSet<i64>, entered: &mut Vec<(i32, sunrise_core::position::BlockPos)>, max_iterations: i32, damage_only: bool) -> std::ops::ControlFlow<Result<i32>> {
    std::ops::ControlFlow::Break(world.pluto_check_inside_blocks(from, to, visited, entered, max_iterations, damage_only))
}
