// Source Bouncy landing behavior for the local authoritative player.
use super::*;

pub(crate) fn multiplier(world: &mut MotionWorld<'_, '_, '_>, original: f32, contact: bool) -> f32 {
    let player = &mut world.player;
    let state = crate::motion::pluto_landing_state::get_mut(player);
    state.damage_multiplier = None;
    state.rebound_y = None;
    if !contact || !state.enabled || player.server_copy || player.client_side { return original; }
    let distance = player.fall_distance;
    let velocity = player.velocity.y;
    let Ok(step) = player.attribute("minecraft:step_height") else { return original; };
    if (distance < 3.0 && velocity > -0.3) || distance <= 0.5 + (step - 0.6) { return original; }
    if player.shift_key_down { return 0.5; }
    let Ok(gravity) = player.attribute("minecraft:gravity") else { return original; };
    if gravity <= 0.0 || !gravity.is_finite() { return original; }
    crate::motion::pluto_landing_state::get_mut(player).rebound_y = Some((gravity * distance).sqrt());
    0.0
}

pub(super) fn finish(world: &mut MotionWorld<'_, '_, '_>, value: Result<()>) -> Result<()> {
    let rebound = crate::motion::pluto_landing_state::get_mut(world.player).rebound_y.take();
    if value.is_ok() {
        if let Some(y) = rebound {
            world.player.velocity.x /= f64::from(0.975_f32);
            world.player.velocity.y = y;
            world.player.velocity.z /= f64::from(0.975_f32);
            world.player.health.sync_velocity = true;
        }
    }
    value
}
