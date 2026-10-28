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

/// `PartRecipe.assemble` changes the part count when a material's value
/// exceeds the part cost and the material recipe has no leftover item.
fn material_output_count(output_count: i32, value: i32, cost: i32, has_leftover: bool) -> Option<i32> {
    if output_count <= 0 || value <= 0 || cost <= 0 { return None; }
    Some(if !has_leftover && value > cost { output_count.wrapping_mul(value) / cost } else { output_count })
}

/// `TinkerStationRepairRecipe.repairFromSlot`: Java performs the division and
/// multiplication as floats, rounds the item count up, then truncates the
/// repaired durability. The caller supplies the material/modifier-derived rate.
fn repair_from_slot(needed: i32, stack: i32, durability_per_item: f32) -> (i32, i32) {
    if needed <= 0 || stack <= 0 || !(durability_per_item > 0.0) { return (0, 0); }
    let applied = stack.min(((needed as f32) / durability_per_item).ceil() as i32);
    (applied, (applied as f32 * durability_per_item) as i32)
}

/// The original `IMaterialItem` stores the chosen material under `Material`.
/// Horizon 26.3 keeps that legacy item tag inside `minecraft:custom_data`.
fn part_with_material(id: &str, material: &str) -> ItemStack {
    let mut part = ItemStack::new(id, 1);
    part.data_mut().set_other("minecraft:custom_data", Some(horizon_world::nbt::Tag::compound([
        ("Material", horizon_world::nbt::Tag::String(material.into())),
    ])));
    let (model, title) = match id {
        "tconstruct:pick_head" => ("tconstruct:flint_pick_head", "Flint Pick Head"),
        "tconstruct:tool_handle" => ("tconstruct:wood_handle", "Wood Tool Handle"),
        "tconstruct:tool_binding" => ("tconstruct:wood_binding", "Wood Tool Binding"),
        _ => return part,
    };
    part.data_mut().set_other("minecraft:item_model", Some(horizon_world::nbt::Tag::String(model.into())));
    part.data_mut().set_other("minecraft:item_name", Some(horizon_world::nbt::Tag::String(title.into())));
    part
}

/// Legacy prototype IDs encode a single material. If a Material tag is
/// present, reject a conflicting value rather than silently assembling it.
fn part_matches_material(stack: &ItemStack, expected: &str) -> bool {
    use horizon_world::nbt::Tag;
    match stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|v| v.get("Material")) {
        None => matches!(&*stack.item, "tconstruct:flint_pick_head" | "tconstruct:wood_handle" | "tconstruct:wood_binding"),
        Some(Tag::String(value)) => value.to_string_lossy() == expected,
        Some(_) => false,
    }
}

fn fixed_pickaxe_with_materials() -> ItemStack {
    use horizon_world::nbt::Tag;
    let mut tool = ItemStack::new("tconstruct:pickaxe", 1);
    let materials = ["tconstruct:flint", "tconstruct:wood", "tconstruct:wood"]
        .into_iter().map(|id| Tag::String(id.into())).collect();
    tool.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_materials", Tag::List(materials))])));
    tool.data_mut().set_other("minecraft:item_model", Some(Tag::String("tconstruct:flint_pickaxe".into())));
    tool.data_mut().set_other("minecraft:item_name", Some(Tag::String("Flint Pickaxe".into())));
    tool
}

