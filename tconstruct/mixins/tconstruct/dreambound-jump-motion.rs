use crate::motion::MotionWorld;

pub fn attempt(motion: &mut MotionWorld<'_, '_, '_>, capacity: u8) -> crate::Result<bool> {
    if motion.player.ground.on_ground {
        *motion
            .player
            .__horizon_mixins_tconstruct_dreambound_jumps_used_mut() = 0;
        return Ok(false);
    }
    let used = *motion
        .player
        .__horizon_mixins_tconstruct_dreambound_jumps_used();
    if capacity == 0
        || capacity > 4
        || used >= capacity
        || motion.player.flying
        || motion.player.may_fly
        || motion.player.spectator
        || motion.player.is_passenger
        || motion.player.sleeping
        || motion.player.replaying
        || motion.player.health.dead_or_dying()
        || motion.player.fall_flying
        || motion.player.fluids.was_touching_water
        || motion.player.is_in_lava()
    {
        return Ok(false);
    }
    // Negative-gravity input has its own source priority/reset rules; do not
    // consume a Dreambound jump for that unsupported path.
    let gravity = motion.player.attribute("minecraft:gravity")?;
    if !gravity.is_finite() || gravity <= 0.0 || motion.climbable()? {
        return Ok(false);
    }
    motion.jump_from_ground()?;
    *motion
        .player
        .__horizon_mixins_tconstruct_dreambound_jumps_used_mut() = used + 1;
    Ok(true)
}

pub(super) fn input(motion: &mut MotionWorld<'_, '_, '_>, jumping: bool) -> crate::Result<()> {
    if motion.player.ground.on_ground {
        *motion
            .player
            .__horizon_mixins_tconstruct_dreambound_jumps_used_mut() = 0;
    }
    let fresh = crate::tconstruct_dreambound::fresh_jump(
        jumping,
        motion.player.was_jumping,
        motion.player.was_auto_jump,
    );
    let capacity = *motion
        .player
        .__horizon_mixins_tconstruct_dreambound_jump_capacity();
    if fresh && attempt(motion, capacity)? {
        *motion
            .player
            .__horizon_mixins_tconstruct_dreambound_jump_pending_mut() = true;
    }
    Ok(())
}
