fn tconstruct_depth_protection_minimum(status: &crate::effects::StatusFlags) -> std::ops::ControlFlow<f32> {
    let points = *status.__horizon_mixins_tconstruct_depth_protection_points();
    if points.is_finite() && points < 0.0 {
        std::ops::ControlFlow::Break(-20.0)
    } else { std::ops::ControlFlow::Continue(()) }
}
