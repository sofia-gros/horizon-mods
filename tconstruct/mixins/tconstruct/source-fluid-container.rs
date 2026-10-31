// The source heating-structure menu owns two temporary container slots per
// viewer. This module stages one bucket transfer; the caller owns the slots,
// synchronizes them, and returns remaining stacks when the menu closes.
use horizon_items::items::ItemStack;
use super::tconstruct_source_smeltery_tank::SmelteryTank;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Direction { Auto, EmptyItem, FillItem }
impl Direction {
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
    if slots[1].is_some() { return false; }
    let Some(input) = slots[0].as_ref() else { return false; };
    if input.count == 0 || input.damage != 0 || input.extra.is_some() || !known_item(&input.item) { return false; }
    let mut next = tank.clone();
    let replacement = if input.item.as_ref() == "minecraft:bucket" && direction != Direction::EmptyItem {
        let Some(fluid) = next.fluids.first().map(|fluid| fluid.id.clone()) else { return false; };
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
}
