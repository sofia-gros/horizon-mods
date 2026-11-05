fn tconstruct_depth_protection_height(player: &mut crate::motion::PlayerMotion) {
    // Read the actual hit's Y, not the previous tick's or Game render position.
    let points = crate::tconstruct_depth_protection::points(
        player.position.y,player.status.__horizon_mixins_tconstruct_depth_protection_levels(),
    ).unwrap_or(0.0);
    *player.status.__horizon_mixins_tconstruct_depth_protection_points_mut() = points;
}
