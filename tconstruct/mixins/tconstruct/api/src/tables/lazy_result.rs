// Ported from Tinkers' Construct LazyResultContainer and LazyResultSlot.
// Copyright (c) 2022 SlimeKnights. MIT; see ../../../LICENSE.
// Source: f4e77d43f39ba1047bf93ae816b7e1beaab514c2, tables/block/entity/inventory/
// LazyResultContainer.java and tables/menu/slot/LazyResultSlot.java.

use pluto_lib::ItemSlot;

/// Block-entity-owned preview cache, separate from each viewer's amount tracker.
/// `None` means unevaluated; `Some(None)` is a cached empty recipe result.
#[derive(Default)]
pub(crate) struct LazyResultContainer {
    result: Option<Option<ItemSlot>>,
}

impl LazyResultContainer {
    pub(crate) fn result(&mut self, calculate: impl FnOnce() -> Option<ItemSlot>) -> Option<&ItemSlot> {
        self.result.get_or_insert_with(|| calculate().filter(|stack| stack.count > 0)).as_ref()
    }

    /// Java removeItem ignores the requested size and never consumes the preview.
    pub(crate) fn remove(&mut self, calculate: impl FnOnce() -> Option<ItemSlot>) -> Option<ItemSlot> {
        self.result(calculate).cloned()
    }

    pub(crate) fn clear(&mut self) {
        self.result = None;
    }

    /// Invalidate only after onCraft returns, including when it does no work.
    pub(crate) fn craft_result(&mut self, stack: &mut ItemSlot, amount: u32,
        on_craft: impl FnOnce(&mut ItemSlot, u32)) {
        on_craft(stack, amount);
        self.clear();
    }
}

/// Menu-owned state: viewers share a preview but never share amountCrafted.
#[derive(Default)]
pub(crate) struct LazyResultSlot {
    amount_crafted: u32,
}

impl LazyResultSlot {
    pub(crate) fn remove(&mut self, inventory: &mut LazyResultContainer, amount: u32,
        calculate: impl FnOnce() -> Option<ItemSlot>) -> Option<ItemSlot> {
        let result = inventory.remove(calculate);
        if let Some(stack) = &result {
            self.amount_crafted = self.amount_crafted.wrapping_add(amount.min(stack.count));
        }
        result
    }

    /// Slot.checkTakeAchievements is empty here; crafting occurs in onTake.
    pub(crate) fn on_quick_craft(&mut self, amount: u32) {
        self.amount_crafted = self.amount_crafted.wrapping_add(amount);
    }

    pub(crate) fn on_swap_craft(&mut self, amount: u32) {
        self.amount_crafted = self.amount_crafted.wrapping_add(amount);
    }

    pub(crate) fn on_take(&mut self, inventory: &mut LazyResultContainer, stack: &mut ItemSlot,
        on_craft: impl FnOnce(&mut ItemSlot, u32)) {
        inventory.craft_result(stack, self.amount_crafted, on_craft);
        self.amount_crafted = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::{Identifier, Nbt};

    fn stack(count: u32) -> ItemSlot {
        let mut stack = ItemSlot::new(Identifier::parse("tconstruct:test_part").unwrap(), count);
        stack.custom_data.insert("material".into(), Nbt::String("test:wood".into()));
        stack
    }

    #[test]
    fn caches_empty_and_present_results_until_invalidated() {
        let mut inventory = LazyResultContainer::default();
        assert!(inventory.result(|| None).is_none());
        assert!(inventory.result(|| panic!("empty result must stay cached")).is_none());
        inventory.clear();
        assert_eq!(inventory.result(|| Some(stack(4))).unwrap().count, 4);
        let mut removed = inventory.remove(|| panic!("must use cached preview")).unwrap();
        removed.count = 1;
        removed.custom_data.clear();
        let preview = inventory.result(|| panic!("removal must not invalidate")).unwrap();
        assert_eq!(preview.count, 4);
        assert_eq!(preview.custom_data.get("material"), Some(&Nbt::String("test:wood".into())));
    }

    #[test]
    fn removal_returns_full_preview_but_tracks_requested_amount_separately() {
        let mut inventory = LazyResultContainer::default();
        let mut slot = LazyResultSlot::default();
        let mut taken = slot.remove(&mut inventory, 1, || Some(stack(4))).unwrap();
        assert_eq!(taken.count, 4);
        slot.on_take(&mut inventory, &mut taken, |output, amount| {
            assert_eq!(amount, 1);
            output.damage = 7;
        });
        assert_eq!(taken.damage, 7);
        assert_eq!(inventory.result(|| Some(stack(2))).unwrap().count, 2);
        slot.on_take(&mut inventory, &mut taken, |_, amount| assert_eq!(amount, 0));
        assert!(inventory.result(|| None).is_none());
    }

    #[test]
    fn craft_amounts_accumulate_per_viewer_and_empty_removal_adds_nothing() {
        let mut inventory = LazyResultContainer::default();
        let mut first = LazyResultSlot::default();
        let mut second = LazyResultSlot::default();
        assert!(first.remove(&mut inventory, 99, || None).is_none());
        inventory.clear();
        let mut taken = first.remove(&mut inventory, 99, || Some(stack(4))).unwrap();
        first.on_quick_craft(2);
        first.on_swap_craft(3);
        second.on_take(&mut inventory, &mut taken, |_, amount| assert_eq!(amount, 0));
        first.on_take(&mut inventory, &mut taken, |_, amount| assert_eq!(amount, 9));
        first.on_take(&mut inventory, &mut taken, |_, amount| assert_eq!(amount, 0));
    }
}
