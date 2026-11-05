fn tconstruct_boundless_maximum(status: &crate::effects::StatusFlags) -> std::ops::ControlFlow<f32> {
    let cap = *status.__horizon_mixins_tconstruct_boundless_cap();
    if cap.is_finite() && cap > 20.0 && cap <= 23.75 {
        std::ops::ControlFlow::Break(cap)
    } else {
        std::ops::ControlFlow::Continue(())
    }
}
