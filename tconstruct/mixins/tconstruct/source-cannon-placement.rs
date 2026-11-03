fn tconstruct_cannon_placement(level: &crate::game::GameLevel<'_>, state: horizon_world::registry::BlockStateId,
    context: &horizon_world::level::PlacementContext, answer: &mut Option<Option<horizon_world::registry::BlockStateId>>) {
    use horizon_core::geometry::{Direction, trig};
    if answer.is_some() || &*level.facts.get(state).name != "tconstruct:seared_fluid_cannon" { return; }
    let pitch = context.pitch * (std::f32::consts::PI / 180.0); let yaw = -context.yaw * (std::f32::consts::PI / 180.0);
    let (ps,pc,ys,yc) = (trig::sin(f64::from(pitch)),trig::cos(f64::from(pitch)),trig::sin(f64::from(yaw)),trig::cos(f64::from(yaw)));
    let (x,y,z) = (if ys > 0.0 {Direction::East} else {Direction::West}, if ps < 0.0 {Direction::Up} else {Direction::Down}, if yc > 0.0 {Direction::South} else {Direction::North});
    let (xy,ym,zy) = (ys.abs(),ps.abs(),yc.abs()); let (xm,zm) = (xy*pc,zy*pc);
    let nearest = if xy > zy { if ym > xm {y} else {x} } else if ym > zm {y} else {z};
    let name = match nearest.opposite() { Direction::Down => "down", Direction::Up => "up", Direction::North => "north", Direction::South => "south", Direction::West => "west", Direction::East => "east" };
    let Some(mut value) = level.registry.state(state).cloned() else { *answer = Some(None); return; };
    value.properties.insert("facing".into(),name.into());
    *answer = Some(level.registry.resolve(&value).ok());
}
