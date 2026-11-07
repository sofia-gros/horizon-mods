// Lossless, read-only equipment input for a single synchronous dispatch.
// This is not an inventory, persistence key, transaction, or component codec.
use crate::items::ItemStack;
use horizon_world::player_inventory::EquipmentSlot;

#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    entity: i32,
    dimension: String,
    // Native Game armor order: head, chest, legs, feet.
    armor: [Option<ItemStack>; 4],
    hands: [Option<ItemStack>; 2],
}
impl Snapshot {
    /// The caller must capture these inputs from the same active actor/world,
    /// immediately before dispatch. Entity numbers are not persistent identity.
    /// No stack/component normalization, serialization or filtering is performed.
    pub fn capture(
        entity: i32,
        dimension: &str,
        armor: &[Option<ItemStack>; 4],
        main_hand: Option<&ItemStack>,
        off_hand: Option<&ItemStack>,
    ) -> Self {
        Self {
            entity,
            dimension: dimension.to_owned(),
            armor: armor.clone(),
            hands: [main_hand.cloned(), off_hand.cloned()],
        }
    }
    pub fn entity(&self) -> i32 {
        self.entity
    }
    pub fn dimension(&self) -> &str {
        &self.dimension
    }
    /// Outer None means an unsupported slot (body/saddle), not an empty slot.
    /// Some(None) means a captured empty armor/hand slot.
    pub fn get(&self, slot: EquipmentSlot) -> Option<Option<&ItemStack>> {
        let stack = match slot {
            EquipmentSlot::Head => &self.armor[0],
            EquipmentSlot::Chest => &self.armor[1],
            EquipmentSlot::Legs => &self.armor[2],
            EquipmentSlot::Feet => &self.armor[3],
            EquipmentSlot::MainHand => &self.hands[0],
            EquipmentSlot::OffHand => &self.hands[1],
            EquipmentSlot::Body | EquipmentSlot::Saddle => return None,
        };
        Some(stack.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn stack(id: &str, count: u32, damage: u32) -> ItemStack {
        ItemStack {
            item: id.into(),
            count,
            damage,
            extra: None,
        }
    }
    #[test]
    fn pluto_equipment_preserves_native_slots_and_empty_vs_unknown() {
        let armor = [
            Some(stack("example:head", 1, 2)),
            None,
            Some(stack("example:legs", 1, 3)),
            Some(stack("example:feet", 1, 4)),
        ];
        let hand = stack("example:tool", 7, 5);
        let view = Snapshot::capture(19, "example:dimension", &armor, Some(&hand), None);
        assert_eq!(view.entity(), 19);
        assert_eq!(view.dimension(), "example:dimension");
        assert_eq!(view.get(EquipmentSlot::Head), Some(armor[0].as_ref()));
        assert_eq!(view.get(EquipmentSlot::Legs), Some(armor[2].as_ref()));
        assert_eq!(view.get(EquipmentSlot::Feet), Some(armor[3].as_ref()));
        assert_eq!(view.get(EquipmentSlot::MainHand), Some(Some(&hand)));
        assert_eq!(view.get(EquipmentSlot::Chest), Some(None));
        assert_eq!(view.get(EquipmentSlot::OffHand), Some(None));
        assert_eq!(view.get(EquipmentSlot::Body), None);
        assert_eq!(view.get(EquipmentSlot::Saddle), None);
    }
    #[test]
    fn pluto_equipment_clone_retains_components_without_live_aliasing() {
        use horizon_world::nbt::{Compound, Tag};
        let mut original = stack("example:tool", 3, 11);
        let mut data = crate::item_data::StackData::default();
        let mut custom = Compound::new();
        custom.insert("unknown:payload".into(), Tag::Long(i64::MIN));
        data.set_other("minecraft:custom_data", Some(Tag::Compound(custom)));
        original.extra = Some(Box::new(data));
        let before = original.clone();
        let view = Snapshot::capture(
            5,
            "minecraft:overworld",
            &[None, None, None, None],
            Some(&original),
            None,
        );
        original.count = 0;
        original.damage = 99;
        original
            .extra
            .as_mut()
            .unwrap()
            .set_other("minecraft:custom_data", None);
        assert_eq!(view.get(EquipmentSlot::MainHand), Some(Some(&before)));
        assert_ne!(view.get(EquipmentSlot::MainHand), Some(Some(&original)));
    }
}
