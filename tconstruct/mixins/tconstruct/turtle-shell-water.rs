fn tconstruct_turtle_shell_water(player: &mut crate::motion::PlayerMotion) {
    // Mantle's eyes_in_water is wasEyeInWater, not a new eye-fluid query.
    // Its feet_in_water is Entity.isInWater, the body's native water flag.
    let points = crate::tconstruct_turtle_shell::points(
        player.status.__horizon_mixins_tconstruct_turtle_shell_levels(),
        player.fluids.was_eye_in_water,player.fluids.was_touching_water,
    ).unwrap_or(0.0);
    *player.status.__horizon_mixins_tconstruct_turtle_shell_points_mut() = points;
}
