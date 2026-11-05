fn tconstruct_fire_duration_ignite(value: i32, player: &mut crate::motion::PlayerMotion, ticks: i32) -> i32 {
    let Some((level, native_factor)) = *player.__horizon_mixins_tconstruct_fire_duration_context() else { return value; };
    // Never override command/mod changes to the native burning-time attribute.
    if player.attribute("minecraft:burning_time").ok() != Some(native_factor) { return value; }
    crate::tconstruct_fire_duration::duration(ticks, level).unwrap_or(value)
}