/// Apply a single result pickup to the authoritative slots and cursor.
/// Inputs are checked before any mutation; tool data is carried by cloning it.
pub(super) fn take(stacks: &mut [Option<ItemStack>], carried: &mut Option<ItemStack>, station: bool, action: i16) -> bool {
    if stacks.len() < 27 { return false; }
    let is = |slot: usize, item: &str, count: u32| stacks.get(slot).and_then(Option::as_ref).is_some_and(|s| &*s.item == item && s.count >= count);
    let (output, consume): (ItemStack, Vec<(usize, u32)>) = if !station {
        let (material, cost, result) = match action {
            24 => ("minecraft:flint", 2, "tconstruct:pick_head"),
            25 => ("minecraft:oak_planks", 1, "tconstruct:tool_handle"),
            26 => ("minecraft:oak_planks", 1, "tconstruct:tool_binding"),
            _ => return false,
        };
        let Some((amount, remainder)) = material_cost(cost, 1, 1) else { return false; };
        let Some(count) = material_output_count(1, 1, cost, false) else { return false; };
        // Only unit-value materials and one-item results are currently wired in.
        if count != 1 { return false; }
        // The current two inputs have unit value and no remainder. Other
        // material recipes and their remainder items are not yet supported.
        let Ok(amount) = u32::try_from(amount) else { return false; };
        if amount == 0 || remainder != 0 || !is(0, "tconstruct:pattern", 1) || !is(1, material, amount) { return false; }
        let material_id = if material == "minecraft:flint" { "tconstruct:flint" } else { "tconstruct:wood" };
        (part_with_material(result, material_id), vec![(0, 1), (1, amount)])
    } else if action == 24 {
        if !(is(0, "tconstruct:pick_head", 1) || is(0, "tconstruct:flint_pick_head", 1)) || !(is(1, "tconstruct:tool_handle", 1) || is(1, "tconstruct:wood_handle", 1)) || !(is(2, "tconstruct:tool_binding", 1) || is(2, "tconstruct:wood_binding", 1)) { return false; }
        for (slot, material) in [(0, "tconstruct:flint"), (1, "tconstruct:wood"), (2, "tconstruct:wood")] {
            if !part_matches_material(stacks[slot].as_ref().unwrap(), material) { return false; }
        }
        (fixed_pickaxe_with_materials(), vec![(0, 1), (1, 1), (2, 1)])
    } else if action == 25 {
        if !(is(0, "tconstruct:pickaxe", 1) || is(0, "tconstruct:flint_pickaxe", 1)) { return false; }
        let mut tool = stacks[0].as_ref().unwrap().clone();
        if tool.count != 1 || tool.damage == 0 { return false; }
        if (1..=2).any(|i| stacks[i].as_ref().is_some_and(|s| &*s.item != "minecraft:flint" || s.count == 0)) { return false; }
        let Ok(mut needed) = i32::try_from(tool.damage) else { return false; };
        let mut consume = vec![(0, 1)];
        for slot in 1..=2 {
            if let Some(stack) = &stacks[slot] {
                let Ok(available) = i32::try_from(stack.count) else { return false; };
                // Fixed prototype rate; material stats and modifier hooks still need porting.
                let (count, repaired) = repair_from_slot(needed, available, 56.0);
                if count > 0 { consume.push((slot, count as u32)); needed = (needed - repaired).max(0); }
            }
        }
        if consume.len() == 1 { return false; }
        tool.damage = needed as u32;
        (tool, consume)
    } else { return false; };
    if let Some(existing) = carried.as_ref() {
        let max = if &*output.item == "tconstruct:flint_pickaxe" || &*output.item == "tconstruct:pickaxe" { 1 } else { 64 };
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
    fn original_part_count_depends_on_material_value_and_leftover() {
        assert_eq!(material_output_count(1, 1, 2, false), Some(1));
        assert_eq!(material_output_count(1, 3, 1, false), Some(3));
        assert_eq!(material_output_count(2, 5, 2, false), Some(5));
        assert_eq!(material_output_count(1, 3, 2, true), Some(1));
        assert_eq!(material_output_count(1, 3, 0, false), None);
    }
    #[test]
    fn original_repair_rounds_count_before_truncating_total() {
        let per_item = (85.0_f32 / 3.0) * 2.0;
        assert_eq!(repair_from_slot(85, 2, per_item), (2, 113));
        assert_eq!(repair_from_slot(85, 1, per_item), (1, 56));
        assert_eq!(repair_from_slot(29, 1, per_item), (1, 56));
        assert_eq!(repair_from_slot(85, 2, 0.0), (0, 0));
    }
    #[test]
    fn part_take_is_atomic_and_consumes_default_pattern() {
        let mut slots = vec![None; 27];
        slots[0] = Some(ItemStack::new("tconstruct:pattern", 1));
        slots[1] = Some(ItemStack::new("minecraft:flint", 2));
        let mut cursor = Some(ItemStack::new("minecraft:stick", 1));
        let before = slots.clone();
        assert!(!take(&mut slots, &mut cursor, false, 24));
        assert_eq!(slots, before);
        cursor = None;
        assert!(take(&mut slots, &mut cursor, false, 24));
        assert_eq!(cursor.as_ref().unwrap().item.as_ref(), "tconstruct:pick_head");
        assert_eq!(cursor.as_ref().unwrap().data().and_then(|d| d.other("minecraft:custom_data")), Some(&horizon_world::nbt::Tag::compound([("Material", horizon_world::nbt::Tag::String("tconstruct:flint".into()))])));
        assert!(slots[1].is_none());
        assert!(slots[0].is_none());
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
        assert_eq!(part_preview(&slots, 0).unwrap().item.as_ref(), "tconstruct:pick_head");
        assert_eq!(slots, before);
        assert!(part_preview(&slots, 1).is_none());
        slots[1] = Some(ItemStack::new("minecraft:flint", 1));
        assert!(part_preview(&slots, 0).is_none());
    }
    #[test]
    fn assembly_retains_materials_and_rejects_conflicting_identity() {
        assert!(!part_matches_material(&ItemStack::new("tconstruct:pick_head", 1), "tconstruct:flint"));
        assert!(part_matches_material(&ItemStack::new("tconstruct:flint_pick_head", 1), "tconstruct:flint"));
        let mut slots = vec![None; 27];
        slots[0] = Some(part_with_material("tconstruct:flint_pick_head", "tconstruct:flint"));
        slots[1] = Some(part_with_material("tconstruct:wood_handle", "tconstruct:wood"));
        slots[2] = Some(part_with_material("tconstruct:wood_binding", "tconstruct:flint"));
        let before = slots.clone();
        let mut cursor = None;
        assert!(!take(&mut slots, &mut cursor, true, 24));
        assert_eq!(slots, before);
        slots[2] = Some(part_with_material("tconstruct:wood_binding", "tconstruct:wood"));
        assert!(take(&mut slots, &mut cursor, true, 24));
        let data = cursor.unwrap();
        let list = data.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|v| v.get("tic_materials"));
        assert_eq!(list, Some(&horizon_world::nbt::Tag::List(["tconstruct:flint", "tconstruct:wood", "tconstruct:wood"].into_iter().map(|id| horizon_world::nbt::Tag::String(id.into())).collect())));
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
