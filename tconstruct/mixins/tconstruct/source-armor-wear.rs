// Map the original vanilla durability calculations back to unchanged slots.
use horizon_client::items::ItemStack;

pub(super) fn matched_losses(slots: &[Option<ItemStack>; 4], losses: &[(ItemStack, u32)]) -> Vec<(usize, u32)> {
    let mut used = [false; 4];
    let mut result = Vec::new();
    for (original, loss) in losses {
        if let Some(index) = slots.iter().enumerate().position(|(index, slot)| !used[index] && slot.as_ref() == Some(original)) {
            used[index] = true;
            result.push((index, *loss));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maps_mixed_and_identical_slots_once() {
        let helmet = ItemStack::new("tconstruct:plate_helmet", 1);
        let boots = ItemStack::new("tconstruct:plate_boots", 1);
        let vanilla = ItemStack::new("minecraft:iron_chestplate", 1);
        let slots = [Some(helmet.clone()), Some(vanilla), Some(boots.clone()), Some(boots.clone())];
        let losses = vec![(helmet, 1), (boots.clone(), 2), (boots, 0)];
        assert_eq!(matched_losses(&slots, &losses), vec![(0, 1), (2, 2), (3, 0)]);
    }
    #[test]
    fn ignores_replaced_item_and_excess_loss() {
        let original = ItemStack::new("tconstruct:plate_helmet", 1);
        let slots = [Some(ItemStack::new("minecraft:iron_helmet", 1)), None, None, None];
        assert!(matched_losses(&slots, &[(original, 1)]).is_empty());
    }
}
