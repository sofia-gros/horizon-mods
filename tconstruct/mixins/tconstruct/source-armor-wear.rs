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

/// Refuse malformed durability, retained broken state, opaque wire data and
/// malformed stored shield amounts. The armor shield plan validates capacity.
pub(super) fn valid_record(stack: &ItemStack) -> bool {
    use horizon_world::nbt::Tag;
    if stack.count != 1 || stack.wire_patch().is_some() || !stack.enchantments().is_empty() { return false; }
    let Some(data) = stack.data() else { return false; };
    if data.other("minecraft:unbreakable").is_some()
        || data.other("minecraft:enchantments").is_some() { return false; }
    let Some(custom) = data.other("minecraft:custom_data") else { return false; };
    let maximum = match custom.get("tic_max_damage") {
        Some(Tag::Int(n)) if *n > 0 => *n as u32,
        _ => return false,
    };
    if stack.damage >= maximum { return false; }
    for key in ["tic_broken", "Unbreakable"] {
        if !matches!(custom.get(key), None | Some(Tag::Byte(0))) { return false; }
    }
    match custom.get("tic_overslime") {
        None => true, Some(Tag::Int(n)) => *n >= 0, _ => false,
    }
}

/// Do not guess between nested/repeated native wear callbacks. Both the
/// equipped stack and its recorded loss must have exactly one match.
pub(super) fn unambiguous_losses(slots: &[Option<ItemStack>; 4], losses: &[(ItemStack, u32)]) -> Vec<(usize, u32)> {
    slots.iter().enumerate().filter_map(|(slot, stack)| {
        let stack = stack.as_ref()?;
        if slots.iter().filter(|candidate| candidate.as_ref() == Some(stack)).count() != 1 { return None; }
        let mut matching = losses.iter().filter(|(original, _)| original == stack);
        let (_, loss) = matching.next()?;
        matching.next().is_none().then_some((slot, *loss))
    }).collect()
}

/// Pair the calculation and its final native return before any slot mutation.
/// A missing/duplicate shield record must not become raw durability writeback.
pub(super) fn paired_losses(slots: &[Option<ItemStack>; 4], losses: &[(ItemStack, u32)], shields: &[(ItemStack, u32)]) -> Vec<(usize, u32, u32)> {
    let shields = unambiguous_losses(slots, shields);
    unambiguous_losses(slots, losses).into_iter().filter_map(|(slot, loss)| {
        let (_, spent) = shields.iter().find(|(index, _)| *index == slot)?;
        Some((slot, loss, *spent))
    }).collect()
}

#[cfg(test)] mod retained_tests {
    use super::*;
    use horizon_world::nbt::Tag;
    fn armor() -> ItemStack {
        let mut item = ItemStack::new("tconstruct:plate_helmet", 1);
        item.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
            ("tic_max_damage", Tag::Int(5)), ("tic_broken", Tag::Byte(0)),
        ])));
        item
    }
    #[test] fn duplicate_callbacks_and_replaced_slots_are_not_committed() {
        let item = armor();
        let slots = [Some(item.clone()), None, None, None];
        assert_eq!(unambiguous_losses(&slots, &[(item.clone(), 2)]), vec![(0, 2)]);
        assert!(unambiguous_losses(&slots, &[(item.clone(), 2), (item.clone(), 3)]).is_empty());
        let mut changed = item.clone(); changed.damage = 1;
        assert!(unambiguous_losses(&[Some(changed), None, None, None], &[(item, 2)]).is_empty());
    }
    #[test] fn exhaustion_retains_item_and_marks_it_broken() {
        let mut item = armor();
        assert!(valid_record(&item));
        super::super::tconstruct_tx::fixed_damage(&mut item, 9);
        assert_eq!(item.count, 1);
        assert_eq!(item.damage, 5);
        assert!(!valid_record(&item));
        assert!(matches!(item.data().unwrap().other("minecraft:custom_data").unwrap().get("tic_broken"), Some(Tag::Byte(1))));
    }
    #[test] fn malformed_shields_and_broken_flags_are_rejected() {
        let mut item = armor();
        item.count = 2; assert!(!valid_record(&item));
        item.count = 1;
        item.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
            ("tic_max_damage", Tag::Int(5)), ("tic_overslime", Tag::Int(-1)),
        ])));
        assert!(!valid_record(&item));
        item.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
            ("tic_max_damage", Tag::Int(5)), ("tic_broken", Tag::String("false".into())),
        ])));
        assert!(!valid_record(&item));
    }
}

/// A retained armor break is an equipment animation, never an item deletion.
/// The caller already owns a unique exact-stack commit. Repeated, shield-only
/// and already-broken records must not produce another event.
pub(super) fn break_event(slot: usize, before: &ItemStack, after: &ItemStack) -> Option<i8> {
    if !valid_record(before) || before.item != after.item || after.count != 1
        || after.damage <= before.damage
        || !super::tconstruct_source_ranged_wear::is_broken(after)
        || super::tconstruct_fire_duration::armor_slot(&before.item) != Some(slot) {
        return None;
    }
    [49, 50, 51, 52].get(slot).copied()
}

#[cfg(test)] mod break_tests {
    use super::*;
    use horizon_world::nbt::Tag;
    fn armor(item: &str) -> ItemStack {
        let mut stack = ItemStack::new(item, 1);
        stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
            ("tic_max_damage", Tag::Int(5)), ("tic_overslime", Tag::Int(3)),
        ])));
        stack
    }
    #[test] fn each_retained_armor_slot_gets_its_own_break_event_once() {
        for (slot, item) in ["tconstruct:plate_helmet", "tconstruct:plate_chestplate",
            "tconstruct:plate_leggings", "tconstruct:plate_boots"].iter().enumerate() {
            let before = armor(item);
            let mut after = before.clone();
            super::super::tconstruct_tx::fixed_damage(&mut after, 7);
            assert_eq!(after.count, 1);
            assert_eq!(break_event(slot, &before, &after), Some(49 + slot as i8));
            assert_eq!(break_event(slot, &after, &after), None);
            assert_eq!(break_event((slot + 1) % 4, &before, &after), None);
        }
    }
    #[test] fn partial_shield_only_replaced_and_missing_gear_are_silent() {
        let before = armor("tconstruct:plate_helmet");
        let mut after = before.clone();
        super::super::tconstruct_source_overslime::spend(&mut after, 3);
        assert_eq!(break_event(0, &before, &after), None);
        super::super::tconstruct_tx::fixed_damage(&mut after, 4);
        assert_eq!(break_event(0, &before, &after), None);
        super::super::tconstruct_tx::fixed_damage(&mut after, 1);
        after.count = 0;
        assert_eq!(break_event(0, &before, &after), None);
        after.count = 1;
        after.item = "minecraft:iron_helmet".into();
        assert_eq!(break_event(0, &before, &after), None);
        assert_eq!(break_event(4, &before, &before), None);
    }
}
