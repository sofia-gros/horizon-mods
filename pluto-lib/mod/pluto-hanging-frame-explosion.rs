// HEAD + RETURN on Game::explosion_breaks_frames: fire-immune frames
// (netherite, the original's ignoreExplosion) step out of the frame list
// for the blast and back in afterwards, in their original order.
fn pluto_hanging_frame_explosion_head(
    game: &mut Game<'_>,
    blast: [f64; 3],
    diameter: f64,
    interaction: super::explosion::BlockInteraction,
) {
    let _ = (blast, diameter, interaction);
    crate::game::pluto_hanging_game::stash_immune_frames(game);
}

fn pluto_hanging_frame_explosion_return(
    game: &mut Game<'_>,
    blast: [f64; 3],
    diameter: f64,
    interaction: super::explosion::BlockInteraction,
) {
    let _ = (blast, diameter, interaction);
    crate::game::pluto_hanging_game::restore_immune_frames(game);
}
