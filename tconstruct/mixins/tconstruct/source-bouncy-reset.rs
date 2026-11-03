fn tconstruct_bouncy_reset(world: &mut crate::motion::MotionWorld<'_, '_, '_>, dy: f64) {
    let state = horizon_client_render::chunk_map::pluto_landing_state::get_mut(&mut world.scene.chunks, world.player.entity_id);
    state.rebound_y = None;
    state.damage_multiplier = None;
    state.movement_y = dy;
}
