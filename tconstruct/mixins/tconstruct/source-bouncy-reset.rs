fn tconstruct_bouncy_reset(world: &mut crate::motion::MotionWorld<'_, '_, '_>) {
    let state = crate::motion::pluto_landing_state::get_mut(world.player);
    state.rebound_y = None;
    state.damage_multiplier = None;
}
