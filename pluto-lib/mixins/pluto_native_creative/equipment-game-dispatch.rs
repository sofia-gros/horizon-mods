// Capture only the current authoritative Game actor. No persistent identity,
// global map, pointer key, serialization, or inventory write is involved.
use super::Game;
use horizon_client::pluto_equipment::Snapshot;

/// Called immediately before the matching native player borrow. The owned
/// snapshot is a local value, so actor/world swaps and nested calls cannot
/// retrieve it from an ambient registry.
pub(super) fn capture(game: &Game<'_>) -> Option<Snapshot> {
    if game.remote.is_some() {
        return None;
    }
    let player = game.player.as_ref()?;
    if !horizon_client::health::pluto_protection::equipment_allowed(player) {
        return None;
    }
    Some(Snapshot::capture(
        player.entity_id,
        game.dim.id(),
        &game.armor,
        // held() follows the acting hand, which may be offhand.
        // Equipment MainHand is always the selected inventory slot.
        game.stack_in(game.selected),
        game.offhand.as_ref(),
    ))
}
