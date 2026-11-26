// Applies the brush strokes the mods queued (pluto_brush_access) to the brushable block
// entities of the level in place, as a stroke of the game's own brush lands: the brush
// sound and the dust on a brushable block, then the block entity's own brush count (see
// brush_stroke and brush_block in this module). A stroke queued against a block that is
// not brushable — or no longer is, by the time the seam runs — does nothing, as the
// original's brush call on such a block does nothing. The stroke's tool context (the
// archaeology drop of a completed block rolls against it) is the stack of the use in
// progress; a stroke whose use already ended names a slot past the inventory, whose
// stack reads as none, so its block drops its item with no tool. A stroke that brushed
// nothing queues no stroke, so its generic sound is queued apart (see take_fallbacks)
// and played here for the acting player alone, as the original plays it on the
// brushing player's client only.
use super::*;

pub(crate) fn flush(game: &mut Game<'_>) {
    if !game.native_authoritative() { return; }
    let dimension = game.dim.id();
    let slot = game.using.map(|(slot, _)| slot).unwrap_or(usize::MAX);
    for queued in crate::pluto_brush_access::take_for(dimension) {
        let state = game.world.block(queued.pos[0], queued.pos[1], queued.pos[2]);
        let name = game.res.facts().get(state).name.clone();
        let sound = match &*name {
            "minecraft:suspicious_sand" => "minecraft:item.brush.brushing.sand",
            "minecraft:suspicious_gravel" => "minecraft:item.brush.brushing.gravel",
            _ => continue,
        };
        if game.lan_acting.is_none() {
            let hit = Hit {
                pos: queued.pos,
                face: queued.face,
                point: [
                    f64::from(queued.pos[0]) + 0.5 + f64::from(queued.face[0]) * 0.5,
                    f64::from(queued.pos[1]) + 0.5 + f64::from(queued.face[1]) * 0.5,
                    f64::from(queued.pos[2]) + 0.5 + f64::from(queued.face[2]) * 0.5,
                ],
            };
            game.brush_dust(&hit, slot == crate::game::shield::OFFHAND_SLOT);
        }
        let at = queued.pos.map(|v| f64::from(v) + 0.5);
        game.audio.play(sound, Some(at), 1.0, 1.0);
        game.brush_block(queued.pos, queued.face, slot);
    }
    // The generic sound of the strokes that brushed nothing (BrushModule.onUsingTick's
    // `damage == 0` fallback, `SoundEvents.BRUSH_GENERIC`): the brushing player alone
    // hears it, as the original plays it on their client only. The local player hears
    // it unshared from this client; a guest hears it as a sound sent to them alone —
    // the game's own broadcast sound (`World::play_sound`) would reach the guests near
    // the block instead and never the local player, the reverse of the original.
    for queued in crate::pluto_brush_access::take_fallbacks(dimension) {
        let at = queued.pos.map(|v| f64::from(v) + 0.5);
        if game.lan_acting.is_none() {
            game.audio.play_unshared("minecraft:item.brush.brushing.generic", Some(at), 1.0, 1.0);
        } else {
            let player = sunrise_client_lib::lan::playerdata::ints_uuid(game.acting_player_uuid());
            game.native_play_sound_to(player, "minecraft:item.brush.brushing.generic", "block", Some(at), 1.0, 1.0);
        }
    }
}
