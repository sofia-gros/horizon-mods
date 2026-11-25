//! Read-only actor/equipment facts shared by synchronous event capabilities.
//! Semantic item views are never native snapshot/commit identity. Nothing here
//! owns a world, mutates a slot, calculates wear or validates a write.
use crate::advancements::predicates::ItemSnapshot;

#[derive(Clone, Copy, Debug)]
pub struct Actor<'a> {
    pub entity: i32,
    pub dimension: &'a str,
    pub creative: bool,
    pub spectator: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EquipmentSlot {
    Head,
    Chest,
    Legs,
    Feet,
    MainHand,
    OffHand,
}

/// Unavailable is distinct from a known empty slot. Item components may also
/// be unavailable independently; ItemSnapshot is semantic read data, never a
/// retained-wire equality token or authorization to replace the live stack.
#[derive(Clone, Copy, Debug)]
pub enum Slot<'a> {
    Unavailable,
    Empty,
    Item(ItemSnapshot<'a>),
}
#[derive(Clone, Copy, Debug)]
pub struct ActiveUse<'a> {
    /// None means an active slot outside the six equipment slots, not no use.
    pub slot: Option<EquipmentSlot>,
    pub item: Slot<'a>,
}

/// Borrow only during the synchronous callback. Each event specifies its capture
/// phase; these facts are not a live inventory or transaction snapshot. There is
/// intentionally no mutation, wear or commit operation.
pub trait Equipment {
    fn slot(&self, slot: EquipmentSlot) -> Slot<'_>;
    /// None means no active use. Some with an empty/unavailable item is retained.
    fn active_use(&self) -> Option<ActiveUse<'_>>;
}

/// A narrower captured-slot capability for events that do not know active use.
/// It deliberately has no active-use query, so unknown is never called inactive.
pub trait EquipmentSlots {
    fn slot(&self, slot: EquipmentSlot) -> Slot<'_>;
}
impl<T: Equipment + ?Sized> EquipmentSlots for T {
    fn slot(&self, slot: EquipmentSlot) -> Slot<'_> {
        Equipment::slot(self, slot)
    }
}

#[cfg(test)]
mod slot_tests {
    use super::*;
    struct Known;
    impl Equipment for Known {
        fn slot(&self, _: EquipmentSlot) -> Slot<'_> {
            Slot::Empty
        }
        fn active_use(&self) -> Option<ActiveUse<'_>> {
            panic!("slot projection must not query use")
        }
    }
    #[test]
    fn pluto_equipment_slots_projection_does_not_query_active_use() {
        let slots: &dyn EquipmentSlots = &Known;
        assert!(matches!(slots.slot(EquipmentSlot::OffHand), Slot::Empty));
    }
}
