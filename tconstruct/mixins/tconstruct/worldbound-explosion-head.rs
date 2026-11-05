fn tconstruct_worldbound_explosion_head(
    game: &mut crate::game::Game<'_>,
    center: [f64; 3],
    radius: f32,
    damage_type: &str,
    interaction: crate::game::explosion::BlockInteraction,
    blocks: &[[i32; 3]],
) -> std::ops::ControlFlow<Vec<(u32, [f64; 3])>> {
    crate::game::explosion::tconstruct_worldbound_explosion::try_hurt(
        game,
        center,
        radius,
        damage_type,
        interaction,
        blocks,
    )
}
