// `look_at_entity` handles mobs and objects but not another guest. Permit the
// already range-checked guest interaction only for the packet's splashing
// hand, then let the normal per-guest interaction and queue process it.
fn tconstruct_splashing_guest_aim(game: &mut crate::game::Game<'_>, host: &mut crate::game::lan::LanHost, gi: usize, id: i32, location: Option<[f64; 3]>) -> std::ops::ControlFlow<Option<([f64; 3], [f64; 3])>> {
    use std::ops::ControlFlow::{Break, Continue};
    use horizon_world::nbt::Tag;
    let Some(actor) = host.hub.guests.get(gi) else { return Continue(()); };
    let Some((packet_id, offhand)) = game.world.chunks.__horizon_mixins_tconstruct_splashing_guest_packet().get(&actor.conn).copied() else { return Continue(()); };
    if packet_id != id { return Continue(()); }
    let held = if offhand { actor.offhand.as_ref() } else { actor.inventory.get(actor.selected).and_then(Option::as_ref) };
    let enabled = held.and_then(|stack| stack.data()).and_then(|data| data.other("minecraft:custom_data")).and_then(|data| data.get("tic_upgrades")).and_then(|upgrades| upgrades.get("tconstruct:splashing"));
    if !matches!(enabled, Some(Tag::Int(level)) if *level > 0) { return Continue(()); }
    let Some(target) = host.hub.guests.iter().find(|target| target.conn != actor.conn && target.entity_id == id && target.placed && target.motion.is_some() && target.dimension == actor.dimension) else { return Continue(()); };
    let bounds = target.bounds(); let lo = bounds.min_position(); let hi = bounds.max_position();
    let lower = [lo.x, lo.y, lo.z]; let upper = [hi.x, hi.y, hi.z];
    let origin = target.position;
    let centre = match location {
        None => std::array::from_fn(|axis| (lower[axis] + upper[axis]) / 2.0),
        Some(location) => {
            let point: [f64; 3] = std::array::from_fn(|axis| origin[axis] + location[axis]);
            let inside: [f64; 3] = std::array::from_fn(|axis| {
                let margin = ((upper[axis] - lower[axis]) / 2.0).min(0.01);
                point[axis].clamp(lower[axis] + margin, upper[axis] - margin)
            });
            if (0..3).all(|axis| (point[axis] - inside[axis]).abs() < 0.1) { point }
            else { std::array::from_fn(|axis| (lower[axis] + upper[axis]) / 2.0) }
        }
    };
    let eye = [actor.position[0], actor.position[1] + actor.eye_height(), actor.position[2]];
    let delta = [centre[0] - eye[0], centre[1] - eye[1], centre[2] - eye[2]];
    let horizontal = (delta[0] * delta[0] + delta[2] * delta[2]).sqrt();
    let actor = &mut host.hub.guests[gi];
    actor.yaw = (-delta[0].atan2(delta[2])).to_degrees() as f32;
    actor.pitch = (-delta[1].atan2(horizontal)).to_degrees() as f32;
    Break(Some((centre, eye)))
}
