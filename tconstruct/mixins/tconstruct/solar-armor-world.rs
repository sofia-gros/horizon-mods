use super::{Dim, Game, tconstruct_solar_armor_wear as rule};

/// The Game contains the acting guest's player and dimension during as_guest.
/// Never read the renderer's cached `darken` or substitute an unloaded column's
/// full-sky fallback. No holder means no entity-variable fallback for this path.
pub(super) fn chance(game: &Game<'_>, level: i32) -> Option<f32> {
    if level == 0 {
        return Some(0.0);
    }
    if level < 0 || game.remote.is_some() {
        return None;
    }
    let player = game.player.as_ref()?;
    let [x, y, z] =
        rule::block_position([player.position.x, player.position.y, player.position.z])?;
    // Pinned vanilla Nether and End dimension types have no sky light. Host
    // column air above max-Y can report 15, so do not use that for these worlds.
    if game.dim != Dim::Overworld {
        return Some(0.0);
    }
    let chunk = game.world.chunks.get(&(x >> 4, z >> 4))?;
    if !chunk.lit {
        return None;
    }
    let sky = i32::from(chunk.light(x & 15, y, z & 15) >> 4);
    rule::probability(level, sky, game.sky_darken())
}
