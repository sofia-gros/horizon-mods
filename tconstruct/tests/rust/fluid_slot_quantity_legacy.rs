// Complete unchanged old quantity helpers. Dependencies are the real parent module.
pub(super) fn click_with_color(slots: &mut [Option<ItemStack>; 2], carried: &mut Option<ItemStack>, index: usize,
    right: bool, known_fluid: &impl Fn(&str) -> bool, allow_color: bool) -> bool {
    if index > 1 { return false; }
    if index == 1 && carried.is_some() { return false; }
    let Some(mut held) = carried.take() else {
        let Some(mut stack) = slots[index].take() else { return false; };
        let count = if right { stack.count.div_ceil(2) } else { stack.count };
        stack.count -= count;
        let mut taken = stack.clone(); taken.count = count;
        slots[index] = (stack.count > 0).then_some(stack);
        *carried = Some(taken);
        return true;
    };
    if index != 0 || !accepts_with_color(&held, known_fluid, allow_color) { *carried = Some(held); return false; }
    let max = if matches!(&*held.item, "tconstruct:seared_lantern" | "tconstruct:scorched_lantern" | "minecraft:bowl" | "minecraft:sponge" | "minecraft:glass_bottle" | "tconstruct:splash_bottle" | "tconstruct:lingering_bottle") { 64 } else if held.item.as_ref() == "tconstruct:copper_can" || held.item.as_ref() == "minecraft:bucket" || bottle_fluid(&held.item).is_some() { 16 } else { 1 };
    match slots[index].as_mut() {
        None => {
            let count = if right { 1 } else { held.count.min(max) };
            let mut placed = held.clone(); placed.count = count; held.count -= count;
            slots[index] = Some(placed); *carried = (held.count > 0).then_some(held);
        }
        Some(existing) if existing.item == held.item && existing.damage == held.damage && existing.extra == held.extra => {
            let available = max.saturating_sub(existing.count);
            let count = if right { held.count.min(available).min(1) } else { held.count.min(available) };
            if count == 0 { *carried = Some(held); return false; }
            existing.count += count; held.count -= count; *carried = (held.count > 0).then_some(held);
        }
        Some(_) => { *carried = Some(held); return false; }
    }
    true
}

pub(super) fn quick_input_with_color(slots: &mut [Option<ItemStack>; 2], stack: &mut Option<ItemStack>,
    known_fluid: &impl Fn(&str) -> bool, allow_color: bool) -> bool {
    let Some(held) = stack.as_mut() else { return false; };
    if !accepts_with_color(held, known_fluid, allow_color) { return false; }
    let max = if matches!(&*held.item, "tconstruct:seared_lantern" | "tconstruct:scorched_lantern" | "minecraft:bowl" | "minecraft:sponge" | "minecraft:glass_bottle" | "tconstruct:splash_bottle" | "tconstruct:lingering_bottle") { 64 } else if held.item.as_ref() == "tconstruct:copper_can" || held.item.as_ref() == "minecraft:bucket" || bottle_fluid(&held.item).is_some() { 16 } else { 1 };
    let amount = match slots[0].as_mut() {
        None => {
            let amount = held.count.min(max);
            let mut placed = held.clone(); placed.count = amount;
            slots[0] = Some(placed); amount
        }
        Some(existing) if existing.item == held.item && existing.damage == held.damage && existing.extra == held.extra => {
            let amount = held.count.min(max.saturating_sub(existing.count));
            existing.count += amount; amount
        }
        _ => 0,
    };
    if amount == 0 { return false; }
    held.count -= amount;
    if held.count == 0 { *stack = None; }
    true
}
