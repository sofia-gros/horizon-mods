fn pluto_path_node_dispatch(world: &crate::mobs::MobWorld<'_>, x: i32, y: i32, z: i32) -> std::ops::ControlFlow<PathType> {
    match pluto_block_path::classify(world, [x, y, z]) {
        Some(kind) => std::ops::ControlFlow::Break(kind),
        None => std::ops::ControlFlow::Continue(()),
    }
}
