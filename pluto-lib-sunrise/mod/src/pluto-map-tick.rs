// Before the maps tick runs: the stored map in a worn piece ticks as if it were held (Tinkers'
// MinimapModule, at its inventory tick: the stored map goes into the offhand, vanilla ticks
// it, and it goes back). The maps tick reads the offhand directly, so the swap brackets the
// tick itself: the inventory-tick reach runs too early for a LAN guest (its maps tick runs
// before its inventory ticks) and would leave the map in the offhand for a full tick. The
// return half cannot hang on the maps tick itself (its body refuses a RETURN seam), so it
// rides the compass tick — the first call after the maps tick on both tick paths, before
// anything else runs (the compass pass itself reads the offhand and must find it restored):
// it restores the offhand first, always, then sends a guest the update packet when
// anything changed; the local player needs no packet (its client reads the map data live).
use super::*;

use sunrise_client::mod_api::Host;

/// The head slot in the carried inventory (`EquipmentSlot::Head`).
const HEAD_SLOT: i32 = 103;

/// Swaps the worn head's stored map into the offhand for the maps tick: nothing happens when
/// no map-tick handler listens, when the head carries nothing, or when no handler answers the
/// head. A previous tick that never returned left a stale swap; it is restored first, so the
/// offhand is never lost to a missed return.
pub(crate) fn tick_head(game: &mut Game<'_>) {
    if let Some(stale) = crate::pluto_map_tick_access::take_swap() {
        game.offhand = stale.old_offhand;
    }
    if !crate::pluto_map_tick_access::enabled() {
        return;
    }
    let Some(head) = game.native_carried_stack(HEAD_SLOT).cloned() else {
        return;
    };
    let Some(id) = crate::pluto_map_tick_access::on_map_tick_head(game, &head) else {
        return;
    };
    let mut map = ItemStack::new("minecraft:filled_map", 1);
    map.data_mut().map_id = Some(id);
    let old_offhand = std::mem::replace(&mut game.offhand, Some(map));
    crate::pluto_map_tick_access::stash(crate::pluto_map_tick_access::Swap {
        old_offhand,
        map_id: id,
        who: game.lan_acting.unwrap_or(HOST),
    });
}

/// As the compass tick opens (the first call after the maps tick on both tick paths):
/// restores the offhand first, always, then sends a LAN guest the update packet when the
/// tick changed anything. A packet is only ever pushed when `update_packet` returns one,
/// so a guest's decorations are never erased by a packet that carries none. The `kind`
/// closure reads the decoration type's registry id from `DECORATION_TYPES` by position:
/// the table is in registry order for this pin, verified against
/// `base-26.3/registries.json` (`minecraft:map_decoration_type`, protocol ids 0-39).
pub(crate) fn tick_return(game: &mut Game<'_>) {
    let Some(swap) = crate::pluto_map_tick_access::take_swap() else {
        return;
    };
    game.offhand = swap.old_offhand;
    if game.lan_acting.is_none() {
        return;
    }
    let packet = game.maps.data.get_mut(&swap.map_id).and_then(|map| {
        map.update_packet(swap.map_id, swap.who, |name| {
            DECORATION_TYPES.iter().position(|(kind, _)| *kind == name).map(|index| index as i32)
        })
    });
    if let Some(packet) = packet {
        game.guest_packets.push(sunrise_protocol::packets::play::Clientbound::MapItemData(packet));
    }
}
