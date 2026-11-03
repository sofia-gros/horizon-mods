// Conservative access to the host's static crafting remainder for an item.
use horizon_items::items::ItemStack;

/// For an unpatched stack, report whether its registry item has no remainder.
/// Stack patches and custom container behaviour need a caller-specific check.
pub fn plain_has_no_remainder(stack: &ItemStack, remainder: Option<&str>) -> Option<bool> {
    if stack.count == 0 || stack.wire_patch().is_some() || stack.extra.is_some() { return None; }
    Some(remainder.is_none())
}
