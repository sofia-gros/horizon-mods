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
        "tconstruct:venom_bottle" => Some("tconstruct:venom"),
        "tconstruct:magma_bottle" => Some("tconstruct:magma"),
        "tconstruct:earth_slime_bottle" => Some("tconstruct:earth_slime"),
        "tconstruct:sky_slime_bottle" => Some("tconstruct:sky_slime"),
        "tconstruct:ichor_slime_bottle" => Some("tconstruct:ichor"),
        "tconstruct:ender_slime_bottle" => Some("tconstruct:ender_slime"),
        _ => None,
    }
}
fn filled_bottle(fluid: &str) -> Option<&'static str> {
    match fluid {
        "tconstruct:honey" | "tconstruct:flowing_honey" => Some("minecraft:honey_bottle"),
        "tconstruct:venom" | "tconstruct:flowing_venom" => Some("tconstruct:venom_bottle"),
        "tconstruct:magma" => Some("tconstruct:magma_bottle"),
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
/// Source NoContainerIngredient for the explicitly supported TConstruct
/// portable containers. Opaque or malformed data remains unknown.
pub(super) fn no_crafting_remainder(stack: &ItemStack) -> Option<bool> {
    use horizon_world::nbt::Tag;
    if stack.count == 0 || stack.wire_patch().is_some() { return None; }
    if stack.item.as_ref() == "tconstruct:copper_can" {
        return can_contents(stack).map(|(fluid, _)| fluid.is_none());
    }
    super::tconstruct_source_smeltery_fuel::item_capacity(&stack.item)?;
    match stack.data().and_then(|data| data.other("minecraft:custom_data")) {
        None => Some(true),
        Some(Tag::Compound(data)) => match data.get(&"tconstruct:tank".into()) {
            None => Some(true),
            Some(Tag::Compound(_)) => Some(false),
            Some(_) => None,
        },
        Some(_) => None,
    }
}
pub(super) fn can_fluid(stack: &ItemStack) -> Option<Option<String>> {
    let (fluid, tag) = can_contents(stack)?;
    tag.is_none().then_some(fluid)
}
/// Conservative display view only; transfer validation remains unchanged.
/// Tagged fluids need their own variant-aware names and colors before display.
pub(super) fn can_display_fluid(stack: &ItemStack, known: &impl Fn(&str) -> bool) -> Option<String> {
    if stack.wire_patch().is_some() || stack.data().is_some_and(|data| data.other("minecraft:item_model").is_some()) { return None; }
    let (fluid, tag) = can_contents(stack)?;
    let fluid = fluid?;
    if tag.is_some() || fluid == "tconstruct:potion" || !known(&fluid)
        || !(matches!(fluid.as_str(), "minecraft:water" | "minecraft:lava") || fluid.starts_with("tconstruct:")) { return None; }
    Some(fluid)
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
// Capacity is written during the narrow source-validated Station rebuild.
fn tool_capacity(stack: &ItemStack) -> Option<u32> {
    use horizon_world::nbt::Tag;
    if stack.count != 1 || !stack.item.starts_with("tconstruct:") || stack.wire_patch().is_some() { return None; }
    let data = stack.data()?.other("minecraft:custom_data")?;
    match data.get("tic_tank_capacity")? { Tag::Int(n) if *n > 0 => u32::try_from(*n).ok(), _ => None }
}

fn transfer_tool(tank: &mut SmelteryTank, slots: &mut [Option<ItemStack>; 2], direction: Direction,
    selected: Option<&str>, known_fluid: &impl Fn(&str) -> bool, allow_tags: bool) -> bool {
    let Some(input) = slots[0].as_ref() else { return false; };
    let Some(capacity) = tool_capacity(input) else { return false; };
    let Some(current) = super::pluto_stack_fluid::read(input, "tic_persistent", "tconstruct:tank_fluid") else { return false; };
    if current.as_ref().is_some_and(|f| f.amount > capacity || (!allow_tags && f.tag.is_some())) { return false; }
    let mut next = tank.clone();
    let mut after = current;
    let mut moved = false;
    if direction != Direction::FillItem {
        if let Some(fluid) = &mut after {
            let amount = fluid.amount.min(next.capacity.saturating_sub(next.total()));
            if amount > 0 && known_fluid(&fluid.id) && next.insert_tagged(&fluid.id, fluid.tag.as_ref(), amount) {
                fluid.amount -= amount; moved = true;
            }
        }
        if after.as_ref().is_some_and(|fluid| fluid.amount == 0) { after = None; }
    }
    if !moved && direction != Direction::EmptyItem {
        let entry = if let Some(id) = selected {
            let mut choices = next.fluids.iter().filter(|fluid| fluid.id == id);
            let Some(entry) = choices.next() else { return false; };
            if choices.next().is_some() { return false; } entry
        } else { let Some(entry) = next.fluids.first() else { return false; }; entry };
        if !known_fluid(&entry.id) || (!allow_tags && entry.tag.is_some()) { return false; }
        if after.as_ref().is_some_and(|fluid| fluid.id != entry.id || fluid.tag != entry.tag) { return false; }
        let (id, tag) = (entry.id.clone(), entry.tag.clone());
        let amount = entry.amount.min(capacity.saturating_sub(after.as_ref().map_or(0, |fluid| fluid.amount)));
        if amount == 0 || !next.take_tagged(&id, tag.as_ref(), amount) { return false; }
        if let Some(fluid) = &mut after { fluid.amount += amount; }
        else { after = Some(super::pluto_stack_fluid::StackFluid { id, amount, tag }); }
        moved = true;
    }
    if !moved { return false; }
    let Some(output) = super::pluto_stack_fluid::write(input, "tic_persistent", "tconstruct:tank_fluid", after.as_ref(), capacity) else { return false; };
    slots[0] = None; slots[1] = Some(output); *tank = next; true
}

pub(super) fn accepts(stack: &ItemStack, known_fluid: &impl Fn(&str) -> bool) -> bool {
    accepts_with_color(stack, known_fluid, false)
}
pub(super) fn accepts_with_color(stack: &ItemStack, known_fluid: &impl Fn(&str) -> bool, allow_color: bool) -> bool {
    if tool_capacity(stack).is_some() {
        return super::pluto_stack_fluid::read(stack, "tic_persistent", "tconstruct:tank_fluid")
            .is_some_and(|fluid| fluid.as_ref().is_none_or(|f| known_fluid(&f.id) && (allow_color || f.tag.is_none())));
    }
    if let Some((fluid, _)) = can_contents(stack) { return fluid.as_deref().is_none_or(known_fluid); }
    if matches!(&*stack.item, "minecraft:potion" | "minecraft:splash_potion" | "minecraft:lingering_potion") {
        return potion_data(stack, allow_color).is_some();
    }
    if matches!(&*stack.item, "tconstruct:seared_lantern" | "tconstruct:scorched_lantern") {
        return lantern_tank(stack).is_some();
    }
    stack.count > 0 && stack.damage == 0 && stack.extra.is_none() && (is_bucket(stack, known_fluid) || stack.item.as_ref() == "minecraft:glass_bottle" || stack.item.as_ref() == "minecraft:bowl" || stack.item.as_ref() == "minecraft:sponge" || (stack.item.as_ref() == "tconstruct:meat_soup" && known_fluid("tconstruct:meat_soup")) || bottle_fluid(&stack.item).is_some_and(known_fluid))
}
// Effect IDs changed from numeric bytes to namespaced IDs after 1.20.1.
// Only the source-era vanilla IDs and fields with unchanged NBT types are
// translated; unsupported effects stay in the original stack/tank.
const EFFECT_IDS: [&str; 33] = [
    "minecraft:speed", "minecraft:slowness", "minecraft:haste", "minecraft:mining_fatigue",
    "minecraft:strength", "minecraft:instant_health", "minecraft:instant_damage", "minecraft:jump_boost",
    "minecraft:nausea", "minecraft:regeneration", "minecraft:resistance", "minecraft:fire_resistance",
    "minecraft:water_breathing", "minecraft:invisibility", "minecraft:blindness", "minecraft:night_vision",
    "minecraft:hunger", "minecraft:weakness", "minecraft:poison", "minecraft:wither", "minecraft:health_boost",
    "minecraft:absorption", "minecraft:saturation", "minecraft:glowing", "minecraft:levitation",
    "minecraft:luck", "minecraft:unluck", "minecraft:slow_falling", "minecraft:conduit_power",
    "minecraft:dolphins_grace", "minecraft:bad_omen", "minecraft:hero_of_the_village", "minecraft:darkness",
];
fn effect_instance(value: &horizon_world::nbt::Tag, to_source: bool, depth: u8) -> Option<horizon_world::nbt::Tag> {
    use horizon_world::nbt::{Compound, Tag};
    if depth > 16 { return None; }
    let Tag::Compound(fields) = value else { return None; };
    let (id_key, target_id) = if to_source { ("id", "Id") } else { ("Id", "id") };
    let id = match (to_source, fields.get(&id_key.into())?) {
        (true, Tag::String(id)) => Tag::Byte((EFFECT_IDS.iter().position(|candidate| *candidate == id.to_string_lossy().as_str())? + 1) as i8),
        (false, Tag::Byte(id)) if *id > 0 => Tag::String((*EFFECT_IDS.get(*id as usize - 1)?).into()),
        _ => return None,
    };
    let mut result = Compound::new(); result.insert(target_id.into(), id);
    for (key, value) in fields {
        let name = key.to_string_lossy();
        if name == id_key { continue; }
        let (old, modern) = match (to_source, name.as_str()) {
            (true, "amplifier") | (false, "Amplifier") => ("Amplifier", "amplifier"),
            (true, "duration") | (false, "Duration") => ("Duration", "duration"),
            (true, "ambient") | (false, "Ambient") => ("Ambient", "ambient"),
            (true, "show_particles") | (false, "ShowParticles") => ("ShowParticles", "show_particles"),
            (true, "show_icon") | (false, "ShowIcon") => ("ShowIcon", "show_icon"),
            (true, "hidden_effect") | (false, "HiddenEffect") => ("HiddenEffect", "hidden_effect"),
            _ => return None,
        };
        let converted = match old {
            "HiddenEffect" => effect_instance(value, to_source, depth + 1)?,
            "Duration" if matches!(value, Tag::Int(_)) => value.clone(),
            "Amplifier" if matches!(value, Tag::Byte(_)) => value.clone(),
            "Ambient" | "ShowParticles" | "ShowIcon" if matches!(value, Tag::Byte(0 | 1)) => value.clone(),
            _ => return None,
        };
        result.insert((if to_source { old } else { modern }).into(), converted);
    }
    Some(Tag::Compound(result))
}
fn effects(value: &horizon_world::nbt::Tag, to_source: bool) -> Option<horizon_world::nbt::Tag> {
    use horizon_world::nbt::Tag;
    let Tag::List(values) = value else { return None; };
    Some(Tag::List(values.iter().map(|value| effect_instance(value, to_source, 0)).collect::<Option<Vec<_>>>()?))
}
fn potion_data(stack: &ItemStack, allow_color: bool) -> Option<(String, Option<i32>, Option<horizon_world::nbt::Tag>)> {
    use horizon_world::nbt::Tag;
    if !matches!(&*stack.item, "minecraft:potion" | "minecraft:splash_potion" | "minecraft:lingering_potion") || stack.count == 0 || stack.damage != 0 { return None; }
    let potion = stack.potion()?;
    let mut plain = ItemStack::new(&stack.item, 1).with_potion(potion);
    let extra = stack.data().map(|data| data.potion_extra.as_slice()).unwrap_or(&[]);
    let mut color = None; let mut custom_effects = None;
    for (key, value) in extra {
        match (key.as_str(), value) {
            ("custom_color", Tag::Int(value)) if allow_color && color.is_none() => color = Some(*value),
            ("custom_effects", value) if allow_color && custom_effects.is_none() => custom_effects = Some(effects(value, true)?),
            _ => return None,
        }
    }
    plain.data_mut().potion_extra = extra.to_vec();
    stack.same_item_same_components(&plain).then(|| (potion.to_owned(), color, custom_effects))
}

pub(super) fn potion_tag(tag: &horizon_world::nbt::Tag, allow_color: bool) -> Option<(String, Option<i32>, Option<horizon_world::nbt::Tag>)> {
    use horizon_world::nbt::Tag;
    let Tag::Compound(fields) = tag else { return None; };
    let Tag::String(potion) = fields.get(&"Potion".into())? else { return None; };
    let potion = potion.to_string_lossy();
    if potion.is_empty() || potion == "minecraft:empty" { return None; }
    let color = match fields.get(&"CustomPotionColor".into()) {
        None => None,
        Some(Tag::Int(color)) if allow_color => Some(*color),
        _ => return None,
    };
    let custom_effects = match fields.get(&"CustomPotionEffects".into()) {
        None => None,
        Some(value) if allow_color => Some(effects(value, false)?),
        _ => return None,
    };
    if fields.len() != 1 + usize::from(color.is_some()) + usize::from(custom_effects.is_some()) { return None; }
    Some((potion, color, custom_effects))
}
pub(super) fn potion_stack(item: &str, potion: &str, color: Option<i32>, custom_effects: Option<horizon_world::nbt::Tag>) -> ItemStack {
    use horizon_world::nbt::Tag;
    let mut result = ItemStack::new(item, 1).with_potion(potion);
    if let Some(color) = color { result.data_mut().potion_extra.push(("custom_color".into(), Tag::Int(color))); }
    if let Some(effects) = custom_effects { result.data_mut().potion_extra.push(("custom_effects".into(), effects)); }
    result
}

// Container-slot transfers operate on one item of the stack, as in Mantle's
// FluidTransferHelper. The item tank representation is shared with casting.
fn lantern_tank(stack: &ItemStack) -> Option<super::tconstruct_source_melter_state::Tank> {
    use horizon_world::nbt::Tag;
    if stack.count == 0 || stack.damage != 0 { return None; }
    let capacity = super::tconstruct_source_smeltery_fuel::item_capacity(&stack.item)?;
    let custom = stack.data().and_then(|data| data.other("minecraft:custom_data"));
    match custom {
        None => Some(super::tconstruct_source_melter_state::Tank::new(capacity)),
        Some(Tag::Compound(data)) => match data.get(&"tconstruct:tank".into()) {
            None => Some(super::tconstruct_source_melter_state::Tank::new(capacity)),
            Some(tag @ Tag::Compound(fields)) if fields.keys().all(|key| { let name = key.to_string_lossy(); name == "fluid" || name == "amount" || name == "fluid_tag" }) =>
                super::tconstruct_source_melter_state::Tank::from_nbt(tag, capacity),
            _ => None,
        },
        _ => None,
    }
}

/// A normal left/right click on either temporary slot. The output never
/// accepts inserted items; all changes are staged in caller-owned copies.
pub(super) fn click(slots: &mut [Option<ItemStack>; 2], carried: &mut Option<ItemStack>, index: usize,
    right: bool, known_fluid: &impl Fn(&str) -> bool) -> bool {
    click_with_color(slots, carried, index, right, known_fluid, false)
}
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
    let max = if matches!(&*held.item, "tconstruct:seared_lantern" | "tconstruct:scorched_lantern" | "minecraft:bowl" | "minecraft:sponge" | "minecraft:glass_bottle") { 64 } else if held.item.as_ref() == "tconstruct:copper_can" || held.item.as_ref() == "minecraft:bucket" || bottle_fluid(&held.item).is_some() { 16 } else { 1 };
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
    quick_input_with_color(slots, stack, known_fluid, false)
}
pub(super) fn quick_input_with_color(slots: &mut [Option<ItemStack>; 2], stack: &mut Option<ItemStack>,
    known_fluid: &impl Fn(&str) -> bool, allow_color: bool) -> bool {
    let Some(held) = stack.as_mut() else { return false; };
    if !accepts_with_color(held, known_fluid, allow_color) { return false; }
    let max = if matches!(&*held.item, "tconstruct:seared_lantern" | "tconstruct:scorched_lantern" | "minecraft:bowl" | "minecraft:sponge" | "minecraft:glass_bottle") { 64 } else if held.item.as_ref() == "tconstruct:copper_can" || held.item.as_ref() == "minecraft:bucket" || bottle_fluid(&held.item).is_some() { 16 } else { 1 };
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
    transfer_selected_with_tags(tank, slots, direction, selected, known_item, known_fluid, None)
}
pub(super) fn transfer_selected_with_tags(tank: &mut SmelteryTank, slots: &mut [Option<ItemStack>; 2], direction: Direction,
    selected: Option<&str>, known_item: &impl Fn(&str) -> bool, known_fluid: &impl Fn(&str) -> bool,
    item_tags: Option<&std::collections::HashMap<String, std::collections::HashSet<String>>>) -> bool {
    transfer_selected_with_color(tank, slots, direction, selected, known_item, known_fluid, item_tags, false)
}
pub(super) fn transfer_selected_with_color(tank: &mut SmelteryTank, slots: &mut [Option<ItemStack>; 2], direction: Direction,
    selected: Option<&str>, known_item: &impl Fn(&str) -> bool, known_fluid: &impl Fn(&str) -> bool,
    item_tags: Option<&std::collections::HashMap<String, std::collections::HashSet<String>>>, allow_color: bool) -> bool {
    if slots[1].is_some() { return false; }
    let Some(input) = slots[0].as_ref() else { return false; };
    if input.count == 0 || !known_item(&input.item) { return false; }
    if tool_capacity(input).is_some() { return transfer_tool(tank, slots, direction, selected, known_fluid, allow_color); }
    if input.damage != 0 { return false; }
    if matches!(&*input.item, "minecraft:potion" | "minecraft:splash_potion" | "minecraft:lingering_potion") {
        use horizon_world::nbt::Tag;
        if direction == Direction::FillItem { return false; }
        let Some((potion, color, custom_effects)) = potion_data(input, allow_color) else { return false; };
        let empty = match &*input.item {
            "minecraft:potion" => "minecraft:glass_bottle".to_owned(),
            item => {
                let Some(tags) = item_tags else { return false; };
                let key = if item == "minecraft:splash_potion" { "forge:bottles/splash" } else { "forge:bottles/lingering" };
                let mut available = tags.get(key).into_iter().flatten().filter(|id| known_item(id));
                match available.next() {
                    Some(id) => { let id = id.clone(); if available.next().is_some() { return false; } id },
                    None => "minecraft:glass_bottle".to_owned(),
                }
            }
        };
        if !known_item(&empty) { return false; }
        // Mantle's EmptyPotionTransfer explicitly returns plain water for
        // every water-base potion, including one with a custom color.
        let (fluid, metadata) = if potion == "minecraft:water" { ("minecraft:water", None) }
            else {
                let mut fields = vec![("Potion", Tag::String(potion.as_str().into()))];
                if let Some(color) = color { fields.push(("CustomPotionColor", Tag::Int(color))); }
                if let Some(effects) = custom_effects { fields.push(("CustomPotionEffects", effects)); }
                ("tconstruct:potion", Some(Tag::compound(fields)))
            };
        let mut next = tank.clone();
        if !known_fluid(fluid) || !next.insert_tagged(fluid, metadata.as_ref(), 250) { return false; }
        let mut remaining = input.clone(); remaining.count -= 1;
        slots[0] = (remaining.count > 0).then_some(remaining);
        slots[1] = Some(ItemStack::new(&empty, 1)); *tank = next;
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
    if matches!(&*input.item, "tconstruct:seared_lantern" | "tconstruct:scorched_lantern") {
        use horizon_world::nbt::{Compound, Tag};
        if !known_item(&input.item) { return false; }
        let Some(mut portable) = lantern_tank(input) else { return false; };
        let mut next = tank.clone();
        let mut transferred = false;
        if portable.amount > 0 && direction != Direction::FillItem {
            let Some(fluid) = portable.fluid.clone() else { return false; };
            let tag = portable.tag.clone();
            let amount = portable.amount.min(next.capacity.saturating_sub(next.total()));
            if known_fluid(&fluid) && amount > 0 && next.insert_tagged(&fluid, tag.as_ref(), amount) {
                if portable.take_tagged(tag.as_ref(), amount).is_none() { return false; }
                transferred = true;
            }
        }
        if !transferred {
            if direction == Direction::EmptyItem { return false; }
            let entry = if let Some(id) = selected {
                let mut entries = next.fluids.iter().filter(|entry| entry.id == id);
                let Some(entry) = entries.next() else { return false; };
                if entries.next().is_some() { return false; }
                entry
            } else { let Some(entry) = next.fluids.first() else { return false; }; entry };
            let (fluid, tag) = (entry.id.clone(), entry.tag.clone());
            let amount = entry.amount.min(portable.capacity.saturating_sub(portable.amount));
            if !known_fluid(&fluid) || amount == 0 || !portable.insert_tagged(&fluid, tag.as_ref(), amount) || !next.take_tagged(&fluid, tag.as_ref(), amount) { return false; }
        }
        let mut custom = match input.data().and_then(|data| data.other("minecraft:custom_data")) {
            None => Compound::new(), Some(Tag::Compound(data)) => data.clone(), _ => return false,
        };
        if portable.amount == 0 { custom.remove(&"tconstruct:tank".into()); }
        else { custom.insert("tconstruct:tank".into(), portable.to_nbt()); }
        let mut output = input.clone(); output.count = 1;
        output.data_mut().set_other("minecraft:custom_data", (!custom.is_empty()).then_some(Tag::Compound(custom)));
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
            let (potion, color, custom_effects) = if is_water { ("minecraft:water".to_owned(), None, None) } else {
                let Some(tag) = entry.tag.as_ref() else { return false; };
                let Some(data) = potion_tag(tag, allow_color) else { return false; };
                data
            };
            let (fluid, tag) = (entry.id.clone(), entry.tag.clone());
            let amount = if is_water { 500 } else { 250 };
            let mut next = tank.clone();
            if !known_item("minecraft:potion") || !known_fluid(&fluid) || !next.take_tagged(&fluid, tag.as_ref(), amount) { return false; }
            let mut remaining = input.clone(); remaining.count -= 1;
            slots[0] = (remaining.count > 0).then_some(remaining);
            slots[1] = Some(potion_stack("minecraft:potion", &potion, color, custom_effects)); *tank = next;
            return true;
        }
    }
    if input.extra.is_some() { return false; }
    let mut next = tank.clone();
    let replacement = if input.item.as_ref() == "minecraft:sponge" && direction != Direction::EmptyItem {
        let Some(fluid) = selected.map(str::to_owned).or_else(|| next.fluids.first().map(|fluid| fluid.id.clone())) else { return false; };
        if !matches!(fluid.as_str(), "minecraft:water" | "minecraft:flowing_water")
            || next.fluids.iter().any(|entry| entry.id == fluid && entry.tag.is_some())
            || !known_fluid(&fluid) || !known_item("minecraft:wet_sponge") || !next.take(&fluid, 250) { return false; }
        "minecraft:wet_sponge".to_owned()
    } else if input.item.as_ref() == "minecraft:bowl" && direction != Direction::EmptyItem {
        let Some(fluid) = selected.map(str::to_owned).or_else(|| next.fluids.first().map(|fluid| fluid.id.clone())) else { return false; };
        let output = match fluid.as_str() {
            "tconstruct:meat_soup" | "tconstruct:flowing_meat_soup" => "tconstruct:meat_soup",
            "tconstruct:beetroot_soup" | "tconstruct:flowing_beetroot_soup" => "minecraft:beetroot_soup",
            "tconstruct:mushroom_stew" | "tconstruct:flowing_mushroom_stew" => "minecraft:mushroom_stew",
            "tconstruct:rabbit_stew" | "tconstruct:flowing_rabbit_stew" => "minecraft:rabbit_stew",
            _ => return false,
        };
        if next.fluids.iter().any(|entry| entry.id == fluid && entry.tag.is_some())
            || !known_fluid(&fluid) || !known_item(output) || !next.take(&fluid, 250) { return false; }
        output.to_owned()
    } else if input.item.as_ref() == "tconstruct:meat_soup" && direction != Direction::FillItem {
        if !known_fluid("tconstruct:meat_soup") || !known_item("minecraft:bowl") || !next.insert("tconstruct:meat_soup", 250) { return false; }
        "minecraft:bowl".to_owned()
    } else if input.item.as_ref() == "minecraft:glass_bottle" && direction != Direction::EmptyItem {
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
    transfer_single_with_tags(tank, slots, direction, known_item, known_fluid, None)
}
pub(super) fn transfer_single_with_tags(tank: &mut super::tconstruct_source_melter_state::Tank, slots: &mut [Option<ItemStack>; 2], direction: Direction,
    known_item: &impl Fn(&str) -> bool, known_fluid: &impl Fn(&str) -> bool,
    item_tags: Option<&std::collections::HashMap<String, std::collections::HashSet<String>>>) -> bool {
    transfer_single_with_color(tank, slots, direction, known_item, known_fluid, item_tags, false)
}
pub(super) fn transfer_single_with_color(tank: &mut super::tconstruct_source_melter_state::Tank, slots: &mut [Option<ItemStack>; 2], direction: Direction,
    known_item: &impl Fn(&str) -> bool, known_fluid: &impl Fn(&str) -> bool,
    item_tags: Option<&std::collections::HashMap<String, std::collections::HashSet<String>>>, allow_color: bool) -> bool {
    let mut staged = SmelteryTank::new(tank.capacity);
    if let Some(id) = tank.fluid.as_deref() {
        if !staged.insert_tagged(id, tank.tag.as_ref(), tank.amount) { return false; }
    }
    let mut staged_slots = slots.clone();
    if !transfer_selected_with_color(&mut staged, &mut staged_slots, direction, None, known_item, known_fluid, item_tags, allow_color) { return false; }
    if staged.fluids.len() > 1 { return false; }
    *slots = staged_slots;
    tank.amount = staged.total();
    tank.fluid = staged.fluids.first().map(|fluid| fluid.id.clone());
    tank.tag = staged.fluids.first().and_then(|fluid| fluid.tag.clone());
    true
}

#[cfg(test)]
mod tests {
    #[test]
    fn can_display_is_read_only_and_conservative() {
        use horizon_world::nbt::Tag;
        let empty = ItemStack::new("tconstruct:copper_can", 8);
        assert_eq!(can_display_fluid(&empty, &known), None);
        let mut filled = can_set_fluid(&empty, Some("minecraft:water")).unwrap();
        filled.count = 8;
        let before = filled.clone();
        assert_eq!(can_display_fluid(&filled, &known).as_deref(), Some("minecraft:water"));
        assert_eq!(filled, before);
        assert_eq!(can_display_fluid(&filled, &|_| false), None);
        filled.data_mut().set_other("minecraft:item_model", Some(Tag::String("example:can".into())));
        assert_eq!(can_display_fluid(&filled, &known), None);
        filled = before.clone(); filled.damage = 1;
        assert_eq!(can_display_fluid(&filled, &known), None);
        filled = before.clone(); filled.count = 0;
        assert_eq!(can_display_fluid(&filled, &known), None);
        filled = before.clone();
        for data in [Tag::Int(90), Tag::compound([("fluid", Tag::Int(1))]),
            Tag::compound([("fluid", Tag::String("example:water".into()))]),
            Tag::compound([("fluid", Tag::String("tconstruct:potion".into()))])] {
            filled.data_mut().set_other("minecraft:custom_data", Some(data));
            assert_eq!(can_display_fluid(&filled, &known), None);
        }
        for color in [Tag::String("red".into()), Tag::Int(-1), Tag::Int(0x1000000), Tag::Int(0x123456)] {
            let tag = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into())), ("CustomPotionColor", color)]);
            let potion = can_set_tagged(&empty, Some("tconstruct:potion"), Some(&tag)).unwrap();
            assert_eq!(can_display_fluid(&potion, &known), None);
        }
        let tag = Tag::compound([("variant", Tag::String("unknown".into()))]);
        assert_eq!(can_display_fluid(&can_set_tagged(&empty, Some("minecraft:water"), Some(&tag)).unwrap(), &known), None);
        assert_eq!(can_display_fluid(&ItemStack::new("minecraft:water_bucket", 1), &known), None);
    }
    #[test]
    fn tool_tank_transfer_preserves_item_and_rejects_mixing() {
        use super::*;
        use horizon_world::nbt::Tag;
        let mut tool = ItemStack::new("tconstruct:pickaxe", 1);
        tool.damage = 10;
        tool.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
            ("tic_upgrades", Tag::compound([("tconstruct:tank", Tag::Int(1))])),
            ("tic_tank_capacity", Tag::Int(1000)), ("unrelated", Tag::Int(7)),
        ])));
        let mut machine = SmelteryTank::new(2000);
        assert!(machine.insert("minecraft:water", 1000));
        let mut slots = [Some(tool), None];
        assert!(transfer_selected_with_color(&mut machine, &mut slots, Direction::FillItem, None, &|_| true, &|_| true, None, true));
        assert_eq!(machine.total(), 0);
        let filled = slots[1].take().unwrap();
        assert_eq!(filled.damage, 10);
        assert_eq!(filled.data().unwrap().other("minecraft:custom_data").unwrap().get("unrelated"), Some(&Tag::Int(7)));
        slots[0] = Some(filled);
        assert!(machine.insert("minecraft:lava", 1000));
        let before = machine.clone();
        let saved = slots[0].clone();
        assert!(!transfer_selected_with_color(&mut machine, &mut slots, Direction::FillItem, None, &|_| true, &|_| true, None, true));
        assert_eq!(machine, before);
        assert!(slots[0].as_ref().unwrap().same_item_same_components(saved.as_ref().unwrap()));
    }
    #[test]
    fn source_remainder_requires_no_tank_compound_or_can_fluid() {
        use super::*;
        use horizon_world::nbt::Tag;
        let mut tank = ItemStack::new("tconstruct:seared_fuel_gauge", 1);
        assert_eq!(no_crafting_remainder(&tank), Some(true));
        tank.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tconstruct:tank", Tag::compound([("amount", Tag::Int(0))]))])));
        assert_eq!(no_crafting_remainder(&tank), Some(false));
        let mut can = ItemStack::new("tconstruct:copper_can", 1);
        assert_eq!(no_crafting_remainder(&can), Some(true));
        can = can_set_fluid(&can, Some("minecraft:water")).unwrap();
        assert_eq!(no_crafting_remainder(&can), Some(false));
        assert_eq!(no_crafting_remainder(&ItemStack::new("minecraft:stone", 1)), None);
    }
    #[test]
    fn lantern_transfer_keeps_tag_and_refuses_ambiguous_variants() {
        use horizon_world::nbt::Tag;
        let tag = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into()))]);
        let other = Tag::compound([("Potion", Tag::String("minecraft:healing".into()))]);
        let mut tank = SmelteryTank::new(100);
        assert!(tank.insert_tagged("tconstruct:potion", Some(&tag), 50));
        let mut slots = [Some(ItemStack::new("tconstruct:seared_lantern", 1)), None];
        assert!(transfer_selected(&mut tank, &mut slots, Direction::FillItem, Some("tconstruct:potion"), &known, &known));
        let filled = slots[1].take().unwrap();
        assert_eq!(lantern_tank(&filled).unwrap().tag, Some(tag.clone()));
        slots = [Some(filled), None];
        assert!(transfer_selected(&mut tank, &mut slots, Direction::EmptyItem, None, &known, &known));
        assert_eq!(tank.fluids[0].tag, Some(tag.clone()));
        assert!(tank.insert_tagged("tconstruct:potion", Some(&other), 50));
        slots = [Some(ItemStack::new("tconstruct:seared_lantern", 1)), None];
        let before = (tank.clone(), slots.clone());
        assert!(!transfer_selected(&mut tank, &mut slots, Direction::FillItem, Some("tconstruct:potion"), &known, &known));
        assert_eq!((tank, slots), before);
    }
    #[test]
    fn potion_effects_round_trip_known_source_ids_and_reject_unknown() {
        use horizon_world::nbt::Tag;
        let modern = Tag::List(vec![Tag::compound([
            ("id", Tag::String("minecraft:water_breathing".into())),
            ("duration", Tag::Int(600)),
            ("amplifier", Tag::Byte(1)),
            ("show_particles", Tag::Byte(0)),
        ])]);
        let source = super::effects(&modern, true).unwrap();
        assert_eq!(super::effects(&source, false), Some(modern));
        let unknown = Tag::List(vec![Tag::compound([("id", Tag::String("example:unknown".into()))])]);
        assert!(super::effects(&unknown, true).is_none());
        let unsupported = Tag::List(vec![Tag::compound([("id", Tag::String("minecraft:speed".into())), ("amplifier", Tag::Int(1))])]);
        assert!(super::effects(&unsupported, true).is_none());
    }
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
    fn magma_bottle_round_trip_conserves_fluid_and_containers() {
        let mut tank = SmelteryTank::new(250);
        let mut slots = [Some(ItemStack::new("tconstruct:magma_bottle", 2)), None];
        assert!(transfer(&mut tank, &mut slots, Direction::Auto, &known, &known));
        assert_eq!(tank.amount("tconstruct:magma"), 250);
        assert_eq!(slots[0].as_ref().unwrap().count, 1);
        assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), "minecraft:glass_bottle");
        slots = [Some(ItemStack::new("minecraft:glass_bottle", 1)), None];
        assert!(transfer(&mut tank, &mut slots, Direction::Auto, &known, &known));
        assert_eq!(tank.total(), 0);
        assert!(slots[0].is_none());
        assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), "tconstruct:magma_bottle");
        let before = (tank.clone(), slots.clone());
        assert!(!transfer(&mut tank, &mut slots, Direction::Auto, &known, &known));
        assert_eq!((tank, slots), before);
    }
    #[test]
    fn magma_bottle_failed_transfers_leave_fluid_and_slots_unchanged() {
        let mut full = SmelteryTank::new(250);
        assert!(full.insert("tconstruct:magma", 250));
        let mut slots = [Some(ItemStack::new("tconstruct:magma_bottle", 2)), None];
        let before = (full.clone(), slots.clone());
        assert!(!transfer(&mut full, &mut slots, Direction::Auto, &known, &known));
        assert_eq!((full, slots), before);

        let mut empty = SmelteryTank::new(250);
        let mut occupied = [Some(ItemStack::new("tconstruct:magma_bottle", 1)), Some(ItemStack::new("minecraft:stone", 1))];
        let before = (empty.clone(), occupied.clone());
        assert!(!transfer(&mut empty, &mut occupied, Direction::Auto, &known, &known));
        assert_eq!((empty, occupied), before);

        let mut unknown = SmelteryTank::new(250);
        let mut input = [Some(ItemStack::new("tconstruct:magma_bottle", 1)), None];
        let before = (unknown.clone(), input.clone());
        assert!(!transfer(&mut unknown, &mut input, Direction::Auto, &known, &|_| false));
        assert_eq!((unknown, input), before);
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
    fn honey_bottle_filling_accepts_bundled_fluids() {
        for fluid in ["tconstruct:honey", "tconstruct:flowing_honey"] {
            let mut tank = SmelteryTank::new(1250);
            assert!(tank.insert("minecraft:lava", 1000));
            assert!(tank.insert(fluid, 250));
            let mut slots = [Some(ItemStack::new("minecraft:glass_bottle", 2)), None];
            assert!(transfer_selected(&mut tank, &mut slots, Direction::FillItem, Some(fluid), &known, &known));
            assert_eq!(tank.amount("minecraft:lava"), 1000);
            assert_eq!(tank.amount(fluid), 0);
            assert_eq!(slots[0].as_ref().unwrap().count, 1);
            assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), "minecraft:honey_bottle");
        }
    }
    #[test]
    fn wet_sponge_filling_accepts_bundled_water_and_stack_limit() {
        for fluid in ["minecraft:water", "minecraft:flowing_water"] {
            let mut tank = super::super::tconstruct_source_melter_state::Tank::new(250);
            assert!(tank.insert(fluid, 250));
            let mut slots = [Some(ItemStack::new("minecraft:sponge", 2)), None];
            assert!(transfer_single(&mut tank, &mut slots, Direction::Auto, &known, &known));
            assert_eq!(tank.amount, 0);
            assert_eq!(slots[0].as_ref().unwrap().count, 1);
            assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), "minecraft:wet_sponge");
        }
        let mut slots = [None, None];
        let mut held = Some(ItemStack::new("minecraft:sponge", 64));
        assert!(click(&mut slots, &mut held, 0, false, &known));
        assert_eq!(slots[0].as_ref().unwrap().count, 64);
        assert!(held.is_none());
        let mut slots = [None, None];
        let mut held = Some(ItemStack::new("minecraft:sponge", 64));
        assert!(quick_input(&mut slots, &mut held, &known));
        assert_eq!(slots[0].as_ref().unwrap().count, 64);
        assert!(held.is_none());
        for quick in [false, true] {
            let mut slots = [None, None];
            let mut held = Some(ItemStack::new("minecraft:glass_bottle", 64));
            if quick { assert!(quick_input(&mut slots, &mut held, &known)); }
            else { assert!(click(&mut slots, &mut held, 0, false, &known)); }
            assert_eq!(slots[0].as_ref().unwrap().count, 64);
            assert!(held.is_none());
        }
    }
    #[test]
    fn honey_and_sponge_filling_fail_without_mutation() {
        use horizon_world::nbt::Tag;
        let mut wrong = SmelteryTank::new(250);
        assert!(wrong.insert("minecraft:lava", 250));
        let mut sponge = [Some(ItemStack::new("minecraft:sponge", 1)), None];
        let before = (wrong.clone(), sponge.clone());
        assert!(!transfer(&mut wrong, &mut sponge, Direction::FillItem, &known, &known));
        assert_eq!((wrong, sponge), before);
        for (item, fluid, output) in [("minecraft:glass_bottle", "tconstruct:honey", "minecraft:honey_bottle"), ("minecraft:sponge", "minecraft:water", "minecraft:wet_sponge")] {
            let mut tank = SmelteryTank::new(250);
            assert!(tank.insert(fluid, 249));
            let mut slots = [Some(ItemStack::new(item, 1)), None];
            let before = (tank.clone(), slots.clone());
            assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &known, &known));
            assert_eq!((tank.clone(), slots.clone()), before);
            assert!(tank.insert(fluid, 1));
            let before = (tank.clone(), slots.clone());
            assert!(!transfer(&mut tank, &mut slots, Direction::EmptyItem, &known, &known));
            assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &|id| id != output, &known));
            assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &known, &|_| false));
            assert_eq!((tank.clone(), slots.clone()), before);
            slots[1] = Some(ItemStack::new("minecraft:stone", 1));
            let before = (tank.clone(), slots.clone());
            assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &known, &known));
            assert_eq!((tank, slots), before);
            let mut tank = SmelteryTank::new(250);
            assert!(tank.insert_tagged(fluid, Some(&Tag::compound([("example", Tag::Int(1))])), 250));
            let mut slots = [Some(ItemStack::new(item, 1)), None];
            let before = (tank.clone(), slots.clone());
            assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &known, &known));
            assert_eq!((tank, slots), before);
        }
    }
    #[test]
    fn vanilla_soup_bowl_filling_matches_bundled_fluid_ids() {
        for (source, flowing, output) in [
            ("tconstruct:beetroot_soup", "tconstruct:flowing_beetroot_soup", "minecraft:beetroot_soup"),
            ("tconstruct:mushroom_stew", "tconstruct:flowing_mushroom_stew", "minecraft:mushroom_stew"),
            ("tconstruct:rabbit_stew", "tconstruct:flowing_rabbit_stew", "minecraft:rabbit_stew"),
        ] {
            for fluid in [source, flowing] {
                let mut tank = SmelteryTank::new(1250);
                assert!(tank.insert("minecraft:lava", 1000));
                assert!(tank.insert(fluid, 250));
                let mut slots = [Some(ItemStack::new("minecraft:bowl", 2)), None];
                assert!(transfer_selected(&mut tank, &mut slots, Direction::FillItem, Some(fluid), &known, &known));
                assert_eq!(tank.amount("minecraft:lava"), 1000);
                assert_eq!(tank.amount(fluid), 0);
                assert_eq!(slots[0].as_ref().unwrap().count, 1);
                assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), output);
                assert_eq!(slots[1].as_ref().unwrap().count, 1);
            }
        }
    }
    #[test]
    fn vanilla_soup_filling_is_atomic_and_rejects_unsupported_inputs() {
        use horizon_world::nbt::Tag;
        for (fluid, output) in [("tconstruct:beetroot_soup", "minecraft:beetroot_soup"),
            ("tconstruct:mushroom_stew", "minecraft:mushroom_stew"), ("tconstruct:rabbit_stew", "minecraft:rabbit_stew")] {
            let mut tank = SmelteryTank::new(250);
            assert!(tank.insert(fluid, 249));
            let mut slots = [Some(ItemStack::new("minecraft:bowl", 1)), None];
            let before = (tank.clone(), slots.clone());
            assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &known, &known));
            assert_eq!((tank.clone(), slots.clone()), before);
            assert!(tank.insert(fluid, 1));
            let before = (tank.clone(), slots.clone());
            assert!(!transfer(&mut tank, &mut slots, Direction::EmptyItem, &known, &known));
            assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &|id| id != output, &known));
            assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &known, &|_| false));
            assert_eq!((tank.clone(), slots.clone()), before);
            slots[1] = Some(ItemStack::new("minecraft:stone", 1));
            let before = (tank.clone(), slots.clone());
            assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &known, &known));
            assert_eq!((tank, slots), before);
            let mut tagged = SmelteryTank::new(250);
            assert!(tagged.insert_tagged(fluid, Some(&Tag::compound([("example", Tag::Int(1))])), 250));
            let mut slots = [Some(ItemStack::new("minecraft:bowl", 1)), None];
            let before = (tagged.clone(), slots.clone());
            assert!(!transfer(&mut tagged, &mut slots, Direction::FillItem, &known, &known));
            assert_eq!((tagged, slots), before);
        }
    }
    #[test]
    fn vanilla_soup_filling_supports_single_tank_but_not_emptying() {
        for (fluid, output) in [("tconstruct:beetroot_soup", "minecraft:beetroot_soup"),
            ("tconstruct:mushroom_stew", "minecraft:mushroom_stew"), ("tconstruct:rabbit_stew", "minecraft:rabbit_stew")] {
            let mut tank = super::super::tconstruct_source_melter_state::Tank::new(250);
            assert!(tank.insert(fluid, 250));
            let mut slots = [Some(ItemStack::new("minecraft:bowl", 1)), None];
            assert!(transfer_single(&mut tank, &mut slots, Direction::Auto, &known, &known));
            assert_eq!(tank.amount, 0);
            assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), output);
            slots = [slots[1].take(), None];
            let before = (tank.clone(), slots.clone());
            assert!(!transfer_single(&mut tank, &mut slots, Direction::EmptyItem, &known, &known));
            assert_eq!((tank, slots), before);
        }
    }
    #[test]
    fn meat_soup_bowl_transfers_round_trip_and_preserves_other_fluid() {
        let mut tank = SmelteryTank::new(1500);
        assert!(tank.insert("minecraft:lava", 1000));
        assert!(tank.insert("tconstruct:meat_soup", 250));
        let mut slots = [Some(ItemStack::new("minecraft:bowl", 2)), None];
        assert!(transfer_selected(&mut tank, &mut slots, Direction::FillItem, Some("tconstruct:meat_soup"), &known, &known));
        assert_eq!(tank.amount("minecraft:lava"), 1000);
        assert_eq!(tank.amount("tconstruct:meat_soup"), 0);
        assert_eq!(slots[0].as_ref().unwrap().count, 1);
        assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), "tconstruct:meat_soup");
        slots = [slots[1].take(), None];
        assert!(transfer(&mut tank, &mut slots, Direction::EmptyItem, &known, &known));
        assert_eq!(tank.amount("tconstruct:meat_soup"), 250);
        assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), "minecraft:bowl");
    }
    #[test]
    fn meat_soup_fill_accepts_source_fluids_and_single_tank() {
        for fluid in ["tconstruct:meat_soup", "tconstruct:flowing_meat_soup"] {
            let mut tank = super::super::tconstruct_source_melter_state::Tank::new(250);
            assert!(tank.insert(fluid, 250));
            let mut slots = [Some(ItemStack::new("minecraft:bowl", 1)), None];
            assert!(transfer_single(&mut tank, &mut slots, Direction::Auto, &known, &known));
            assert_eq!(tank.amount, 0);
            assert!(slots[0].is_none());
            assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), "tconstruct:meat_soup");
        }
    }
    #[test]
    fn meat_soup_failures_leave_tank_and_slots_unchanged() {
        use horizon_world::nbt::Tag;
        let mut tank = SmelteryTank::new(250);
        assert!(tank.insert("tconstruct:meat_soup", 249));
        let mut slots = [Some(ItemStack::new("minecraft:bowl", 1)), None];
        let before = (tank.clone(), slots.clone());
        assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &known, &known));
        assert_eq!((tank.clone(), slots.clone()), before);
        assert!(tank.insert("tconstruct:meat_soup", 1));
        let before = (tank.clone(), slots.clone());
        assert!(!transfer(&mut tank, &mut slots, Direction::EmptyItem, &known, &known));
        assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &|id| id != "tconstruct:meat_soup", &known));
        assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &known, &|_| false));
        assert_eq!((tank.clone(), slots.clone()), before);
        slots[1] = Some(ItemStack::new("minecraft:stone", 1));
        let before = (tank.clone(), slots.clone());
        assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &known, &known));
        assert_eq!((tank, slots), before);
        let mut tagged = SmelteryTank::new(250);
        assert!(tagged.insert_tagged("tconstruct:meat_soup", Some(&Tag::compound([("example", Tag::Int(1))])), 250));
        let mut slots = [Some(ItemStack::new("minecraft:bowl", 1)), None];
        let before = (tagged.clone(), slots.clone());
        assert!(!transfer(&mut tagged, &mut slots, Direction::FillItem, &known, &known));
        assert_eq!((tagged, slots), before);
    }
    #[test]
    fn bowl_slot_accepts_sixty_four_and_soup_only_one() {
        let mut slots = [None, None];
        let mut carried = Some(ItemStack::new("minecraft:bowl", 64));
        assert!(click(&mut slots, &mut carried, 0, false, &known));
        assert_eq!(slots[0].as_ref().unwrap().count, 64);
        assert!(carried.is_none());
        let mut slots = [None, None];
        let mut carried = Some(ItemStack::new("tconstruct:meat_soup", 2));
        assert!(click(&mut slots, &mut carried, 0, false, &known));
        assert_eq!(slots[0].as_ref().unwrap().count, 1);
        assert_eq!(carried.as_ref().unwrap().count, 1);
    }
    #[test]
    fn glass_bottle_fills_with_venom_atomically() {
        let mut tank = SmelteryTank::new(1500);
        assert!(tank.insert("minecraft:lava", 1000));
        assert!(tank.insert("tconstruct:venom", 249));
        let mut slots = [Some(ItemStack::new("minecraft:glass_bottle", 2)), None];
        let before = (tank.clone(), slots.clone());
        assert!(!transfer_selected(&mut tank, &mut slots, Direction::FillItem, Some("tconstruct:venom"), &known, &known));
        assert_eq!((tank.clone(), slots.clone()), before);
        assert!(tank.insert("tconstruct:venom", 1));
        let before = (tank.clone(), slots.clone());
        assert!(!transfer_selected(&mut tank, &mut slots, Direction::EmptyItem, Some("tconstruct:venom"), &known, &known));
        assert_eq!((tank.clone(), slots.clone()), before);
        assert!(!transfer_selected(&mut tank, &mut slots, Direction::FillItem, Some("tconstruct:venom"), &|id| id != "tconstruct:venom_bottle", &known));
        assert_eq!((tank.clone(), slots.clone()), before);
        assert!(!transfer_selected(&mut tank, &mut slots, Direction::FillItem, Some("tconstruct:venom"), &known, &|_| false));
        assert_eq!((tank.clone(), slots.clone()), before);
        assert!(transfer_selected(&mut tank, &mut slots, Direction::FillItem, Some("tconstruct:venom"), &known, &known));
        assert_eq!(tank.amount("minecraft:lava"), 1000);
        assert_eq!(tank.amount("tconstruct:venom"), 0);
        assert_eq!(slots[0].as_ref().unwrap().count, 1);
        assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), "tconstruct:venom_bottle");
    }
    #[test]
    fn venom_fill_accepts_both_pinned_fluid_ids_and_single_tank() {
        for fluid in ["tconstruct:venom", "tconstruct:flowing_venom"] {
            let mut tank = super::super::tconstruct_source_melter_state::Tank::new(250);
            assert!(tank.insert(fluid, 250));
            let mut slots = [Some(ItemStack::new("minecraft:glass_bottle", 1)), None];
            assert!(transfer_single(&mut tank, &mut slots, Direction::Auto, &known, &known));
            assert_eq!(tank.amount, 0);
            assert!(slots[0].is_none());
            assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), "tconstruct:venom_bottle");
        }
    }
    #[test]
    fn venom_fill_refuses_tagged_fluid_and_blocked_output() {
        use horizon_world::nbt::Tag;
        let mut tank = SmelteryTank::new(250);
        let tag = Tag::compound([("example", Tag::Int(1))]);
        assert!(tank.insert_tagged("tconstruct:venom", Some(&tag), 250));
        let mut slots = [Some(ItemStack::new("minecraft:glass_bottle", 1)), None];
        let before = (tank.clone(), slots.clone());
        assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &known, &known));
        assert_eq!((tank, slots), before);
        let mut tank = SmelteryTank::new(250);
        assert!(tank.insert("tconstruct:venom", 250));
        let mut slots = [Some(ItemStack::new("minecraft:glass_bottle", 1)), Some(ItemStack::new("minecraft:stone", 1))];
        let before = (tank.clone(), slots.clone());
        assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &known, &known));
        assert_eq!((tank, slots), before);
    }
    #[test]
    fn venom_bottle_returns_glass_only_after_full_transfer() {
        let mut tank = SmelteryTank::new(249);
        let mut slots = [Some(ItemStack::new("tconstruct:venom_bottle", 2)), None];
        let before = (tank.clone(), slots.clone());
        assert!(!transfer(&mut tank, &mut slots, Direction::EmptyItem, &known, &known));
        assert_eq!((tank, slots.clone()), before);
        tank = SmelteryTank::new(250);
        assert!(transfer(&mut tank, &mut slots, Direction::EmptyItem, &known, &known));
        assert_eq!(tank.amount("tconstruct:venom"), 250);
        assert_eq!(slots[0].as_ref().unwrap().count, 1);
        assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), "minecraft:glass_bottle");
    }
    #[test]
    fn venom_transfer_preserves_other_fluids_and_blocked_stacks() {
        let mut tank = SmelteryTank::new(1500);
        assert!(tank.insert("minecraft:lava", 1000));
        let mut slots = [Some(ItemStack::new("tconstruct:venom_bottle", 2)), None];
        assert!(transfer(&mut tank, &mut slots, Direction::EmptyItem, &known, &known));
        assert_eq!(tank.amount("minecraft:lava"), 1000);
        assert_eq!(tank.amount("tconstruct:venom"), 250);
        assert_eq!(slots[0].as_ref().unwrap().count, 1);
        assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), "minecraft:glass_bottle");
        let before = (tank.clone(), slots.clone());
        assert!(!transfer(&mut tank, &mut slots, Direction::EmptyItem, &known, &known));
        assert_eq!((tank, slots), before);
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
    fn special_potions_enter_via_click_and_quick_move_then_transfer() {
        use std::collections::{HashMap, HashSet};
        use horizon_world::nbt::Tag;
        for (item, tag, bottle) in [
            ("minecraft:splash_potion", "forge:bottles/splash", "test:splash_bottle"),
            ("minecraft:lingering_potion", "forge:bottles/lingering", "test:lingering_bottle"),
        ] {
            let potion = ItemStack::new(item, 1).with_potion("minecraft:healing");
            assert!(accepts(&potion, &known));
            let mut slots = [None, None]; let mut carried = Some(potion.clone());
            assert!(click(&mut slots, &mut carried, 0, false, &known));
            assert!(carried.is_none());
            let tags: HashMap<String, HashSet<String>> = [(tag.to_owned(), [bottle.to_owned()].into_iter().collect())].into_iter().collect();
            let mut tank = SmelteryTank::new(250);
            assert!(transfer_selected_with_tags(&mut tank, &mut slots, Direction::Auto, None, &known, &known, Some(&tags)));
            assert_eq!(tank.total(), 250);
            assert_eq!(tank.fluids[0].tag.as_ref().unwrap().get("Potion"), Some(&Tag::String("minecraft:healing".into())));
            assert!(slots[0].is_none());
            assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), bottle);
            let mut slots = [None, None]; let mut held = Some(potion);
            assert!(quick_input(&mut slots, &mut held, &known));
            assert!(held.is_none());
            assert_eq!(slots[0].as_ref().unwrap().item.as_ref(), item);
            let mut tank = SmelteryTank::new(250);
            assert!(transfer_selected_with_tags(&mut tank, &mut slots, Direction::Auto, None, &known, &known, Some(&tags)));
            assert_eq!(tank.total(), 250);
            assert!(slots[0].is_none());
            assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), bottle);
        }
    }
    #[test]
    fn special_potion_transfer_fails_closed_without_tags_or_space() {
        use std::collections::{HashMap, HashSet};
        for (item, tag) in [("minecraft:splash_potion", "forge:bottles/splash"), ("minecraft:lingering_potion", "forge:bottles/lingering")] {
            let potion = ItemStack::new(item, 1).with_potion("minecraft:water");
            let tags: HashMap<String, HashSet<String>> = [(tag.to_owned(), ["test:bottle".to_owned()].into_iter().collect())].into_iter().collect();
            let mut slots = [Some(potion.clone()), None]; let mut tank = SmelteryTank::new(250);
            let before = (tank.clone(), slots.clone());
            assert!(!transfer_selected_with_tags(&mut tank, &mut slots, Direction::Auto, None, &known, &known, None));
            assert_eq!((tank.clone(), slots.clone()), before);
            slots[1] = Some(ItemStack::new("minecraft:stone", 1));
            let before = (tank.clone(), slots.clone());
            assert!(!transfer_selected_with_tags(&mut tank, &mut slots, Direction::Auto, None, &known, &known, Some(&tags)));
            assert_eq!((tank.clone(), slots.clone()), before);
            slots[1] = None;
            assert!(transfer_selected_with_tags(&mut tank, &mut slots, Direction::Auto, None, &known, &known, Some(&tags)));
            assert_eq!(tank.amount("minecraft:water"), 250);
            assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), "test:bottle");
        }
    }
    #[test]
    fn water_base_special_potion_discards_custom_data_like_mantle_1_11_115() {
        use std::collections::{HashMap, HashSet};
        use horizon_world::nbt::Tag;
        for (item, tag) in [("minecraft:splash_potion", "forge:bottles/splash"), ("minecraft:lingering_potion", "forge:bottles/lingering")] {
            let mut potion = ItemStack::new(item, 1).with_potion("minecraft:water");
            potion.data_mut().potion_extra.push(("custom_color".into(), Tag::Int(0x123456)));
            potion.data_mut().potion_extra.push(("custom_effects".into(), Tag::List(vec![Tag::compound([
                ("id", Tag::String("minecraft:speed".into())), ("duration", Tag::Int(100))
            ])])));
            let tags: HashMap<String, HashSet<String>> = [(tag.to_owned(), ["test:bottle".to_owned()].into_iter().collect())].into_iter().collect();
            let mut slots = [None, None]; let mut held = Some(potion);
            assert!(click_with_color(&mut slots, &mut held, 0, false, &known, true));
            let mut tank = SmelteryTank::new(250);
            assert!(transfer_selected_with_color(&mut tank, &mut slots, Direction::Auto, None, &known, &known, Some(&tags), true));
            assert_eq!(tank.amount("minecraft:water"), 250);
            assert_eq!(tank.fluids[0].tag, None);
            assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), "test:bottle");
        }
    }
    #[test]
    fn special_potion_input_rejects_unsupported_data_without_mutation() {
        use horizon_world::nbt::Tag;
        for item in ["minecraft:splash_potion", "minecraft:lingering_potion"] {
            let mut potion = ItemStack::new(item, 1).with_potion("minecraft:healing");
            potion.data_mut().potion_extra.push(("custom_color".into(), Tag::Int(0x123456)));
            assert!(!accepts(&potion, &known));
            assert!(accepts_with_color(&potion, &known, true));
            let mut slots = [None, None]; let mut held = Some(potion.clone());
            assert!(!quick_input(&mut slots, &mut held, &known));
            assert_eq!(held, Some(potion));
            assert_eq!(slots, [None, None]);
        }
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
