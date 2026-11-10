// PlutoLib's live configured-feature boundary. Generation workers are a separate path.
// Keep native level/RNG semantics here, never in a consumer mod.
use super::{Behaviours, LevelAccess, Ticks};
use horizon_core::position::BlockPos;
use serde_json::Value;

/// Read the active live catalog, not an embedded configuration or a global registry snapshot.
pub fn configured_json(behaviours: &Behaviours, name: &str) -> Option<Value> {
    behaviours.live_features()?.configured_json(name)
}

/// Preserve ConfiguredFeature.place's native random stream, updates and failure semantics.
/// This concrete entry lets PlutoLib own a live adapter without injecting into generic methods.
pub fn place_native(
    behaviours: &Behaviours,
    level: &mut dyn LevelAccess,
    ticks: &mut Ticks,
    name: &str,
    position: BlockPos,
) -> Option<bool> {
    behaviours.place_feature(level, ticks, name, position)
}
