fn pluto_protection_arm(environment: &crate::health::Environment<'_>) {
    horizon_world::combat_rules::pluto_protection::arm(
        std::ptr::from_ref(&environment.status) as usize
    );
}
