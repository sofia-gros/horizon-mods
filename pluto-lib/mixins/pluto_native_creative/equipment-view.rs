// Shared native capture of read-only semantic equipment facts.
// This is not the full native stack snapshot used for transaction revalidation.
use super::Game;
use horizon_client::{pluto_component_view::ItemFacts, pluto_inventory::equipment as api};

enum Captured {
    Unavailable,
    Empty,
    Item(ItemFacts),
}
impl Captured {
    fn slot(slot: Option<&Option<horizon_client::items::ItemStack>>) -> Self {
        match slot {
            None => Self::Unavailable,
            Some(None) => Self::Empty,
            Some(Some(stack)) => Self::Item(ItemFacts::capture(stack)),
        }
    }
    fn view(&self) -> api::Slot<'_> {
        match self {
            Self::Unavailable => api::Slot::Unavailable,
            Self::Empty => api::Slot::Empty,
            Self::Item(item) => api::Slot::Item(item.view()),
        }
    }
}
pub(crate) struct Snapshot {
    slots: [Captured; 6],
    active: Option<(Option<api::EquipmentSlot>, Captured)>,
}
impl Snapshot {
    pub(crate) fn capture(game: &Game<'_>) -> Self {
        use api::EquipmentSlot;
        let slots = [
            Captured::slot(Some(&game.armor[0])),
            Captured::slot(Some(&game.armor[1])),
            Captured::slot(Some(&game.armor[2])),
            Captured::slot(Some(&game.armor[3])),
            // Game::held() is the acting hand, which may be the off hand.
            // The main-hand fact must use the selected inventory slot itself.
            Captured::slot(game.inventory.get(game.selected)),
            Captured::slot(Some(&game.offhand)),
        ];
        let active = game.using.map(|(slot, _)| {
            if slot == super::shield::OFFHAND_SLOT {
                (
                    Some(EquipmentSlot::OffHand),
                    Captured::slot(Some(&game.offhand)),
                )
            } else {
                let equipment_slot = (slot == game.selected).then_some(EquipmentSlot::MainHand);
                (equipment_slot, Captured::slot(game.inventory.get(slot)))
            }
        });
        Self { slots, active }
    }
}
impl api::Equipment for Snapshot {
    fn slot(&self, slot: api::EquipmentSlot) -> api::Slot<'_> {
        use api::EquipmentSlot::*;
        self.slots[match slot {
            Head => 0,
            Chest => 1,
            Legs => 2,
            Feet => 3,
            MainHand => 4,
            OffHand => 5,
        }]
        .view()
    }
    fn active_use(&self) -> Option<api::ActiveUse<'_>> {
        self.active.as_ref().map(|(slot, item)| api::ActiveUse {
            slot: *slot,
            item: item.view(),
        })
    }
}
