// The two ends of the block removal seam (#1196). A block entity's removal runs inside the
// game's level operations (GameLevel::remove_entity), where no Game — and so no bridge Host —
// exists, so the seam cannot run a type's removal hook inside the removal the way the game's
// own spill runs. Instead the head of the removal captures the removal (see
// pluto-block-remove-head): the block entity is still in the world there, and its saved data
// and item slots are copied into the access module's queue. The head of Game::flush_spill
// (see pluto-block-remove-drain-head) ends every level operation and materialises its drops,
// so it drains the queue there: the bridge sink runs the type's hook over the captured data,
// and the stacks it answers are dropped at the position in the same flush as the game's own
// spill of the slots. The sink is the only way back into the bridge: the game cannot call the
// mod crate directly.
use super::*;

pub(crate) fn capture(level: &mut GameLevel<'_>, pos: BlockPos, old: BlockStateId, side_effects: bool) {
    if !side_effects || !level.native_authoritative {
        return;
    }
    if saved_entity_waits(level, pos) {
        return;
    }
    let key = (pos.x, pos.y, pos.z);
    let Some(BlockEntity::Mod(entity)) = level.world.block_entities.get(&key) else {
        return;
    };
    let removed = crate::entry::pluto_block_remove_access::BlockRemoved {
        dimension: dimension_id(level.dimension),
        pos: [pos.x, pos.y, pos.z],
        block: (*level.facts.get(old).name).to_owned(),
        nbt: entity.nbt.clone(),
        items: entity.items.clone(),
    };
    crate::entry::pluto_block_remove_access::stash_removal(removed);
}

pub(crate) fn drain(game: &mut Game<'_>) {
    if !game.native_authoritative() {
        return;
    }
    let Some(sink) = crate::entry::pluto_block_remove_access::block_remove_sink() else {
        return;
    };
    let removed = crate::entry::pluto_block_remove_access::take_removals_for(game.dim.id());
    for removed in removed {
        let pos = BlockPos::new(removed.pos[0], removed.pos[1], removed.pos[2]);
        let drops = (sink.removed)(game, removed);
        for stack in drops {
            drop_stack(game, pos, stack);
        }
    }
}

/// Whether a saved (never materialised) block entity tag waits at `pos`: the game's own
/// removal runs that one first and returns before the live path (GameLevel::remove_entity's
/// remove_saved_entity), so a live capture must not answer for the same position.
fn saved_entity_waits(level: &GameLevel<'_>, pos: BlockPos) -> bool {
    use horizon_world::nbt::Tag;
    let chunk_key = (pos.x >> 4, pos.z >> 4);
    let Some(chunk) = level.world.chunks.get(&chunk_key) else {
        return false;
    };
    let Some(saved) = chunk.saved.as_ref() else {
        return false;
    };
    saved.block_entities.iter().any(|tag| {
        let int = |key: &str| match tag.get(key) {
            Some(Tag::Int(value)) => Some(*value),
            _ => None,
        };
        int("x") == Some(pos.x) && int("y") == Some(pos.y) && int("z") == Some(pos.z)
    })
}

/// The id the bridge names a dimension by, for the level's dimension (the game's own
/// Dim::id spellings, game.rs).
fn dimension_id(dimension: horizon_world::level::Dimension) -> String {
    match dimension {
        horizon_world::level::Dimension::Overworld => "minecraft:overworld",
        horizon_world::level::Dimension::Nether => "minecraft:the_nether",
        horizon_world::level::Dimension::End => "minecraft:the_end",
    }
    .to_owned()
}

/// `Containers.dropContents` of one stack at `pos`, as the level's own spill draws it
/// (GameLevel::drop_item_stack): the spawns join the game's spill queue, and the flush
/// this drain heads materialises them with the rest of the operation's drops.
fn drop_stack(game: &mut Game<'_>, pos: BlockPos, stack: ItemStack) {
    if stack.count == 0 || &*stack.item == "minecraft:air" {
        return;
    }
    for piece in horizon_world::level::removal::drop_item_stack(&mut game.world.random, pos, stack.count) {
        game.spill.spawns.push(super::spill::Spawn {
            stack: ItemStack { count: piece.count, ..stack.clone() },
            pos: piece.pos,
            motion: Some(piece.motion),
            delay: 0,
        });
    }
}
