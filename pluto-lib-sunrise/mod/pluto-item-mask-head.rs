// Adapted from pinned MIT Horizon mobs/enderman.rs::stared_at.
// License: pluto-lib/LICENSE.horizon. Unregistered queries run the original.
fn pluto_item_ender_mask(m: &Mob, w: &MobWorld<'_>) -> std::ops::ControlFlow<bool> {
    let Some(p) = &w.player else { return std::ops::ControlFlow::Continue(()) };
    let Some(item) = p.head.as_deref() else { return std::ops::ControlFlow::Continue(()) };
    let mut uuid = 0_u128;
    for word in p.uuid { uuid = (uuid << 32) | u128::from(word as u32); }
    let Some(mask) = sunrise_world::level::pluto_item_behavior_access::mask(item, uuid, m.id) else {
        return std::ops::ControlFlow::Continue(());
    };
    if !p.alive || mask { return std::ops::ControlFlow::Break(false); }
    // Explicit false must bypass the native pumpkin check, not become None.
    // Preserve its exact eye/cone/line-of-sight calculation for that branch.
    let look = p.look;
    let dir = [m.position.x - p.position.x, m.eye_y() - p.eye[1], m.position.z - p.position.z];
    let dist = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
    if dist < 1.0e-6 { return std::ops::ControlFlow::Break(false); }
    let dot = (look[0] * dir[0] + look[1] * dir[1] + look[2] * dir[2]) / dist;
    std::ops::ControlFlow::Break(dot > 1.0 - 0.025 / dist && w.line_of_sight(
        Vec3::new(p.eye[0], p.eye[1], p.eye[2]), Vec3::new(m.position.x, m.eye_y(), m.position.z)))
}
