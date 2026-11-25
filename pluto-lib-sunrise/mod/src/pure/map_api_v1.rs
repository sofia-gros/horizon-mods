//! A map stored in a worn piece, ticked while it is worn (Tinkers' minimap module): the
//! stored map's id, answered by the mod that owns the storage.
//!
//! The host cannot parse a mod's storage layout, so the game asks instead of reading: once per
//! tick, while the maps tick runs, the game hands the worn head stack to the registered handler
//! and the handler answers the id of the stored map to tick, if any. The game then swaps a
//! filled map with that id into the offhand for the vanilla map tick, swaps the old offhand
//! stack back, and sends a guest the update packet when anything changed. A handler answers
//! `None` for a head it does not own (no minimap module, no stored map, no map id), and the
//! first `Some` wins.

use super::base::Event;
use super::block_entity_api_v1::ItemSlot;

/// The worn head stack a map-tick handler is asked about.
#[derive(Clone, Debug, PartialEq)]
pub struct MapTickRequest {
    /// The worn head stack: the piece whose stored map may tick.
    pub head: ItemSlot,
}

static MAP_TICK: Event<MapTickRequest, Option<i32>> = Event::first_some();

/// The stored map's id for a worn head stack (see [`MapTickRequest`]): the mod that owns the
/// storage registers one callback answering the id of the map to tick, or `None` for a head it
/// does not own. The first `Some` wins.
pub struct MapTickCallback;

impl MapTickCallback {
    pub const EVENT: &'static Event<MapTickRequest, Option<i32>> = &MAP_TICK;
}

/// The stored map's id for a worn head stack, through the registered handler; `None` when no
/// handler answers.
pub(crate) fn map_tick(request: &MapTickRequest) -> Option<i32> {
    MAP_TICK.invoke(request)
}

/// Whether a map-tick handler listens, so the game only reads the head stack when someone does.
pub(crate) fn map_tick_heard() -> bool {
    !MAP_TICK.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pure::base::Identifier;

    fn head(item: &str) -> MapTickRequest {
        MapTickRequest { head: ItemSlot::new(Identifier::new("test", item).unwrap(), 1) }
    }

    #[test]
    fn the_first_handler_that_knows_the_head_answers() {
        MapTickCallback::EVENT.register(|request| {
            if request.head.item.to_string() == "test:helmet" {
                Some(7)
            } else {
                None
            }
        });
        MapTickCallback::EVENT.register(|request| {
            if request.head.item.to_string() == "test:other" {
                Some(9)
            } else {
                None
            }
        });
        assert_eq!(map_tick(&head("helmet")), Some(7));
        assert_eq!(map_tick(&head("other")), Some(9));
    }

    #[test]
    fn no_handler_means_no_map_to_tick() {
        assert_eq!(map_tick(&head("plain")), None);
    }

    #[test]
    fn a_handler_listens() {
        MapTickCallback::EVENT.register(|_| None);
        assert!(map_tick_heard());
    }
}
