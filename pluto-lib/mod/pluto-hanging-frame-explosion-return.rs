// RETURN on Game::explosion_breaks_frames: the frames the paired HEAD
// reach stashed (fire-immune: netherite, the original's ignoreExplosion)
// step back into the frame list after the blast, in their original order.
fn pluto_hanging_frame_explosion_return(
    value: (),
    game: &mut Game<'_>,
    blast: [f64; 3],
    diameter: f64,
    interaction: super::explosion::BlockInteraction,
) {
    let _ = (value, blast, diameter, interaction);
    crate::game::pluto_hanging_game::restore_immune_frames(game);
}
