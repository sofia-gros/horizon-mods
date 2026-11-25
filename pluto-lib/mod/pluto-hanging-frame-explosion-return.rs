// HEAD on Game::explosion_breaks_decorations: the frames the paired
// HEAD reach on explosion_breaks_frames stashed (fire-immune: netherite,
// the original's ignoreExplosion) step back into the frame list, in
// their original order. explosion_hurt_entities calls
// explosion_breaks_frames and then explosion_breaks_decorations back to
// back, and decorations never read the frame list, so restoring at this
// head is the frames pass's return point. A RETURN on
// explosion_breaks_frames itself is not weaveable (its body calls
// beneath references, E_UNSUPPORTED_CONTROL_FLOW), and the caller's
// body carries a macro the weave cannot resolve (E_UNSUPPORTED_MACRO).
fn pluto_hanging_frame_explosion_return(game: &mut Game<'_>) {
    crate::game::pluto_hanging_game::restore_immune_frames(game);
}
