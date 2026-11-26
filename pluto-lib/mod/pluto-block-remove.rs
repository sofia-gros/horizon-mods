// Runs the removal's hook inside the removal: the woven head of the game's own block entity removal
// (see pluto-block-remove-head) reports the removal through the sink the bridge installed, so a block
// entity type hears its removal while its block entity is still in the world, and the stacks it keeps
// outside its slots are dropped at the position before the removal goes on. The sink is the only way
// back into the bridge: the game cannot call the mod crate directly.
use super::*;

pub(crate) fn dispatch(game: &mut Game<'_>, pos: BlockPos, old: BlockStateId, side_effects: bool) {
    if !side_effects || !game.native_authoritative() {
        return;
    }
    let key = (pos.x, pos.y, pos.z);
    if !matches!(game.world.block_entities.get(&key), Some(BlockEntity::Mod(_))) {
        return;
    }
    let Some(sink) = crate::entry::pluto_block_remove_access::block_remove_sink() else {
        return;
    };
    let removed = crate::entry::pluto_block_remove_access::BlockRemoved {
        dimension: game.dim.id().to_owned(),
        pos: [pos.x, pos.y, pos.z],
        block: (*game.facts.get(old).name).to_owned(),
    };
    let drops = (sink.removed)(game, removed);
    if !drops.is_empty() {
        game.drop_lost_contents(pos, drops.into_iter().map(Some).collect());
    }
}
