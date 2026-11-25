//! Counterpart of NeoForge's `BlockEvent.EntityPlaceEvent` (the Forge `onBlockPlace`): a
//! block about to be placed by a player or a walk module, heard before the block lands.
//!
//! The event carries the placer, the dimension, the position, the state that would be placed
//! and the state it replaces; a callback sets `cancel` to stop the placement. It fires only
//! where a placer asks for it — the walk hook fires it before a frost walker replacement
//! lands. The game's own place hook does not fire it: that hook runs after the block is
//! already there, so a cancellable event there could not un-place it. There are no listeners
//! in the tree yet: protection and claim mods are the callers this surface is for.

use std::cell::Cell;

use super::base::Event;
use super::block_entity_api_v1::BlockPos;
use super::world_api_v1::BlockState;

/// A block about to be placed.
#[derive(Clone, Debug)]
pub struct BlockPlace {
    /// The placer's uuid, when a player places it; None for a placement no player owns.
    pub placer: Option<u128>,
    /// The dimension, such as `minecraft:overworld`.
    pub dimension: String,
    /// Where the block would go.
    pub pos: BlockPos,
    /// The state that would be placed.
    pub state: BlockState,
    /// The state it replaces.
    pub replaced: BlockState,
    /// Set to true to stop the placement.
    pub cancel: Cell<bool>,
}

impl BlockPlace {
    /// A placement of `state` at `pos` in `dimension`, replacing `replaced`, by `placer`.
    pub fn new(placer: Option<u128>, dimension: &str, pos: BlockPos, state: BlockState, replaced: BlockState) -> Self {
        Self { placer, dimension: dimension.to_owned(), pos, state, replaced, cancel: Cell::new(false) }
    }
}

static BLOCK_PLACE: Event<BlockPlace, ()> = Event::void();

/// NeoForge: `BlockEvent.EntityPlaceEvent`.
pub struct ServerBlockEvents;

impl ServerBlockEvents {
    /// A block is about to be placed by a player or a walk module (NeoForge:
    /// `BlockEvent.EntityPlaceEvent`, the Forge `onBlockPlace`): set `cancel` to stop the
    /// placement. Heard before the block lands, so a later callback may still cancel it.
    pub const BLOCK_PLACE: &'static Event<BlockPlace, ()> = &BLOCK_PLACE;
}

/// Fires [`ServerBlockEvents::BLOCK_PLACE`] for `place`: whether any callback cancelled it.
pub fn block_place(place: &BlockPlace) -> bool {
    BLOCK_PLACE.invoke(place);
    place.cancel.get()
}

/// Whether any callback hears [`ServerBlockEvents::BLOCK_PLACE`].
pub fn block_place_heard() -> bool {
    !BLOCK_PLACE.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_place_is_heard_and_a_cancel_stops_it() {
        // The event is process-wide, so the listener cancels only the frosted ice
        // it is here for, and nothing else a later test fires.
        ServerBlockEvents::BLOCK_PLACE.register(|place| {
            if place.state.block.to_string() == "minecraft:frosted_ice" {
                place.cancel.set(true);
            }
        });
        assert!(block_place_heard());
        let place = BlockPlace::new(Some(42), "minecraft:overworld", [0, 64, 0],
            BlockState::parse("minecraft:frosted_ice").unwrap(),
            BlockState::parse("minecraft:water[level=0]").unwrap());
        assert!(block_place(&place));
        let other = BlockPlace::new(Some(42), "minecraft:overworld", [1, 64, 0],
            BlockState::parse("minecraft:stone").unwrap(),
            BlockState::parse("minecraft:water[level=0]").unwrap());
        assert!(!block_place(&other));
    }
}
