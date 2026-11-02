// Source Bouncy landing behavior, with transient state scoped to this world.
use super::*;
use horizon_client_render::chunk_map::pluto_landing_state as landing;

pub(crate) fn multiplier(world: &mut MotionWorld<'_, '_, '_>, original: f32, contact: bool) -> f32 {
    let player = &mut world.player;
    let (enabled, guest, movement_y) = {
        let state = landing::get_mut(&mut world.scene.chunks, player.entity_id);
        state.damage_multiplier = None;
        state.rebound_y = None;
        (state.enabled, state.guest, state.movement_y)
    };
    if !contact || !enabled || player.server_copy || player.client_side { return original; }
    let distance = player.fall_distance;
    let velocity = if guest { movement_y } else { player.velocity.y };
    let Ok(step) = player.attribute("minecraft:step_height") else { return original; };
    if (distance < 3.0 && velocity > -0.3) || distance <= 0.5 + (step - 0.6) { return original; }
    if player.shift_key_down { return 0.5; }
    let Ok(gravity) = player.attribute("minecraft:gravity") else { return original; };
    if gravity <= 0.0 || !gravity.is_finite() { return original; }
    landing::get_mut(&mut world.scene.chunks, player.entity_id).rebound_y = Some((gravity * distance).sqrt());
    0.0
}

pub(super) fn finish(world: &mut MotionWorld<'_, '_, '_>, value: Result<()>) -> Result<()> {
    let state = landing::get_mut(&mut world.scene.chunks, world.player.entity_id);
    if state.guest { return value; }
    let rebound = state.rebound_y.take();
    apply_rebound(world, value, rebound)
}

pub(super) fn finish_guest(world: &mut MotionWorld<'_, '_, '_>, value: Result<()>) -> Result<()> {
    let state = landing::get_mut(&mut world.scene.chunks, world.player.entity_id);
    if !state.guest { return value; }
    let rebound = state.rebound_y.take();
    apply_rebound(world, value, rebound)
}

fn apply_rebound(world: &mut MotionWorld<'_, '_, '_>, value: Result<()>, rebound: Option<f64>) -> Result<()> {
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
