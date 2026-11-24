// The game side of a piercing projectile's ignore list (pluto-projectile-impact-access): the head on the native
// impact telling reads the arrow's own list off the live projectile and stashes it for the API side, which builds
// the hit from it. The write-back runs the other way, in the flight pass (pluto-projectile-flight-game): the
// entities a callback recorded are added to the live arrow's list as the tick settles, before it flies again.
use crate::entry::pluto_projectile_impact_access as access;
use crate::mobs::Who;

/// The ignore list the game reports for a projectile, as the impact telling names it: a mob's id, the host player's
/// entity id for the player the game names `Who::Player` (a guest player's entity is one of the game's mobs, so it
/// rides as a mob's id, as the game itself reports it).
fn pierced_ids(game: &crate::game::Game<'_>, p: &crate::projectiles::Projectile) -> Vec<u32> {
    let Some(a) = &p.arrow else { return Vec::new() };
    let player = game.player.as_ref().and_then(|player| u32::try_from(player.entity_id).ok());
    a.pierced
        .iter()
        .map(|who| match who {
            Who::Mob(id) => *id,
            Who::Player => player.unwrap_or(0),
        })
        .collect()
}

/// A projectile's hit is being told: stash the ignore list the game has for it now. The game adds the entity this
/// hit is on to the list before it tells the hit, so the snapshot already holds the victim; a callback that re-deals
/// a piercing hit reads it to know whether the arrow has spent its pierce.
pub(crate) fn on_impact(game: &mut crate::game::Game<'_>, projectile: u32) {
    let Some(p) = game.creatures.projectiles.iter().find(|p| p.id == projectile) else { return };
    access::stash(projectile, pierced_ids(game, p));
}
