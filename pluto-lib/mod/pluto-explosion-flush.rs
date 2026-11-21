// Sets off the explosions the mods queued (pluto_explosion_access) in the level in place, as
// `Level.explode` does. The game's own native_explode checks the centre, the radius and the
// loaded chunk again, so a request whose moment has passed is simply refused there. The plan
// guard pins this phase: do not move the flush to a client frame or infer a dimension from a
// global queue.
use super::*;

pub(crate) fn flush(game: &mut Game<'_>) {
    if !game.native_authoritative() { return; }
    let dimension = game.dim.id();
    for queued in crate::entry::pluto_explosion_access::take_for(dimension) {
        let blast = match queued.blast {
            crate::entry::pluto_explosion_access::Blast::Keep => crate::game::native_explosions::NativeBlast::Keep,
            crate::entry::pluto_explosion_access::Blast::Block => crate::game::native_explosions::NativeBlast::Block,
            crate::entry::pluto_explosion_access::Blast::Tnt => crate::game::native_explosions::NativeBlast::Tnt,
        };
        game.native_explode(dimension, queued.center, queued.radius, queued.fire, blast);
    }
}
