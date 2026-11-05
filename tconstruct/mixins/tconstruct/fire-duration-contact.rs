fn tconstruct_fire_duration_contact(value: f64, player: &crate::motion::PlayerMotion) -> f64 {
    crate::tconstruct_fire_duration::contact_multiplier(
        value, *player.__horizon_mixins_tconstruct_fire_duration_context(),
    )
}
