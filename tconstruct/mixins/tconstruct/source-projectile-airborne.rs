// The host swaps the acting player's motion and vehicle into Game during a turn.
pub(crate) fn airborne(game: &mut crate::game::Game<'_>) -> bool {
    if game.riding.is_some() { return false; }
    let Some(player) = game.player.as_mut() else { return false; };
    if player.ground.on_ground || horizon_client::pluto_fluid_facts::player(player).touching_water { return false; }
    let climbable = super::pluto_motion_query::climbable(game).ok().flatten().unwrap_or(true);
    super::tconstruct_projectile_stats::airborne_conditions(false, false, false, climbable)
}
