fn tconstruct_protection_damage_health(status: &crate::effects::StatusFlags, damage_type: &str) -> std::ops::ControlFlow<f32> {
    let mut extra = status.__horizon_mixins_tconstruct_protection_points().iter()
        .find(|(kind,_)| kind == damage_type).map_or(0.0,|(_,points)| *points);
    // Only a verified native melee entry point supplies this transient context.
    // Health's own DamageSource<()> has already lost direct/causing identity.
    if status.__horizon_mixins_tconstruct_melee_protection_source().as_deref() == Some(damage_type) {
        extra += status.__horizon_mixins_tconstruct_melee_protection_points().iter()
            .find(|(kind,_)| kind == damage_type).map_or(0.0,|(_,points)| *points);
    }
    if status.__horizon_mixins_tconstruct_scorch_protection_source().as_deref() == Some(damage_type) {
        extra += *status.__horizon_mixins_tconstruct_scorch_protection_points();
    }
    if status.__horizon_mixins_tconstruct_consecrated_source().as_deref() == Some(damage_type) {
        extra += *status.__horizon_mixins_tconstruct_consecrated_points();
    }
    extra += *status.__horizon_mixins_tconstruct_depth_protection_points();
    extra += *status.__horizon_mixins_tconstruct_turtle_shell_points();
    if !extra.is_finite() || extra == 0.0 { return std::ops::ControlFlow::Continue(()); }
    // Native table lookup, unchanged. The native damage kernel applies this
    // combined EPF only after armor/resistance, and clamps once. Depth Protection
    // opts negative totals in only when its high-altitude penalty is active.
    let native = status.protection.iter().find(|(kind,_)| kind == damage_type).map_or(0.0,|(_,points)| *points);
    let total = native + extra;
    if !total.is_finite() { return std::ops::ControlFlow::Continue(()); }
    std::ops::ControlFlow::Break(total)
}
