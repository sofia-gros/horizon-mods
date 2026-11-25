// Read-only child of the actual native item_data module. This module has no
// client/API dependency and neither clones nor changes retained native state.
use super::Wire;
use crate::items::ItemStack;

/// Borrow the exact decoded baseline and patch stored in the native Wire.
/// Their lifetimes remain tied to `wire`; no write or transport authority is
/// implied. The baseline is not necessarily the item's currently usable state.
pub fn retained(wire: &Wire) -> Option<(&ItemStack, &[u8])> {
    wire.0
        .as_deref()
        .map(|(decoded, patch)| (decoded, patch.as_ref()))
}
