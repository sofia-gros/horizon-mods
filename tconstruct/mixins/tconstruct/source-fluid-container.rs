// The source heating-structure menu owns two temporary container slots per
// viewer. This module stages one bucket transfer; the caller owns the slots,
// synchronizes them, and returns remaining stacks when the menu closes.
use horizon_items::items::ItemStack;
use super::tconstruct_source_smeltery_tank::SmelteryTank;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Direction { Auto, EmptyItem, FillItem }
impl Direction {
    pub(super) fn from_index(index: u8) -> Self {
        match index { 1 => Self::EmptyItem, 2 => Self::FillItem, _ => Self::Auto }
    }
    pub(super) fn index(self) -> u8 {
        match self { Self::Auto => 0, Self::EmptyItem => 1, Self::FillItem => 2 }
    }
    pub(super) fn next(self) -> Self {
        match self { Self::Auto => Self::EmptyItem, Self::EmptyItem => Self::FillItem, Self::FillItem => Self::Auto }
    }
}

fn bucket_fluid(item: &str) -> Option<String> {
    match item {
        "minecraft:water_bucket" => Some("minecraft:water".into()),
        "minecraft:lava_bucket" => Some("minecraft:lava".into()),
        _ => item.strip_prefix("tconstruct:")?.strip_suffix("_bucket").filter(|id| !id.is_empty()).map(|id| format!("tconstruct:{id}")),
    }
}
pub(super) fn accepts(stack: &ItemStack, known_fluid: &impl Fn(&str) -> bool) -> bool {
    stack.count > 0 && stack.damage == 0 && stack.extra.is_none()
        && (stack.item.as_ref() == "minecraft:bucket" || bucket_fluid(&stack.item).is_some_and(|fluid| known_fluid(&fluid)))
}

