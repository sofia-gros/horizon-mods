fn tconstruct_dreambound_jump_ground(motion: &mut crate::motion::MotionWorld<'_, '_, '_>) {
    if motion.player.ground.on_ground {
        *motion
            .player
            .__horizon_mixins_tconstruct_dreambound_jumps_used_mut() = 0;
    }
}
