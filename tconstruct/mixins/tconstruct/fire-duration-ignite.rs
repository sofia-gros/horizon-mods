fn tconstruct_fire_duration_ignite(value: i32, player: &crate::motion::PlayerMotion, ticks: i32) -> i32 {
    crate::tconstruct_fire_duration::explicit_duration(
        value, ticks, *player.status.__horizon_mixins_tconstruct_fire_duration_context(),
    )
}
