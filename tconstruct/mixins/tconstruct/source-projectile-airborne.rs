// The host swaps the acting player's motion and vehicle into Game during a turn.
pub(crate) fn airborne(game: &mut crate::game::Game<'_>) -> bool {
    if game.riding.is_some() { return false; }
    let Some(player) = game.player.as_mut() else { return false; };
    if player.ground.on_ground || player.fluids.was_touching_water { return false; }
    let mut motion = horizon_client::motion::MotionWorld {
        entities: &[], player, scene: &mut game.world, assets: &game.pack.assets,
        collision_binding: game.collision, resources: game.motion,
    };
    super::tconstruct_projectile_stats::airborne_conditions(false, false, false, motion.climbable().unwrap_or(true))
}
