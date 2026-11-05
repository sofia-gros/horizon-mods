fn tconstruct_fire_protection_damage_health(status: &crate::effects::StatusFlags, damage_type: &str) -> std::ops::ControlFlow<f32> {
    let extra = status.__horizon_mixins_tconstruct_fire_protection_points().iter()
        .find(|(kind,_)| kind == damage_type).map_or(0.0,|(_,points)| *points);
    if !extra.is_finite() || extra <= 0.0 { return std::ops::ControlFlow::Continue(()); }
    // Native table lookup, unchanged. The native damage kernel applies this
    // combined EPF only after armor/resistance, and clamps once at twenty.
    let native = status.protection.iter().find(|(kind,_)| kind == damage_type).map_or(0.0,|(_,points)| *points);
    let total = native + extra;
    if !total.is_finite() { return std::ops::ControlFlow::Continue(()); }
    std::ops::ControlFlow::Break(total)
}
