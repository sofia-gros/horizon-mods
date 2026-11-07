// Private host construction for reusable queries on the game's selected actor.
use super::Game;
use horizon_client::motion::MotionWorld;

/// Query native climbability without advancing a movement tick. The native
/// query may remember the climbable block on the actor; this is not a pure read.
/// `None` means there is no selected actor. Failures are returned unchanged so
/// consumers retain their own conservative fallback and eligibility policy.
pub(super) fn climbable(game: &mut Game<'_>) -> horizon_client::Result<Option<bool>> {
    let Some(player) = game.player.as_mut() else {
        return Ok(None);
    };
    MotionWorld {
        // This query reads blocks at the actor's feet, not entity collisions.
        entities: &[],
        player,
        scene: &mut game.world,
        assets: &game.pack.assets,
        collision_binding: game.collision,
        resources: game.motion,
        // Use the active world's pack/tag-aware health context, not a default
        // or a different MotionResources instance.
        health: &game.health_resources,
    }
    .climbable()
    .map(Some)
}
