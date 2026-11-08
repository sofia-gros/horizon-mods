use horizon_items::items::ItemStack;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Remainder { Creative, ReplaceHand, Inventory }

/// How a completed use is paid for. Only if the stack that started the use is still intact in the hand at finish.
pub(super) fn plan(before: &ItemStack, held: &Option<ItemStack>, creative: bool) -> Option<Remainder> {
    if before.count == 0 || before.damage != 0 || before.extra.is_some() || before.wire_patch().is_some() { return None; }
    let held = held.as_ref()?;
    if held.count != before.count || held.damage != before.damage || !held.same_item_same_components(before) { return None; }
    if creative { Some(Remainder::Creative) } else if before.count == 1 { Some(Remainder::ReplaceHand) } else { Some(Remainder::Inventory) }
}
