fn tconstruct_connected_pane_face(
    visibility: &FaceVisibility, state: BlockStateId, neighbor: BlockStateId, direction: Direction,
) -> std::ops::ControlFlow<std::io::Result<bool>> {
    use std::ops::ControlFlow::{Break, Continue};
    if !matches!(direction, Direction::Up | Direction::Down) { return Continue(()); }
    let Ok(current) = visibility.state(state) else { return Continue(()); };
    if !visibility.__horizon_mixins_tconstruct_connected_pane_ids().contains(&current.block) { return Continue(()); }
    let Ok(other) = visibility.state(neighbor) else { return Continue(()); };
    if current.block != other.block { return Continue(()); }
    Break(Ok(tconstruct_pane_culling::uncovered(current.connections, other.connections)))
}
