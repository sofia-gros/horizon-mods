fn pluto_protection_project(value: f32, status: &crate::effects::StatusFlags) -> f32 {
    horizon_world::combat_rules::pluto_protection::project(
        value,
        std::ptr::from_ref(status) as usize,
    )
}
