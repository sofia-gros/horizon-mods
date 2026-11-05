fn tconstruct_dreambound_jump_land(
    motion: &mut crate::motion::MotionWorld<'_, '_, '_>,
    grounded: bool,
) {
    if grounded {
        *motion
            .player
            .status
            .__horizon_mixins_tconstruct_dreambound_jumps_used_mut() = 0;
    }
}
