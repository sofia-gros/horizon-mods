fn pluto_protection_kernel(damage: f32, protection: f32) -> std::ops::ControlFlow<f32> {
    match crate::combat_rules::pluto_protection::after_magic(damage, protection) {
        Some(value) => std::ops::ControlFlow::Break(value),
        None => std::ops::ControlFlow::Continue(()),
    }
}