/// A normal left/right click on either temporary slot. The output never
/// accepts inserted items; all changes are staged in caller-owned copies.
pub(super) fn click(slots: &mut [Option<ItemStack>; 2], carried: &mut Option<ItemStack>, index: usize,
    right: bool, known_fluid: &impl Fn(&str) -> bool) -> bool {
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
    if index != 0 || !accepts(&held, known_fluid) { *carried = Some(held); return false; }
    let max = if held.item.as_ref() == "minecraft:bucket" { 16 } else { 1 };
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
/// Stage a shift move from player inventory into the input slot.
pub(super) fn quick_input(slots: &mut [Option<ItemStack>; 2], stack: &mut Option<ItemStack>,
    known_fluid: &impl Fn(&str) -> bool) -> bool {
    let Some(held) = stack.as_mut() else { return false; };
    if !accepts(held, known_fluid) { return false; }
    let max = if held.item.as_ref() == "minecraft:bucket" { 16 } else { 1 };
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
fn filled_bucket(fluid: &str) -> Option<String> {
    match fluid {
        "minecraft:water" => Some("minecraft:water_bucket".into()),
        "minecraft:lava" => Some("minecraft:lava_bucket".into()),
        _ => fluid.strip_prefix("tconstruct:").filter(|id| !id.is_empty() && !id.starts_with("flowing_"))
            .map(|id| format!("tconstruct:{id}_bucket")),
    }
}

/// Transfer exactly one standard bucket only if both tank and output can be
/// committed. Component-bearing stacks are left untouched until their source
/// semantics are supported; no custom data is silently discarded.
pub(super) fn transfer(tank: &mut SmelteryTank, slots: &mut [Option<ItemStack>; 2], direction: Direction,
    known_item: &impl Fn(&str) -> bool, known_fluid: &impl Fn(&str) -> bool) -> bool {
    transfer_selected(tank, slots, direction, None, known_item, known_fluid)
}

pub(super) fn transfer_selected(tank: &mut SmelteryTank, slots: &mut [Option<ItemStack>; 2], direction: Direction,
    selected: Option<&str>, known_item: &impl Fn(&str) -> bool, known_fluid: &impl Fn(&str) -> bool) -> bool {
    if slots[1].is_some() { return false; }
    let Some(input) = slots[0].as_ref() else { return false; };
    if input.count == 0 || input.damage != 0 || input.extra.is_some() || !known_item(&input.item) { return false; }
    let mut next = tank.clone();
    let replacement = if input.item.as_ref() == "minecraft:bucket" && direction != Direction::EmptyItem {
        let Some(fluid) = selected.map(str::to_owned).or_else(|| next.fluids.first().map(|fluid| fluid.id.clone())) else { return false; };
        let Some(bucket) = filled_bucket(&fluid) else { return false; };
        if !known_fluid(&fluid) || !known_item(&bucket) || !next.take(&fluid, 1000) { return false; }
        bucket
    } else if direction != Direction::FillItem {
        let Some(fluid) = bucket_fluid(&input.item) else { return false; };
        if !known_fluid(&fluid) || !known_item("minecraft:bucket") || !next.insert(&fluid, 1000) { return false; }
        "minecraft:bucket".to_owned()
    } else { return false; };
    let mut remaining = input.clone();
    remaining.count -= 1;
    slots[0] = (remaining.count > 0).then_some(remaining);
    slots[1] = Some(ItemStack::new(&replacement, 1));
    *tank = next;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    fn known(_: &str) -> bool { true }
    #[test]
    fn slot_clicks_conserve_stacks_and_cannot_insert_into_output() {
        let mut slots = [None, None];
        let mut carried = Some(ItemStack::new("minecraft:bucket", 3));
        assert!(click(&mut slots, &mut carried, 0, true, &known));
        assert_eq!((slots[0].as_ref().unwrap().count, carried.as_ref().unwrap().count), (1, 2));
        let before = (slots.clone(), carried.clone());
        assert!(!click(&mut slots, &mut carried, 1, false, &known));
        assert_eq!((slots.clone(), carried.clone()), before);
        assert!(click(&mut slots, &mut carried, 0, false, &known));
        assert_eq!(slots[0].as_ref().unwrap().count, 3);
        assert!(carried.is_none());
        assert!(click(&mut slots, &mut carried, 0, true, &known));
        assert_eq!((slots[0].as_ref().unwrap().count, carried.as_ref().unwrap().count), (1, 2));
    }
    #[test]
    fn quick_input_conserves_buckets_and_rejects_invalid_stacks() {
        let mut slots = [None, None];
        let mut held = Some(ItemStack::new("minecraft:bucket", 20));
        assert!(quick_input(&mut slots, &mut held, &known));
        assert_eq!(slots[0].as_ref().unwrap().count, 16);
        assert_eq!(held.as_ref().unwrap().count, 4);
        assert!(!quick_input(&mut slots, &mut held, &known));
        assert_eq!(held.as_ref().unwrap().count, 4);
        slots = [None, None]; held = Some(ItemStack::new("minecraft:stone", 1));
        assert!(!quick_input(&mut slots, &mut held, &known));
        assert_eq!(held.as_ref().unwrap().count, 1);
    }
    #[test]
    fn fills_and_empties_one_bucket_without_losing_a_stack() {
        let mut tank = SmelteryTank::new(2000);
        let mut slots = [Some(ItemStack::new("minecraft:lava_bucket", 1)), None];
        assert!(transfer(&mut tank, &mut slots, Direction::Auto, &known, &known));
        assert_eq!(tank.amount("minecraft:lava"), 1000);
        assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), "minecraft:bucket");
        slots = [Some(ItemStack::new("minecraft:bucket", 2)), None];
        assert!(transfer(&mut tank, &mut slots, Direction::Auto, &known, &known));
        assert_eq!(tank.total(), 0);
        assert_eq!(slots[0].as_ref().unwrap().count, 1);
        assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), "minecraft:lava_bucket");
    }
    #[test]
    fn full_tank_or_blocked_output_preserves_everything() {
        let mut tank = SmelteryTank::new(999);
        let mut slots = [Some(ItemStack::new("minecraft:lava_bucket", 1)), None];
        let before = slots.clone();
        assert!(!transfer(&mut tank, &mut slots, Direction::Auto, &known, &known));
        assert_eq!(slots, before); assert_eq!(tank.total(), 0);
        tank.capacity = 2000; slots[1] = Some(ItemStack::new("minecraft:stone", 1));
        let before = slots.clone();
        assert!(!transfer(&mut tank, &mut slots, Direction::Auto, &known, &known));
        assert_eq!(slots, before); assert_eq!(tank.total(), 0);
    }
    #[test]
    fn source_material_bucket_requires_a_registered_output() {
        let mut tank = SmelteryTank::new(2000);
        assert!(tank.insert("tconstruct:molten_copper", 1000));
        let mut slots = [Some(ItemStack::new("minecraft:bucket", 1)), None];
        assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &|item| item != "tconstruct:molten_copper_bucket", &known));
        assert_eq!(tank.amount("tconstruct:molten_copper"), 1000);
        assert!(transfer(&mut tank, &mut slots, Direction::FillItem, &known, &known));
        assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), "tconstruct:molten_copper_bucket");
    }
    #[test]
    fn selected_fluid_is_drained_without_reordering_other_layers() {
        let mut tank = SmelteryTank::new(3000);
        assert!(tank.insert("minecraft:water", 1000)); assert!(tank.insert("minecraft:lava", 2000));
        let mut slots = [Some(ItemStack::new("minecraft:bucket", 1)), None];
        assert!(transfer_selected(&mut tank, &mut slots, Direction::FillItem, Some("minecraft:lava"), &known, &known));
        assert_eq!(tank.fluids[0].id, "minecraft:water");
        assert_eq!(tank.amount("minecraft:lava"), 1000);
        assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), "minecraft:lava_bucket");
    }
}
