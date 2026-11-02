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
pub(super) fn is_bucket(stack: &ItemStack, known_fluid: &impl Fn(&str) -> bool) -> bool {
    stack.item.as_ref() == "minecraft:bucket" || bucket_fluid(&stack.item).is_some_and(|fluid| known_fluid(&fluid))
}
fn bottle_fluid(item: &str) -> Option<&'static str> {
    match item {
        "tconstruct:earth_slime_bottle" => Some("tconstruct:earth_slime"),
        "tconstruct:sky_slime_bottle" => Some("tconstruct:sky_slime"),
        "tconstruct:ichor_slime_bottle" => Some("tconstruct:ichor"),
        "tconstruct:ender_slime_bottle" => Some("tconstruct:ender_slime"),
        _ => None,
    }
}
fn filled_bottle(fluid: &str) -> Option<&'static str> {
    match fluid {
        "tconstruct:earth_slime" => Some("tconstruct:earth_slime_bottle"),
        "tconstruct:sky_slime" => Some("tconstruct:sky_slime_bottle"),
        "tconstruct:ichor" => Some("tconstruct:ichor_slime_bottle"),
        "tconstruct:ender_slime" => Some("tconstruct:ender_slime_bottle"),
        _ => None,
    }
}
// Copper cans carry exactly 90 mB per item; fluid tags are part of identity.
fn can_contents(stack: &ItemStack) -> Option<(Option<String>, Option<horizon_world::nbt::Tag>)> {
    use horizon_world::nbt::Tag;
    if stack.item.as_ref() != "tconstruct:copper_can" || stack.damage != 0 || stack.count == 0 { return None; }
    match stack.data().and_then(|data| data.other("minecraft:custom_data")) {
        None => Some((None, None)),
        Some(Tag::Compound(data)) => {
            let fluid = match data.get(&"fluid".into()) { None => None, Some(Tag::String(id)) => { let value = id.to_string_lossy(); if value == "minecraft:empty" { None } else { Some((!value.is_empty()).then_some(value)?) } }, _ => return None };
            let tag = match data.get(&"fluid_tag".into()) { None => None, Some(tag @ Tag::Compound(_)) if fluid.is_some() => Some(tag.clone()), _ => return None };
            Some((fluid, tag))
        }
        _ => None,
    }
}
pub(super) fn can_fluid(stack: &ItemStack) -> Option<Option<String>> {
    let (fluid, tag) = can_contents(stack)?;
    tag.is_none().then_some(fluid)
}
pub(super) fn can_set_fluid(stack: &ItemStack, fluid: Option<&str>) -> Option<ItemStack> {
    can_set_tagged(stack, fluid, None)
}
fn can_set_tagged(stack: &ItemStack, fluid: Option<&str>, tag: Option<&horizon_world::nbt::Tag>) -> Option<ItemStack> {
    use horizon_world::nbt::{Compound, Tag};
    can_contents(stack)?;
    if tag.is_some_and(|tag| !matches!(tag, Tag::Compound(_))) || (fluid.is_none() && tag.is_some()) { return None; }
    let mut data = match stack.data().and_then(|value| value.other("minecraft:custom_data")) {
        None => Compound::new(), Some(Tag::Compound(value)) => value.clone(), _ => return None,
    };
    match fluid { Some(fluid) if !fluid.is_empty() && fluid != "minecraft:empty" => { data.insert("fluid".into(), Tag::String(fluid.into())); }, None => { data.remove(&"fluid".into()); }, _ => return None }
    match tag { Some(tag) => { data.insert("fluid_tag".into(), tag.clone()); }, None => { data.remove(&"fluid_tag".into()); } }
    let mut result = stack.clone(); result.count = 1;
    result.data_mut().set_other("minecraft:custom_data", (!data.is_empty()).then_some(Tag::Compound(data)));
    Some(result)
}
pub(super) fn accepts(stack: &ItemStack, known_fluid: &impl Fn(&str) -> bool) -> bool {
    if let Some((fluid, _)) = can_contents(stack) { return fluid.as_deref().is_none_or(known_fluid); }
    if stack.item.as_ref() == "minecraft:potion" { return simple_potion(stack).is_some(); }
    stack.count > 0 && stack.damage == 0 && stack.extra.is_none() && (is_bucket(stack, known_fluid) || stack.item.as_ref() == "minecraft:glass_bottle" || bottle_fluid(&stack.item).is_some_and(known_fluid))
}
// The host currently models a potion ID separately from its other components.
// Only accept the lossless simple case until custom effects/color are modeled.
fn simple_potion(stack: &ItemStack) -> Option<String> {
    if stack.item.as_ref() != "minecraft:potion" || stack.count == 0 || stack.damage != 0 { return None; }
    let potion = stack.potion()?;
    let plain = ItemStack::new("minecraft:potion", 1).with_potion(potion);
    stack.same_item_same_components(&plain).then(|| potion.to_owned())
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
    let max = if held.item.as_ref() == "tconstruct:copper_can" || held.item.as_ref() == "minecraft:bucket" || held.item.as_ref() == "minecraft:glass_bottle" || bottle_fluid(&held.item).is_some() { 16 } else { 1 };
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
    let max = if held.item.as_ref() == "tconstruct:copper_can" || held.item.as_ref() == "minecraft:bucket" || held.item.as_ref() == "minecraft:glass_bottle" || bottle_fluid(&held.item).is_some() { 16 } else { 1 };
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
pub(super) fn filled_bucket(fluid: &str) -> Option<String> {
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
    if input.count == 0 || input.damage != 0 || !known_item(&input.item) { return false; }
    if input.item.as_ref() == "minecraft:potion" {
        use horizon_world::nbt::Tag;
        if direction == Direction::FillItem || !known_item("minecraft:glass_bottle") { return false; }
        let Some(potion) = simple_potion(input) else { return false; };
        let (fluid, metadata) = if potion == "minecraft:water" { ("minecraft:water", None) }
            else { ("tconstruct:potion", Some(Tag::compound([("Potion", Tag::String(potion.as_str().into()))]))) };
        let mut next = tank.clone();
        if !known_fluid(fluid) || !next.insert_tagged(fluid, metadata.as_ref(), 250) { return false; }
        let mut remaining = input.clone(); remaining.count -= 1;
        slots[0] = (remaining.count > 0).then_some(remaining);
        slots[1] = Some(ItemStack::new("minecraft:glass_bottle", 1)); *tank = next;
        return true;
    }
    if input.item.as_ref() == "tconstruct:copper_can" {
        let Some((existing, metadata)) = can_contents(input) else { return false; };
        let mut next = tank.clone();
        let output = if let Some(fluid) = existing {
            if direction == Direction::FillItem || !known_fluid(&fluid) || !next.insert_tagged(&fluid, metadata.as_ref(), 90) { return false; }
            let Some(output) = can_set_tagged(input, None, None) else { return false; }; output
        } else {
            if direction == Direction::EmptyItem { return false; }
            let entry = if let Some(id) = selected {
                let mut choices = next.fluids.iter().filter(|fluid| fluid.id == id);
                let Some(entry) = choices.next() else { return false; };
                if choices.next().is_some() { return false; }
                entry
            } else { let Some(entry) = next.fluids.first() else { return false; }; entry };
            let (fluid, tag) = (entry.id.clone(), entry.tag.clone());
            if !known_fluid(&fluid) || !next.take_tagged(&fluid, tag.as_ref(), 90) { return false; }
            let Some(output) = can_set_tagged(input, Some(&fluid), tag.as_ref()) else { return false; }; output
        };
        let mut remaining = input.clone(); remaining.count -= 1;
        slots[0] = (remaining.count > 0).then_some(remaining); slots[1] = Some(output); *tank = next;
        return true;
    }
    // Mantle's standard glass-bottle transfers fill a base potion from the
    // potion fluid or a water potion from water. Other potion tags remain
    // rejected until the host can represent them without dropping fields.
    if input.item.as_ref() == "minecraft:glass_bottle" && input.extra.is_none() && direction != Direction::EmptyItem {
        use horizon_world::nbt::Tag;
        let entry = if let Some(id) = selected {
            let mut choices = tank.fluids.iter().filter(|fluid| fluid.id == id);
            let Some(entry) = choices.next() else { return false; };
            if choices.next().is_some() { return false; }
            entry
        } else { let Some(entry) = tank.fluids.first() else { return false; }; entry };
        let is_potion = entry.id == "tconstruct:potion";
        let is_water = entry.id == "minecraft:water" && entry.tag.is_none();
        if is_potion || is_water {
            let potion = if is_water { "minecraft:water".to_owned() } else {
                let Some(Tag::Compound(fields)) = entry.tag.as_ref() else { return false; };
                if fields.len() != 1 { return false; }
                let Some(Tag::String(potion)) = fields.get(&"Potion".into()) else { return false; };
                let potion = potion.to_string_lossy();
                if potion.is_empty() || potion == "minecraft:empty" { return false; }
                potion
            };
            let (fluid, tag) = (entry.id.clone(), entry.tag.clone());
            let amount = if is_water { 500 } else { 250 };
            let mut next = tank.clone();
            if !known_item("minecraft:potion") || !known_fluid(&fluid) || !next.take_tagged(&fluid, tag.as_ref(), amount) { return false; }
            let mut remaining = input.clone(); remaining.count -= 1;
            slots[0] = (remaining.count > 0).then_some(remaining);
            slots[1] = Some(ItemStack::new("minecraft:potion", 1).with_potion(&potion)); *tank = next;
            return true;
        }
    }
    if input.extra.is_some() { return false; }
    let mut next = tank.clone();
    let replacement = if input.item.as_ref() == "minecraft:glass_bottle" && direction != Direction::EmptyItem {
        let Some(fluid) = selected.map(str::to_owned).or_else(|| next.fluids.first().map(|fluid| fluid.id.clone())) else { return false; };
        if next.fluids.iter().any(|entry| entry.id == fluid && entry.tag.is_some()) { return false; }
        let Some(bottle) = filled_bottle(&fluid) else { return false; };
        if !known_fluid(&fluid) || !known_item(bottle) || !next.take(&fluid, 250) { return false; }
        bottle.to_owned()
    } else if let Some(fluid) = bottle_fluid(&input.item) {
        if direction == Direction::FillItem || !known_fluid(fluid) || !known_item("minecraft:glass_bottle") || !next.insert(fluid, 250) { return false; }
        "minecraft:glass_bottle".to_owned()
    } else if input.item.as_ref() == "minecraft:bucket" && direction != Direction::EmptyItem {
        let Some(fluid) = selected.map(str::to_owned).or_else(|| next.fluids.first().map(|fluid| fluid.id.clone())) else { return false; };
        if next.fluids.iter().any(|entry| entry.id == fluid && entry.tag.is_some()) { return false; }
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

/// Apply the same bucket rules to the Melter's single-fluid tank.
pub(super) fn transfer_single(tank: &mut super::tconstruct_source_melter_state::Tank, slots: &mut [Option<ItemStack>; 2], direction: Direction,
    known_item: &impl Fn(&str) -> bool, known_fluid: &impl Fn(&str) -> bool) -> bool {
    let mut staged = SmelteryTank::new(tank.capacity);
    if let Some(id) = tank.fluid.as_deref() {
        if !staged.insert_tagged(id, tank.tag.as_ref(), tank.amount) { return false; }
    }
    let mut staged_slots = slots.clone();
    if !transfer(&mut staged, &mut staged_slots, direction, known_item, known_fluid) { return false; }
    if staged.fluids.len() > 1 { return false; }
    *slots = staged_slots;
    tank.amount = staged.total();
    tank.fluid = staged.fluids.first().map(|fluid| fluid.id.clone());
    tank.tag = staged.fluids.first().and_then(|fluid| fluid.tag.clone());
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    fn known(_: &str) -> bool { true }
    #[test]
    fn slime_bottle_transfer_is_atomic_and_uses_250_mb() {
        let mut tank = SmelteryTank::new(300);
        let mut slots = [Some(ItemStack::new("tconstruct:sky_slime_bottle",2)),None];
        assert!(transfer(&mut tank,&mut slots,Direction::Auto,&known,&known));
        assert_eq!(tank.amount("tconstruct:sky_slime"),250);
        assert_eq!(slots[0].as_ref().unwrap().count,1);
        assert_eq!(slots[1].as_ref().unwrap().item.as_ref(),"minecraft:glass_bottle");
        slots=[Some(ItemStack::new("minecraft:glass_bottle",2)),None];
        assert!(transfer(&mut tank,&mut slots,Direction::Auto,&known,&known));
        assert_eq!(tank.total(),0);
        assert_eq!(slots[0].as_ref().unwrap().count,1);
        assert_eq!(slots[1].as_ref().unwrap().item.as_ref(),"tconstruct:sky_slime_bottle");
        let before=(tank.clone(),slots.clone());
        assert!(!transfer(&mut tank,&mut slots,Direction::Auto,&known,&known));
        assert_eq!((tank,slots),before);
    }
    #[test]
    fn copper_can_moves_one_ingot_and_retains_other_stack_items() {
        let mut tank = SmelteryTank::new(180);
        assert!(tank.insert("minecraft:lava", 90));
        let mut slots = [Some(ItemStack::new("tconstruct:copper_can", 2)), None];
        assert!(transfer(&mut tank, &mut slots, Direction::FillItem, &known, &known));
        assert_eq!(tank.total(), 0);
        assert_eq!(slots[0].as_ref().unwrap().count, 1);
        let filled = slots[1].take().unwrap();
        assert_eq!(can_fluid(&filled), Some(Some("minecraft:lava".into())));
        slots = [Some(filled), None];
        assert!(transfer(&mut tank, &mut slots, Direction::EmptyItem, &known, &known));
        assert_eq!(tank.amount("minecraft:lava"), 90);
        assert_eq!(can_fluid(slots[1].as_ref().unwrap()), Some(None));
    }
    #[test]
    fn single_fluid_tank_rejects_mixing_without_losing_container() {
        let mut tank = super::super::tconstruct_source_melter_state::Tank::new(3000);
        assert!(tank.insert("minecraft:lava", 1000));
        let mut slots = [Some(ItemStack::new("minecraft:water_bucket", 1)), None];
        let before = slots.clone();
        assert!(!transfer_single(&mut tank, &mut slots, Direction::EmptyItem, &known, &known));
        assert_eq!(slots, before); assert_eq!(tank.amount, 1000);
        slots = [Some(ItemStack::new("minecraft:bucket", 1)), None];
        assert!(transfer_single(&mut tank, &mut slots, Direction::FillItem, &known, &known));
        assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), "minecraft:lava_bucket");
        assert_eq!(tank.amount, 0);
    }
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
