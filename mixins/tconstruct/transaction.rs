use horizon_items::items::ItemStack;

/// `IMaterialValue.getItemsUsed` and `getRemainder` from the pinned original.
/// Recipe loaders require strictly positive cost, needed and value.
fn material_cost(cost: i32, needed: i32, value: i32) -> Option<(i32, i32)> {
    if cost <= 0 || needed <= 0 || value <= 0 { return None; }
    // Java's int multiplication/subtraction wrap, and division truncates to zero.
    let total = cost.wrapping_mul(needed);
    let used = total / value + i32::from(total % value != 0);
    let mut remainder = value.wrapping_sub(total) % value;
    if remainder < 0 { remainder += value; }
    Some((used, remainder))
}

/// Apply a single result pickup to the authoritative slots and cursor.
/// Inputs are checked before any mutation; tool data is carried by cloning it.
pub(super) fn take(stacks: &mut [Option<ItemStack>], carried: &mut Option<ItemStack>, station: bool, action: i16) -> bool {
    if stacks.len() < 27 { return false; }
    let is = |slot: usize, item: &str, count: u32| stacks.get(slot).and_then(Option::as_ref).is_some_and(|s| &*s.item == item && s.count >= count);
    let (output, consume): (ItemStack, Vec<(usize, u32)>) = if !station {
        let (material, cost, result) = match action {
            24 => ("minecraft:flint", 2, "tconstruct:flint_pick_head"),
            25 => ("minecraft:oak_planks", 1, "tconstruct:wood_handle"),
            26 => ("minecraft:oak_planks", 1, "tconstruct:wood_binding"),
            _ => return false,
        };
        let Some((amount, remainder)) = material_cost(cost, 1, 1) else { return false; };
        // The current two inputs have unit value and no remainder. Other
        // material recipes and their remainder items are not yet supported.
        let Ok(amount) = u32::try_from(amount) else { return false; };
        if amount == 0 || remainder != 0 || !is(0, "tconstruct:pattern", 1) || !is(1, material, amount) { return false; }
        (ItemStack::new(result, 1), vec![(1, amount)])
    } else if action == 24 {
        if !is(0, "tconstruct:flint_pick_head", 1) || !is(1, "tconstruct:wood_handle", 1) || !is(2, "tconstruct:wood_binding", 1) { return false; }
        (ItemStack::new("tconstruct:flint_pickaxe", 1), vec![(0, 1), (1, 1), (2, 1)])
    } else if action == 25 {
        if !is(0, "tconstruct:flint_pickaxe", 1) { return false; }
        let mut tool = stacks[0].as_ref().unwrap().clone();
        if tool.count != 1 || tool.damage == 0 { return false; }
        if (1..=2).any(|i| stacks[i].as_ref().is_some_and(|s| &*s.item != "minecraft:flint" || s.count == 0)) { return false; }
        let mut needed = tool.damage;
        let mut consume = vec![(0, 1)];
        for slot in 1..=2 {
            if let Some(stack) = &stacks[slot] {
                let count = stack.count.min(needed.div_ceil(56));
                if count > 0 { consume.push((slot, count)); needed = needed.saturating_sub(count.saturating_mul(56)); }
            }
        }
        if consume.len() == 1 { return false; }
        tool.damage = needed;
        (tool, consume)
    } else { return false; };
    if let Some(existing) = carried.as_ref() {
        let max = if &*output.item == "tconstruct:flint_pickaxe" { 1 } else { 64 };
        let mut compare = existing.clone();
        compare.count = output.count;
        if compare != output || existing.count >= max { return false; }
    }
    for (slot, count) in consume {
        let stack = stacks[slot].as_mut().unwrap();
        stack.count -= count;
        if stack.count == 0 { stacks[slot] = None; }
    }
    match carried.as_mut() {
        Some(stack) => stack.count += output.count,
        None => *carried = Some(output),
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_material_cost_rounds_up_and_tracks_remainder() {
        assert_eq!(material_cost(2, 1, 1), Some((2, 0)));
        assert_eq!(material_cost(1, 2, 3), Some((1, 1)));
        assert_eq!(material_cost(2, 1, 3), Some((1, 1)));
        assert_eq!(material_cost(2, 3, 2), Some((3, 0)));
        assert_eq!(material_cost(i32::MAX, 2, 3), Some((1, 2)));
        assert_eq!(material_cost(1, 1, 0), None);
    }
    #[test]
    fn part_take_is_atomic_and_pattern_is_reusable() {
        let mut slots = vec![None; 27];
        slots[0] = Some(ItemStack::new("tconstruct:pattern", 1));
        slots[1] = Some(ItemStack::new("minecraft:flint", 2));
        let mut cursor = Some(ItemStack::new("minecraft:stick", 1));
        let before = slots.clone();
        assert!(!take(&mut slots, &mut cursor, false, 24));
        assert_eq!(slots, before);
        cursor = None;
        assert!(take(&mut slots, &mut cursor, false, 24));
        assert_eq!(cursor.as_ref().unwrap().item.as_ref(), "tconstruct:flint_pick_head");
        assert!(slots[1].is_none());
        assert_eq!(slots[0].as_ref().unwrap().count, 1);
        cursor = None;
        assert!(!take(&mut slots, &mut cursor, false, 24));
        assert!(cursor.is_none());
    }
    #[test]
    fn preview_does_not_consume_and_rechecks_material() {
        let mut slots = vec![None; 27];
        slots[0] = Some(ItemStack::new("tconstruct:pattern", 1));
        slots[1] = Some(ItemStack::new("minecraft:flint", 2));
        let before = slots.clone();
        assert_eq!(part_preview(&slots, 0).unwrap().item.as_ref(), "tconstruct:flint_pick_head");
        assert_eq!(slots, before);
        assert!(part_preview(&slots, 1).is_none());
        slots[1] = Some(ItemStack::new("minecraft:flint", 1));
        assert!(part_preview(&slots, 0).is_none());
    }
    #[test]
    fn repair_preserves_name_and_unknown_components() {
        let mut slots = vec![None; 27];
        let mut tool = ItemStack::new("tconstruct:flint_pickaxe", 1);
        tool.damage = 85;
        tool.data_mut().custom_name = Some("Named tool".into());
        tool.data_mut().other.push(("example:custom".into(), horizon_world::nbt::Tag::Int(17)));
        slots[0] = Some(tool.clone());
        slots[1] = Some(ItemStack::new("minecraft:flint", 1));
        let mut cursor = None;
        assert!(take(&mut slots, &mut cursor, true, 25));
        let repaired = cursor.as_ref().unwrap();
        assert_eq!(repaired.damage, 29);
        assert_eq!(repaired.extra, tool.extra);
        assert!(slots[0].is_none() && slots[1].is_none());
        assert!(!take(&mut slots, &mut None, true, 25));
    }
    #[test]
    fn repair_rechecks_split_inputs_and_never_consumes_excess() {
        let mut slots = vec![None; 27];
        let mut tool = ItemStack::new("tconstruct:flint_pickaxe", 1);
        tool.damage = 85;
        slots[0] = Some(tool);
        slots[1] = Some(ItemStack::new("minecraft:flint", 1));
        slots[2] = Some(ItemStack::new("minecraft:flint", 3));
        let mut cursor = None;
        assert!(take(&mut slots, &mut cursor, true, 25));
        assert_eq!(cursor.unwrap().damage, 0);
        assert!(slots[1].is_none());
        assert_eq!(slots[2].as_ref().unwrap().count, 2);
    }

}

/// The selected recipe has no result unless the current inputs still match.
pub(super) fn part_preview(stacks: &[Option<ItemStack>], selected: i16) -> Option<ItemStack> {
    let mut copy = stacks.to_vec();
    let mut result = None;
    take(&mut copy, &mut result, false, selected + 24).then_some(result).flatten()
}
