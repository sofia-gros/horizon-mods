fn pluto_player_protection(
    player: &mut crate::motion::PlayerMotion,
    resources: &crate::health::HealthResources,
    rules: crate::health::HealthRules,
    source: sunrise_world::damage_source::DamageTypeId,
    amount: f32,
    blocking: &Option<crate::health::Blocking>,
    living_cause: bool,
    causing: i32,
) -> std::ops::ControlFlow<Result<BlockOutcome>> {
    if sunrise_world::combat_rules::pluto_protection::take_reentry() {
        return std::ops::ControlFlow::Continue(());
    }
    if !sunrise_client_lib::pluto_protection::enabled() {
        return std::ops::ControlFlow::Continue(());
    }
    std::ops::ControlFlow::Break(sunrise_client_lib::pluto_protection::run(
        player,
        resources,
        rules,
        source,
        amount,
        blocking.clone(),
        living_cause,
        causing,
        None,
    ))
}
