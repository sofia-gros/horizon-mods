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
        "minecraft:milk_bucket" => Some("minecraft:milk".into()),
        "minecraft:powder_snow_bucket" => Some("tconstruct:powdered_snow".into()),
        _ => item.strip_prefix("tconstruct:")?.strip_suffix("_bucket").filter(|id| !id.is_empty()).map(|id| format!("tconstruct:{id}")),
    }
}
pub(super) fn is_bucket(stack: &ItemStack, known_fluid: &impl Fn(&str) -> bool) -> bool {
    stack.item.as_ref() == "minecraft:bucket" || bucket_fluid(&stack.item).is_some_and(|fluid| known_fluid(&fluid))
}
fn bottle_fluid(item: &str) -> Option<&'static str> {
    match item {
        // Pinned Mantle honey_bottle_empty: 250 mB and one glass bottle.
        "minecraft:honey_bottle" => Some("tconstruct:honey"),
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
        "tconstruct:magma" | "tconstruct:flowing_magma" => Some("tconstruct:magma_bottle"),
        "tconstruct:earth_slime" | "tconstruct:flowing_earth_slime" => Some("tconstruct:earth_slime_bottle"),
        "tconstruct:sky_slime" | "tconstruct:flowing_sky_slime" => Some("tconstruct:sky_slime_bottle"),
        "tconstruct:ichor" | "tconstruct:flowing_ichor" => Some("tconstruct:ichor_slime_bottle"),
        "tconstruct:ender_slime" | "tconstruct:flowing_ender_slime" => Some("tconstruct:ender_slime_bottle"),
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
        || !(matches!(fluid.as_str(), "minecraft:water" | "minecraft:flowing_water" | "minecraft:lava" | "minecraft:flowing_lava" | "minecraft:milk" | "minecraft:flowing_milk") || fluid.starts_with("tconstruct:")) { return None; }
    Some(fluid)
}

/// Resolve known potion contents for tooltip only; icon handling remains
/// conservative because potion color may depend on effects.
pub(super) fn can_display_potion(stack: &ItemStack, known: &impl Fn(&str) -> bool) -> Option<horizon_world::nbt::Tag> {
    if stack.wire_patch().is_some() || stack.data().is_some_and(|data| data.other("minecraft:item_model").is_some()) || !known("tconstruct:potion") { return None; }
    let (fluid, tag) = can_contents(stack)?;
    if fluid.as_deref() != Some("tconstruct:potion") { return None; }
    known_potion_tag(tag?)
}

/// Only validated potion tags are exposed to the conservative icon renderer.
pub(super) fn can_display_potion_icon(stack: &ItemStack, known: &impl Fn(&str) -> bool) -> Option<horizon_world::nbt::Tag> {
    can_display_potion(stack, known)
}

/// A source empty Potion has no tag, an empty compound, or only an explicit
/// empty Potion ID. Other fields may alter color and are not guessed here.
fn empty_potion_tag(tag: Option<&horizon_world::nbt::Tag>) -> bool {
    use horizon_world::nbt::Tag;
    match tag {
        None => true,
        Some(Tag::Compound(fields)) if fields.is_empty() => true,
        Some(Tag::Compound(fields)) if fields.len() == 1 =>
            matches!(fields.get(&"Potion".into()), Some(Tag::String(id)) if matches!(id.to_string_lossy().as_str(), "empty" | "minecraft:empty")),
        _ => false,
    }
}

/// An explicit potion fluid with a recognized empty tag is source empty potion.
pub(super) fn can_display_empty_potion(stack: &ItemStack, known: &impl Fn(&str) -> bool) -> bool {
    if stack.wire_patch().is_some() || stack.data().is_some_and(|data| data.other("minecraft:item_model").is_some()) || !known("tconstruct:potion") { return false; }
    matches!(can_contents(stack), Some((Some(fluid), tag)) if fluid == "tconstruct:potion" && empty_potion_tag(tag.as_ref()))
}

/// Empty cans have source help text; malformed or opaque stacks stay hidden.
pub(super) fn can_display_empty(stack: &ItemStack) -> bool {
    if stack.wire_patch().is_some() || stack.data().is_some_and(|data| data.other("minecraft:item_model").is_some()) { return false; }
    matches!(can_contents(stack), Some((None, None)))
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
    stack.count > 0 && stack.damage == 0 && stack.extra.is_none() && (is_bucket(stack, known_fluid) || matches!(stack.item.as_ref(), "minecraft:glass_bottle" | "tconstruct:splash_bottle" | "tconstruct:lingering_bottle") || stack.item.as_ref() == "minecraft:bowl" || stack.item.as_ref() == "minecraft:sponge" || (stack.item.as_ref() == "tconstruct:meat_soup" && known_fluid("tconstruct:meat_soup")) || bottle_fluid(&stack.item).is_some_and(known_fluid))
}
// Effect IDs changed from numeric bytes to namespaced IDs after 1.20.1.
// Only the source-era vanilla IDs and fields with unchanged NBT types are
// translated; unsupported effects stay in the original stack/tank.
pub(super) const EFFECT_IDS: [&str; 33] = [
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
    let potion = potion.to_string().ok()?;
    if potion.is_empty() || matches!(potion.as_str(), "empty" | "minecraft:empty") { return None; }
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

/// Gate a new casting interaction before the lantern tank is rewritten.
/// Other supported item types use their own component-preserving transfer paths.
pub(super) fn casting_portable_rewrite_safe(stack: &ItemStack) -> bool {
    if !matches!(&*stack.item, "tconstruct:seared_lantern" | "tconstruct:scorched_lantern") { return true; }
    let Some(tank) = lantern_tank(stack) else { return false; };
    match stack.data().and_then(|data| data.other("minecraft:custom_data")).and_then(|data| data.get("tconstruct:tank")) {
        None => true,
        Some(raw) => tank.to_nbt() == *raw,
    }
}

/// Read only supported, untagged portable tank contents for tooltip display.
pub(super) fn tank_display_fluid(stack: &ItemStack, known: &impl Fn(&str) -> bool) -> Option<String> {
    if stack.wire_patch().is_some() || stack.data().is_some_and(|data| data.other("minecraft:item_model").is_some()) { return None; }
    let tank = lantern_tank(stack)?;
    let fluid = tank.fluid?;
    if tank.amount == 0 || tank.tag.is_some() || fluid == "tconstruct:potion" || !known(&fluid)
        || !(matches!(fluid.as_str(), "minecraft:water" | "minecraft:flowing_water" | "minecraft:lava" | "minecraft:flowing_lava" | "minecraft:milk" | "minecraft:flowing_milk") || fluid.starts_with("tconstruct:")) { return None; }
    Some(fluid)
}

/// Potion metadata is complete in a portable item; only known, representable
/// source tags are used to resolve its display name.
pub(super) fn tank_display_potion(stack: &ItemStack, known: &impl Fn(&str) -> bool) -> Option<horizon_world::nbt::Tag> {
    if stack.wire_patch().is_some() || stack.data().is_some_and(|data| data.other("minecraft:item_model").is_some()) || !known("tconstruct:potion") { return None; }
    let tank = lantern_tank(stack)?;
    if tank.amount == 0 || tank.fluid.as_deref() != Some("tconstruct:potion") { return None; }
    known_potion_tag(tank.tag?)
}

/// The same source empty-potion case for complete portable tank items.
pub(super) fn tank_display_empty_potion(stack: &ItemStack, known: &impl Fn(&str) -> bool) -> bool {
    if stack.wire_patch().is_some() || stack.data().is_some_and(|data| data.other("minecraft:item_model").is_some()) || !known("tconstruct:potion") { return false; }
    lantern_tank(stack).is_some_and(|tank| tank.amount > 0 && tank.fluid.as_deref() == Some("tconstruct:potion") && empty_potion_tag(tank.tag.as_ref()))
}

/// Source CompoundTag.getInt on each numeric NBT type. Float/Double use the
/// source Mth.floor order: Java saturating cast, comparison, then wrapping decrement.
pub(super) fn display_potion_color(tag: &horizon_world::nbt::Tag) -> Option<i32> {
    use horizon_world::nbt::Tag;
    match tag.get("CustomPotionColor")? {
        Tag::Byte(value) => Some(i32::from(*value)),
        Tag::Short(value) => Some(i32::from(*value)),
        Tag::Int(value) => Some(*value),
        Tag::Long(value) => Some(*value as i32),
        Tag::Float(bits) => {
            let value = f32::from_bits(*bits);
            let integer = value as i32;
            Some(if value < integer as f32 { integer.wrapping_sub(1) } else { integer })
        }
        Tag::Double(bits) => {
            let value = f64::from_bits(*bits);
            let integer = value as i32;
            Some(if value < integer as f64 { integer.wrapping_sub(1) } else { integer })
        }
        _ => None,
    }
}

fn known_potion_tag(tag: horizon_world::nbt::Tag) -> Option<horizon_world::nbt::Tag> {
    use horizon_world::nbt::Tag;
    // Source PotionFluidType prioritizes an explicit color even for EMPTY.
    // Accept only the exact known fields; unrelated metadata stays opaque.
    if let Tag::Compound(fields) = &tag {
        let color = display_potion_color(&tag).is_some();
        let potion = fields.get(&"Potion".into());
        let empty = match potion {
            None => fields.len() == 1,
            Some(Tag::String(id)) => fields.len() == 2 && matches!(id.to_string_lossy().as_str(), "empty" | "minecraft:empty"),
            _ => false,
        };
        if color && empty { return Some(tag); }
    }
    // Validate a normalized copy only; callers receive the original NBT and
    // the transfer parser remains strict about its representable item data.
    let mut normalized = tag.clone();
    if let Some(color) = display_potion_color(&tag) {
        if let Tag::Compound(fields) = &mut normalized {
            fields.insert("CustomPotionColor".into(), Tag::Int(color));
        }
    }
    let (potion, _, _) = potion_tag(&normalized, true)?;
    if potion == "empty" { return None; }
    super::tconstruct_source_smeltery_fuel::potion_fluid_name(Some(&tag))?;
    Some(tag)
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
/// Stage a shift move from player inventory into the input slot.
pub(super) fn quick_input(slots: &mut [Option<ItemStack>; 2], stack: &mut Option<ItemStack>,
    known_fluid: &impl Fn(&str) -> bool) -> bool {
    quick_input_with_color(slots, stack, known_fluid, false)
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
pub(super) fn filled_bucket(fluid: &str) -> Option<String> {
    // Only the pinned source/flowing pairs share a bucket supplier. This is
    // an output lookup; the caller still consumes the exact input fluid ID.
    let fluid = super::tconstruct_source_fluid_display::display_id(fluid);
    match fluid {
        "minecraft:water" => Some("minecraft:water_bucket".into()),
        "minecraft:lava" => Some("minecraft:lava_bucket".into()),
        "minecraft:milk" => Some("minecraft:milk_bucket".into()),
        "tconstruct:powdered_snow" => Some("minecraft:powder_snow_bucket".into()),
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
    // The two built-in empty special bottles are supported only while their
    // resolved source tags select exactly one kind. Other tag members are not
    // inferred here; missing or overlapping tag context fails closed.
    let potion_output = match input.item.as_ref() {
        "minecraft:glass_bottle" => Some("minecraft:potion"),
        "tconstruct:splash_bottle" | "tconstruct:lingering_bottle" => {
            let Some(tags) = item_tags else { return false; };
            let in_tag = |key: &str| tags.get(key).is_some_and(|values| values.contains(input.item.as_ref()));
            match (in_tag("forge:bottles/splash"), in_tag("forge:bottles/lingering")) {
                (true, false) => Some("minecraft:splash_potion"),
                (false, true) => Some("minecraft:lingering_potion"),
                _ => return false,
            }
        },
        _ => None,
    };
    // Mantle's bottle transfers fill a base potion from potion fluid or a
    // water potion from water. Unsupported metadata remains rejected.
    if let Some(output) = potion_output.filter(|_| input.extra.is_none() && direction != Direction::EmptyItem) {
        use horizon_world::nbt::Tag;
        let entry = if let Some(id) = selected {
            let mut choices = tank.fluids.iter().filter(|fluid| fluid.id == id);
            let Some(entry) = choices.next() else { return false; };
            if choices.next().is_some() { return false; }
            entry
        } else { let Some(entry) = tank.fluids.first() else { return false; }; entry };
        let is_potion = entry.id == "tconstruct:potion";
        let is_water = matches!(entry.id.as_str(), "minecraft:water" | "minecraft:flowing_water") && entry.tag.is_none();
        if is_potion || is_water {
            let (potion, color, custom_effects) = if is_water { ("minecraft:water".to_owned(), None, None) } else {
                let Some(tag) = entry.tag.as_ref() else { return false; };
                let Some(data) = potion_tag(tag, allow_color) else { return false; };
                data
            };
            let (fluid, tag) = (entry.id.clone(), entry.tag.clone());
            let amount = if is_water { 500 } else { 250 };
            let mut next = tank.clone();
            if !known_item(output) || !known_fluid(&fluid) || !next.take_tagged(&fluid, tag.as_ref(), amount) { return false; }
            let mut remaining = input.clone(); remaining.count -= 1;
            slots[0] = (remaining.count > 0).then_some(remaining);
            slots[1] = Some(potion_stack(output, &potion, color, custom_effects)); *tank = next;
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

/// Stage a direct Melter interaction without touching authoritative inventory.
/// Only the tank field is rewritten; machine and item extension data survives.
/// The caller must persist `saved` before applying either inventory result.
pub(super) struct MelterTransfer {
    pub saved: horizon_world::nbt::Tag,
    pub remaining: Option<ItemStack>,
    pub result: Option<ItemStack>,
    pub drained: bool,
}
pub(super) fn plan_melter_held(raw: &horizon_world::nbt::Tag, held: &ItemStack,
    known_item: &impl Fn(&str) -> bool, known_fluid: &impl Fn(&str) -> bool,
    item_tags: Option<&std::collections::HashMap<String, std::collections::HashSet<String>>>,
    allow_color: bool) -> Option<MelterTransfer> {
    use horizon_world::nbt::Tag;
    if held.data().is_some_and(|data| data.wire.patch().is_some())
        || !casting_portable_rewrite_safe(held) { return None; }
    let mut melter = super::tconstruct_source_melter_state::Melter::from_nbt(raw)?;
    let before = melter.tank.amount;
    let mut slots = [Some(held.clone()), None];
    if !transfer_single_with_color(&mut melter.tank, &mut slots, Direction::Auto,
        known_item, known_fluid, item_tags, allow_color) { return None; }
    let Tag::Compound(mut saved) = raw.clone() else { return None; };
    let tank = melter.tank.to_nbt_preserving(saved.get(&"tank".into())?)?;
    saved.insert("tank".into(), tank);
    Some(MelterTransfer { saved: Tag::Compound(saved), remaining: slots[0].take(),
        result: slots[1].take(), drained: melter.tank.amount < before })
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
    fn casting_lantern_rejects_unknown_and_lossy_tank_without_changing_stack() {
        use horizon_world::nbt::{JavaString, Tag};
        let mut stack = ItemStack::new("tconstruct:seared_lantern", 1);
        assert!(casting_portable_rewrite_safe(&stack));
        let invalid = Tag::compound([("fluid", Tag::String(JavaString(vec![0xd800]))), ("amount", Tag::Int(10))]);
        stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tconstruct:tank", invalid.clone())])));
        assert!(!casting_portable_rewrite_safe(&stack));
        assert_eq!(stack.data().unwrap().other("minecraft:custom_data").unwrap().get("tconstruct:tank"), Some(&invalid));
        let extra = Tag::compound([("fluid", Tag::String("minecraft:water".into())), ("amount", Tag::Int(10)), ("future", Tag::Int(1))]);
        stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tconstruct:tank", extra)])));
        assert!(!casting_portable_rewrite_safe(&stack));
    }
    #[test]
    fn numeric_potion_colors_preserve_original_item_tags() {
        use horizon_world::nbt::Tag;
        let can = ItemStack::new("tconstruct:copper_can", 1);
        for color in [Tag::Byte(-128), Tag::Byte(127), Tag::Short(-32768),
            Tag::Short(32767), Tag::Long(0x1234_5678_9abc_def0), Tag::Long(-1),
            Tag::Float((-1.25f32).to_bits()), Tag::Float(f32::NAN.to_bits()),
            Tag::Double((-1.25f64).to_bits()), Tag::Double(f64::NEG_INFINITY.to_bits())] {
            for potion in [None, Some("empty"), Some("minecraft:empty"), Some("minecraft:swiftness")] {
                let mut fields = vec![("CustomPotionColor", color.clone())];
                if let Some(id) = potion { fields.push(("Potion", Tag::String(id.into()))); }
                let tag = Tag::compound(fields);
                let filled = can_set_tagged(&can, Some("tconstruct:potion"), Some(&tag)).unwrap();
                assert_eq!(can_display_potion(&filled, &known), Some(tag.clone()));
                assert_eq!(can_display_potion_icon(&filled, &known), Some(tag.clone()));
                let mut tank = ItemStack::new("tconstruct:seared_fuel_tank", 1);
                tank.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tconstruct:tank", Tag::compound([("fluid", Tag::String("tconstruct:potion".into())), ("amount", Tag::Int(250)), ("fluid_tag", tag.clone())]))])));
                assert_eq!(tank_display_potion(&tank, &known), Some(tag.clone()));
                if potion == Some("minecraft:swiftness") { assert!(potion_tag(&tag, true).is_none()); }
            }
        }
        for color in [Tag::String("red".into())] {
            let tag = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into())), ("CustomPotionColor", color)]);
            assert_eq!(can_display_potion(&can_set_tagged(&can, Some("tconstruct:potion"), Some(&tag)).unwrap(), &known), None);
        }
    }
    // Expected values follow 1.20.1 FloatTag/DoubleTag -> Mth.floor, including
    // Java saturation, float comparison rounding, and wrapping integer subtraction.
    #[test]
    fn floating_potion_colors_match_source_floor_boundaries() {
        use horizon_world::nbt::Tag;
        for (value, expected) in [
            (Tag::Float(1.75f32.to_bits()), 1), (Tag::Float((-1.25f32).to_bits()), -2),
            (Tag::Float(f32::NAN.to_bits()), 0), (Tag::Float(f32::INFINITY.to_bits()), i32::MAX),
            (Tag::Float(f32::NEG_INFINITY.to_bits()), i32::MAX),
            (Tag::Float((-2147483904.0f32).to_bits()), i32::MAX),
            (Tag::Float(2147483648.0f32.to_bits()), i32::MAX),
            (Tag::Double(1.75f64.to_bits()), 1), (Tag::Double((-1.25f64).to_bits()), -2),
            (Tag::Double(f64::NAN.to_bits()), 0), (Tag::Double(f64::INFINITY.to_bits()), i32::MAX),
            (Tag::Double(f64::NEG_INFINITY.to_bits()), i32::MAX),
            (Tag::Double((-2147483648.5f64).to_bits()), i32::MAX),
            (Tag::Double(2147483648.0f64.to_bits()), i32::MAX),
        ] {
            let tag = Tag::compound([("CustomPotionColor", value)]);
            assert_eq!(display_potion_color(&tag), Some(expected));
        }
    }
    #[test]
    fn explicit_empty_potion_color_is_preserved_for_item_views() {
        use horizon_world::nbt::Tag;
        let empty = ItemStack::new("tconstruct:copper_can", 8);
        for tag in [Tag::compound([("CustomPotionColor", Tag::Int(0x123456))]),
            Tag::compound([("Potion", Tag::String("empty".into())), ("CustomPotionColor", Tag::Int(0))]),
            Tag::compound([("Potion", Tag::String("minecraft:empty".into())), ("CustomPotionColor", Tag::Int(-1))])] {
            let can = can_set_tagged(&empty, Some("tconstruct:potion"), Some(&tag)).unwrap();
            let before = can.clone();
            assert!(!can_display_empty_potion(&can, &known));
            assert_eq!(can_display_potion(&can, &known), Some(tag.clone()));
            assert_eq!(can_display_potion_icon(&can, &known), Some(tag.clone()));
            assert_eq!(can, before);
            let mut tank = ItemStack::new("tconstruct:seared_fuel_tank", 2);
            tank.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tconstruct:tank", Tag::compound([("fluid", Tag::String("tconstruct:potion".into())), ("amount", Tag::Int(250)), ("fluid_tag", tag.clone())]))])));
            let before = tank.clone();
            assert!(!tank_display_empty_potion(&tank, &known));
            assert_eq!(tank_display_potion(&tank, &known), Some(tag));
            assert_eq!(tank, before);
        }
        for tag in [Tag::compound([("Potion", Tag::String("example:unknown".into())), ("CustomPotionColor", Tag::Int(1))]),
            Tag::compound([("Potion", Tag::String("empty".into())), ("CustomPotionColor", Tag::String("red".into()))]),
            Tag::compound([("Potion", Tag::String("empty".into())), ("CustomPotionColor", Tag::Int(1)), ("other", Tag::Int(1))])] {
            let can = can_set_tagged(&empty, Some("tconstruct:potion"), Some(&tag)).unwrap();
            assert_eq!(can_display_potion(&can, &known), None);
            assert_eq!(can_display_potion_icon(&can, &known), None);
        }
    }
    #[test]
    fn empty_potion_item_views_require_recognized_empty_tag() {
        use horizon_world::nbt::Tag;
        let can = can_set_fluid(&ItemStack::new("tconstruct:copper_can", 8), Some("tconstruct:potion")).unwrap();
        let before = can.clone();
        assert!(can_display_empty_potion(&can, &known));
        assert!(!can_display_empty(&can));
        assert!(!can_display_empty_potion(&can, &|_| false));
        assert_eq!(can, before);
        let mut custom = can.clone(); custom.data_mut().set_other("minecraft:item_model", Some(Tag::String("example:custom".into())));
        assert!(!can_display_empty_potion(&custom, &known));
        for tag in [Tag::Compound(Default::default()), Tag::compound([("Potion", Tag::String("empty".into()))]), Tag::compound([("Potion", Tag::String("minecraft:empty".into()))])] {
            let tagged = can_set_tagged(&can, Some("tconstruct:potion"), Some(&tag)).unwrap();
            let before = tagged.clone();
            assert!(can_display_empty_potion(&tagged, &known));
            assert_eq!(tagged, before);
        }
        for tag in [Tag::compound([("Potion", Tag::String("example:unknown".into()))]), Tag::compound([("Potion", Tag::String("".into()))]), Tag::compound([("Potion", Tag::String("empty".into())), ("CustomPotionEffects", Tag::List(vec![]))]), Tag::compound([("other", Tag::Int(1))])] {
            let tagged = can_set_tagged(&can, Some("tconstruct:potion"), Some(&tag)).unwrap();
            assert!(!can_display_empty_potion(&tagged, &known));
        }
        for item in ["tconstruct:seared_fuel_tank", "tconstruct:scorched_lantern", "tconstruct:seared_fluid_cannon"] {
            let mut tank = ItemStack::new(item, 2);
            tank.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tconstruct:tank", Tag::compound([("fluid", Tag::String("tconstruct:potion".into())), ("amount", Tag::Int(1))]))])));
            let before = tank.clone();
            assert!(tank_display_empty_potion(&tank, &known));
            assert!(!tank_display_empty_potion(&tank, &|_| false));
            assert_eq!(tank, before);
            for tag in [Tag::Compound(Default::default()), Tag::compound([("Potion", Tag::String("empty".into()))]), Tag::compound([("Potion", Tag::String("minecraft:empty".into()))])] {
                tank.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tconstruct:tank", Tag::compound([("fluid", Tag::String("tconstruct:potion".into())), ("amount", Tag::Int(1)), ("fluid_tag", tag)]))])));
                assert!(tank_display_empty_potion(&tank, &known));
            }
        }
        assert!(!tank_display_empty_potion(&ItemStack::new("minecraft:stone", 1), &known));
    }
    #[test]
    fn copper_can_potion_icon_requires_explicit_color() {
        use horizon_world::nbt::Tag;
        let empty = ItemStack::new("tconstruct:copper_can", 8);
        let tag = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into())), ("CustomPotionColor", Tag::Int(0x123456))]);
        let mut filled = can_set_tagged(&empty, Some("tconstruct:potion"), Some(&tag)).unwrap();
        filled.count = 8;
        let before = filled.clone();
        assert_eq!(can_display_potion_icon(&filled, &known), Some(tag));
        assert_eq!(filled, before);
        assert_eq!(can_display_potion_icon(&filled, &|_| false), None);
        for color in [0, -1, 0x123456] {
            let tag = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into())), ("CustomPotionColor", Tag::Int(color))]);
            let one = can_set_tagged(&empty, Some("tconstruct:potion"), Some(&tag)).unwrap();
            assert_eq!(can_display_potion_icon(&one, &known), Some(tag));
        }
        filled.data_mut().set_other("minecraft:item_model", Some(Tag::String("example:custom".into())));
        assert_eq!(can_display_potion_icon(&filled, &known), None);
        let base = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into()))]);
        assert_eq!(can_display_potion_icon(&can_set_tagged(&empty, Some("tconstruct:potion"), Some(&base)).unwrap(), &known), Some(base));
        let effects = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(1))])]))]);
        assert_eq!(can_display_potion_icon(&can_set_tagged(&empty, Some("tconstruct:potion"), Some(&effects)).unwrap(), &known), Some(effects));
        for tag in [Tag::compound([("Potion", Tag::String("minecraft:swiftness".into())), ("CustomPotionColor", Tag::String("red".into()))]),
            Tag::compound([("Potion", Tag::String("example:unknown".into())), ("CustomPotionColor", Tag::Int(0x123456))])] {
            let filled = can_set_tagged(&empty, Some("tconstruct:potion"), Some(&tag)).unwrap();
            assert_eq!(can_display_potion_icon(&filled, &known), None);
        }
    }
    #[test]
    fn copper_can_potion_tooltip_requires_complete_known_metadata() {
        use horizon_world::nbt::Tag;
        let empty = ItemStack::new("tconstruct:copper_can", 8);
        let tag = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into())), ("CustomPotionColor", Tag::Int(0x123456))]);
        let filled = can_set_tagged(&empty, Some("tconstruct:potion"), Some(&tag)).unwrap();
        let before = filled.clone();
        assert_eq!(can_display_potion(&filled, &known), Some(tag));
        assert_eq!(filled, before);
        for potion in ["minecraft:long_swiftness", "minecraft:strong_swiftness"] {
            let tag = Tag::compound([("Potion", Tag::String(potion.into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(1)), ("Duration", Tag::Int(100))])]))]);
            let mut many = can_set_tagged(&empty, Some("tconstruct:potion"), Some(&tag)).unwrap();
            many.count = 8;
            let before = many.clone();
            assert_eq!(can_display_potion(&many, &known), Some(tag));
            assert_eq!(many, before);
        }
        assert_eq!(can_display_potion(&filled, &|_| false), None);
        assert_eq!(can_display_fluid(&filled, &known), None);
        let mut custom = filled.clone(); custom.data_mut().set_other("minecraft:item_model", Some(Tag::String("example:custom".into())));
        assert_eq!(can_display_potion(&custom, &known), None);
        for bad in [Tag::Compound(Default::default()), Tag::compound([("Potion", Tag::String("example:unknown".into()))]), Tag::compound([("Potion", Tag::String("minecraft:empty".into()))]), Tag::compound([("Potion", Tag::String("empty".into()))]), Tag::compound([("Potion", Tag::Int(1))])] {
            let bad = can_set_tagged(&empty, Some("tconstruct:potion"), Some(&bad)).unwrap();
            assert_eq!(can_display_potion(&bad, &known), None);
        }
        assert_eq!(can_display_potion(&can_set_fluid(&empty, Some("tconstruct:potion")).unwrap(), &known), None);
        assert_eq!(can_display_potion(&can_set_fluid(&empty, Some("minecraft:water")).unwrap(), &known), None);
    }
    #[test]
    fn portable_potion_tooltip_requires_complete_known_metadata() {
        use horizon_world::nbt::Tag;
        let mut stack = ItemStack::new("tconstruct:seared_fuel_tank", 2);
        let make = |tag: Tag| Tag::compound([("tconstruct:tank", Tag::compound([("fluid", Tag::String("tconstruct:potion".into())), ("amount", Tag::Int(250)), ("fluid_tag", tag)]))]);
        let tag = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into())), ("CustomPotionColor", Tag::Int(0x123456))]);
        stack.data_mut().set_other("minecraft:custom_data", Some(make(tag.clone())));
        let before = stack.clone();
        assert_eq!(tank_display_potion(&stack, &known), Some(tag));
        assert_eq!(stack, before);
        for potion in ["minecraft:long_swiftness", "minecraft:strong_swiftness"] {
            let tag = Tag::compound([("Potion", Tag::String(potion.into())), ("CustomPotionEffects", Tag::List(vec![Tag::compound([("Id", Tag::Byte(1)), ("Duration", Tag::Int(100))])]))]);
            stack.data_mut().set_other("minecraft:custom_data", Some(make(tag.clone())));
            assert_eq!(tank_display_potion(&stack, &known), Some(tag));
        }
        stack = before.clone();
        assert_eq!(tank_display_potion(&stack, &|_| false), None);
        let mut custom = stack.clone();
        custom.data_mut().set_other("minecraft:item_model", Some(Tag::String("example:custom".into())));
        assert_eq!(tank_display_potion(&custom, &known), None);
        for bad in [Tag::Int(1), Tag::Compound(Default::default()), Tag::compound([("Potion", Tag::String("example:unknown".into()))]), Tag::compound([("Potion", Tag::String("minecraft:empty".into()))]), Tag::compound([("Potion", Tag::String("empty".into()))]), Tag::compound([("Potion", Tag::Int(1))])] {
            stack.data_mut().set_other("minecraft:custom_data", Some(make(bad)));
            assert_eq!(tank_display_potion(&stack, &known), None);
        }
    }
    #[test]
    fn portable_tank_tooltip_view_is_read_only_and_conservative() {
        use horizon_world::nbt::Tag;
        for item in ["tconstruct:seared_fuel_tank", "tconstruct:seared_fuel_gauge", "tconstruct:scorched_fuel_tank", "tconstruct:scorched_fuel_gauge", "tconstruct:seared_ingot_tank", "tconstruct:seared_ingot_gauge", "tconstruct:scorched_ingot_tank", "tconstruct:scorched_ingot_gauge", "tconstruct:seared_lantern", "tconstruct:scorched_lantern", "tconstruct:seared_casting_tank", "tconstruct:seared_fluid_cannon"] {
            let mut stack = ItemStack::new(item, 2);
            assert_eq!(tank_display_fluid(&stack, &known), None);
            stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tconstruct:tank", Tag::compound([("fluid", Tag::String("minecraft:flowing_water".into())), ("amount", Tag::Int(1))]))])));
            let before = stack.clone();
            assert_eq!(tank_display_fluid(&stack, &known).as_deref(), Some("minecraft:flowing_water"));
            assert_eq!(stack, before);
            assert_eq!(tank_display_fluid(&stack, &|_| false), None);
            stack.data_mut().set_other("minecraft:item_model", Some(Tag::String("example:custom".into())));
            assert_eq!(tank_display_fluid(&stack, &known), None);
            let mut tagged = before.clone();
            tagged.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tconstruct:tank", Tag::compound([("fluid", Tag::String("minecraft:water".into())), ("amount", Tag::Int(1)), ("fluid_tag", Tag::compound([("variant", Tag::Int(1))]))]))])));
            assert_eq!(tank_display_fluid(&tagged, &known), None);
        }
        assert_eq!(tank_display_fluid(&ItemStack::new("minecraft:stone", 1), &known), None);
        for fluid in ["minecraft:empty", "other:fluid", "tconstruct:potion"] {
            let mut stack = ItemStack::new("tconstruct:seared_lantern", 1);
            stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tconstruct:tank", Tag::compound([("fluid", Tag::String(fluid.into())), ("amount", Tag::Int(1))]))])));
            assert_eq!(tank_display_fluid(&stack, &known), None, "{fluid}");
        }
        let mut malformed = ItemStack::new("tconstruct:seared_lantern", 1);
        malformed.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tconstruct:tank", Tag::compound([("fluid", Tag::String("minecraft:water".into())), ("amount", Tag::Int(51))]))])));
        assert_eq!(tank_display_fluid(&malformed, &known), None);
    }
    #[test]
    fn can_display_is_read_only_and_conservative() {
        use horizon_world::nbt::Tag;
        let empty = ItemStack::new("tconstruct:copper_can", 8);
        assert_eq!(can_display_fluid(&empty, &known), None);
        assert!(can_display_empty(&empty));
        let mut custom_empty = empty.clone();
        custom_empty.data_mut().set_other("minecraft:item_model", Some(Tag::String("example:can".into())));
        assert!(!can_display_empty(&custom_empty));
        assert!(!can_display_empty(&ItemStack::new("minecraft:stone", 1)));
        for fluid in ["minecraft:flowing_water", "minecraft:flowing_lava"] {
            let filled = can_set_fluid(&empty, Some(fluid)).unwrap();
            let before = filled.clone();
            assert_eq!(can_display_fluid(&filled, &known).as_deref(), Some(fluid));
            assert_eq!(filled, before);
            assert_eq!(can_display_fluid(&filled, &|_| false), None);
            let mut custom = filled.clone();
            custom.data_mut().set_other("minecraft:item_model", Some(Tag::String("example:can".into())));
            assert_eq!(can_display_fluid(&custom, &known), None);
            let tag = Tag::compound([("variant", Tag::String("unknown".into()))]);
            let tagged = can_set_tagged(&empty, Some(fluid), Some(&tag)).unwrap();
            assert_eq!(can_display_fluid(&tagged, &known), None);
        }
        let mut filled = can_set_fluid(&empty, Some("minecraft:water")).unwrap();
        filled.count = 8;
        let before = filled.clone();
        assert!(!can_display_empty(&filled));
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
    fn special_empty_bottles_fill_water_and_potion_from_resolved_tags() {
        use std::collections::{HashMap, HashSet};
        use horizon_world::nbt::Tag;
        let tags: HashMap<String, HashSet<String>> = [
            ("forge:bottles/splash".into(), ["tconstruct:splash_bottle".into()].into_iter().collect()),
            ("forge:bottles/lingering".into(), ["tconstruct:lingering_bottle".into()].into_iter().collect()),
        ].into_iter().collect();
        for (input, output) in [("tconstruct:splash_bottle", "minecraft:splash_potion"), ("tconstruct:lingering_bottle", "minecraft:lingering_potion")] {
            for fluid in ["minecraft:water", "minecraft:flowing_water"] {
                let mut tank = SmelteryTank::new(1500);
                assert!(tank.insert("minecraft:lava", 1000)); assert!(tank.insert(fluid, 500));
                let mut slots = [Some(ItemStack::new(input, 2)), None];
                assert!(transfer_selected_with_tags(&mut tank, &mut slots, Direction::FillItem, Some(fluid), &known, &known, Some(&tags)));
                assert_eq!(tank.amount("minecraft:lava"), 1000);
                assert_eq!(tank.amount(fluid), 0);
                assert_eq!(slots[0].as_ref().unwrap().count, 1);
                assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), output);
                assert_eq!(potion_data(slots[1].as_ref().unwrap(), false).unwrap().0, "minecraft:water");
            }
            let tag = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into()))]);
            let mut tank = super::super::tconstruct_source_melter_state::Tank::new(250);
            assert!(tank.insert_tagged("tconstruct:potion", Some(&tag), 250));
            let mut slots = [Some(ItemStack::new(input, 1)), None];
            assert!(transfer_single_with_tags(&mut tank, &mut slots, Direction::Auto, &known, &known, Some(&tags)));
            assert_eq!(tank.amount, 0);
            assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), output);
            assert_eq!(potion_data(slots[1].as_ref().unwrap(), false).unwrap().0, "minecraft:swiftness");
        }
    }
    #[test]
    fn special_empty_bottle_fill_fails_closed_without_losing_state() {
        use std::collections::{HashMap, HashSet};
        use horizon_world::nbt::Tag;
        let good: HashMap<String, HashSet<String>> = [("forge:bottles/splash".into(), ["tconstruct:splash_bottle".into()].into_iter().collect())].into_iter().collect();
        let overlap: HashMap<String, HashSet<String>> = [("forge:bottles/splash".into(), ["tconstruct:splash_bottle".into()].into_iter().collect()), ("forge:bottles/lingering".into(), ["tconstruct:splash_bottle".into()].into_iter().collect())].into_iter().collect();
        let empty = HashMap::new();
        let reassigned: HashMap<String, HashSet<String>> = [("forge:bottles/lingering".into(), ["tconstruct:splash_bottle".into()].into_iter().collect())].into_iter().collect();
        let mut reassigned_tank = SmelteryTank::new(500); assert!(reassigned_tank.insert("minecraft:water", 500));
        let mut reassigned_slots = [Some(ItemStack::new("tconstruct:splash_bottle", 1)), None];
        assert!(transfer_selected_with_tags(&mut reassigned_tank, &mut reassigned_slots, Direction::FillItem, None, &known, &known, Some(&reassigned)));
        assert_eq!(reassigned_slots[1].as_ref().unwrap().item.as_ref(), "minecraft:lingering_potion");
        let mut tank = SmelteryTank::new(500); assert!(tank.insert("minecraft:water", 499));
        let mut slots = [Some(ItemStack::new("tconstruct:splash_bottle", 1)), None];
        let before = (tank.clone(), slots.clone());
        assert!(!transfer_selected_with_tags(&mut tank, &mut slots, Direction::FillItem, None, &known, &known, Some(&good)));
        assert_eq!((tank.clone(), slots.clone()), before);
        assert!(tank.insert("minecraft:water", 1));
        let before = (tank.clone(), slots.clone());
        for tags in [None, Some(&empty), Some(&overlap)] {
            assert!(!transfer_selected_with_tags(&mut tank, &mut slots, Direction::FillItem, None, &known, &known, tags));
            assert_eq!((tank.clone(), slots.clone()), before);
        }
        assert!(!transfer_selected_with_tags(&mut tank, &mut slots, Direction::EmptyItem, None, &known, &known, Some(&good)));
        assert!(!transfer_selected_with_tags(&mut tank, &mut slots, Direction::FillItem, None, &|id| id != "minecraft:splash_potion", &known, Some(&good)));
        assert!(!transfer_selected_with_tags(&mut tank, &mut slots, Direction::FillItem, None, &known, &|_| false, Some(&good)));
        assert_eq!((tank.clone(), slots.clone()), before);
        slots[1] = Some(ItemStack::new("minecraft:stone", 1));
        let before = (tank.clone(), slots.clone());
        assert!(!transfer_selected_with_tags(&mut tank, &mut slots, Direction::FillItem, None, &known, &known, Some(&good)));
        assert_eq!((tank, slots), before);
        let colored = Tag::compound([("Potion", Tag::String("minecraft:healing".into())), ("CustomPotionColor", Tag::Int(0x123456))]);
        let mut tank = SmelteryTank::new(250); assert!(tank.insert_tagged("tconstruct:potion", Some(&colored), 250));
        let mut slots = [Some(ItemStack::new("tconstruct:splash_bottle", 1)), None];
        let before = (tank.clone(), slots.clone());
        assert!(!transfer_selected_with_tags(&mut tank, &mut slots, Direction::FillItem, None, &known, &known, Some(&good)));
        assert_eq!((tank.clone(), slots.clone()), before);
        assert!(transfer_selected_with_color(&mut tank, &mut slots, Direction::FillItem, None, &known, &known, Some(&good), true));
        assert_eq!(potion_data(slots[1].as_ref().unwrap(), true).unwrap().1, Some(0x123456));
    }
    #[test]
    fn special_empty_bottles_accept_full_stacks() {
        for item in ["tconstruct:splash_bottle", "tconstruct:lingering_bottle"] {
            for quick in [false, true] {
                let mut slots = [None, None]; let mut held = Some(ItemStack::new(item, 64));
                if quick { assert!(quick_input(&mut slots, &mut held, &known)); }
                else { assert!(click(&mut slots, &mut held, 0, false, &known)); }
                assert_eq!(slots[0].as_ref().unwrap().count, 64);
                assert!(held.is_none());
            }
        }
    }
    #[test]
    fn water_bottle_filling_accepts_source_and_flowing_water() {
        for fluid in ["minecraft:water", "minecraft:flowing_water"] {
            let mut tank = SmelteryTank::new(1500);
            assert!(tank.insert("minecraft:lava", 1000));
            assert!(tank.insert(fluid, 500));
            let mut slots = [Some(ItemStack::new("minecraft:glass_bottle", 2)), None];
            assert!(transfer_selected(&mut tank, &mut slots, Direction::FillItem, Some(fluid), &known, &known));
            assert_eq!(tank.amount("minecraft:lava"), 1000);
            assert_eq!(tank.amount(fluid), 0);
            assert_eq!(slots[0].as_ref().unwrap().count, 1);
            let output = slots[1].as_ref().unwrap();
            assert_eq!(output.item.as_ref(), "minecraft:potion");
            assert_eq!(potion_data(output, false).unwrap().0, "minecraft:water");
            let mut single = super::super::tconstruct_source_melter_state::Tank::new(500);
            assert!(single.insert(fluid, 500));
            let mut slots = [Some(ItemStack::new("minecraft:glass_bottle", 1)), None];
            assert!(transfer_single(&mut single, &mut slots, Direction::Auto, &known, &known));
            assert_eq!(single.amount, 0);
            assert_eq!(potion_data(slots[1].as_ref().unwrap(), false).unwrap().0, "minecraft:water");
        }
    }
    #[test]
    fn flowing_water_bottle_failures_preserve_tank_and_inventory() {
        use horizon_world::nbt::Tag;
        let mut tank = SmelteryTank::new(500);
        assert!(tank.insert("minecraft:flowing_water", 499));
        let mut slots = [Some(ItemStack::new("minecraft:glass_bottle", 1)), None];
        let before = (tank.clone(), slots.clone());
        assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &known, &known));
        assert_eq!((tank.clone(), slots.clone()), before);
        assert!(tank.insert("minecraft:flowing_water", 1));
        let before = (tank.clone(), slots.clone());
        assert!(!transfer(&mut tank, &mut slots, Direction::EmptyItem, &known, &known));
        assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &|id| id != "minecraft:potion", &known));
        assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &known, &|_| false));
        assert_eq!((tank.clone(), slots.clone()), before);
        slots[1] = Some(ItemStack::new("minecraft:stone", 1));
        let before = (tank.clone(), slots.clone());
        assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &known, &known));
        assert_eq!((tank, slots), before);
        let mut tagged = SmelteryTank::new(500);
        assert!(tagged.insert_tagged("minecraft:flowing_water", Some(&Tag::compound([("example", Tag::Int(1))])), 500));
        let mut slots = [Some(ItemStack::new("minecraft:glass_bottle", 1)), None];
        let before = (tagged.clone(), slots.clone());
        assert!(!transfer(&mut tagged, &mut slots, Direction::FillItem, &known, &known));
        assert_eq!((tagged, slots), before);
    }
    #[test]
    fn source_bottle_fills_accept_still_and_flowing_fluids() {
        for (source, flowing, output) in [
            ("tconstruct:magma", "tconstruct:flowing_magma", "tconstruct:magma_bottle"),
            ("tconstruct:earth_slime", "tconstruct:flowing_earth_slime", "tconstruct:earth_slime_bottle"),
            ("tconstruct:sky_slime", "tconstruct:flowing_sky_slime", "tconstruct:sky_slime_bottle"),
            ("tconstruct:ichor", "tconstruct:flowing_ichor", "tconstruct:ichor_slime_bottle"),
            ("tconstruct:ender_slime", "tconstruct:flowing_ender_slime", "tconstruct:ender_slime_bottle"),
        ] {
            for fluid in [source, flowing] {
                let mut tank = SmelteryTank::new(1250);
                assert!(tank.insert("minecraft:lava", 1000));
                assert!(tank.insert(fluid, 250));
                let mut slots = [Some(ItemStack::new("minecraft:glass_bottle", 2)), None];
                assert!(transfer_selected(&mut tank, &mut slots, Direction::FillItem, Some(fluid), &known, &known));
                assert_eq!(tank.amount("minecraft:lava"), 1000);
                assert_eq!(tank.amount(fluid), 0);
                assert_eq!(slots[0].as_ref().unwrap().count, 1);
                assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), output);
            }
            let mut tank = super::super::tconstruct_source_melter_state::Tank::new(250);
            assert!(tank.insert(flowing, 250));
            let mut slots = [Some(ItemStack::new("minecraft:glass_bottle", 1)), None];
            assert!(transfer_single(&mut tank, &mut slots, Direction::Auto, &known, &known));
            assert_eq!(tank.amount, 0);
            assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), output);
        }
    }
    #[test]
    fn flowing_bottle_fills_fail_without_mutation() {
        use horizon_world::nbt::Tag;
        for (fluid, output) in [("tconstruct:flowing_magma", "tconstruct:magma_bottle"),
            ("tconstruct:flowing_earth_slime", "tconstruct:earth_slime_bottle"),
            ("tconstruct:flowing_sky_slime", "tconstruct:sky_slime_bottle"),
            ("tconstruct:flowing_ichor", "tconstruct:ichor_slime_bottle"),
            ("tconstruct:flowing_ender_slime", "tconstruct:ender_slime_bottle")] {
            let mut tank = SmelteryTank::new(250);
            assert!(tank.insert(fluid, 249));
            let mut slots = [Some(ItemStack::new("minecraft:glass_bottle", 1)), None];
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
            let mut slots = [Some(ItemStack::new("minecraft:glass_bottle", 1)), None];
            let before = (tagged.clone(), slots.clone());
            assert!(!transfer(&mut tagged, &mut slots, Direction::FillItem, &known, &known));
            assert_eq!((tagged, slots), before);
        }
    }
    #[test]
    fn powder_snow_bucket_transfer_round_trips_atomically() {
        let mut tank = SmelteryTank::new(2000);
        assert!(tank.insert("minecraft:lava", 1000));
        let mut slots = [Some(ItemStack::new("minecraft:powder_snow_bucket", 1)), None];
        assert!(accepts(slots[0].as_ref().unwrap(), &known));
        assert!(transfer(&mut tank, &mut slots, Direction::EmptyItem, &known, &known));
        assert_eq!(tank.amount("minecraft:lava"), 1000);
        assert_eq!(tank.amount("tconstruct:powdered_snow"), 1000);
        assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), "minecraft:bucket");
        slots = [slots[1].take(), None];
        assert!(transfer_selected(&mut tank, &mut slots, Direction::FillItem, Some("tconstruct:powdered_snow"), &known, &known));
        assert_eq!(tank.amount("minecraft:lava"), 1000);
        assert_eq!(tank.amount("tconstruct:powdered_snow"), 0);
        assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), "minecraft:powder_snow_bucket");
        let mut single = super::super::tconstruct_source_melter_state::Tank::new(1000);
        let mut slots = [Some(ItemStack::new("minecraft:powder_snow_bucket", 1)), None];
        assert!(transfer_single(&mut single, &mut slots, Direction::Auto, &known, &known));
        assert_eq!(single.amount, 1000);
        slots = [slots[1].take(), None];
        assert!(transfer_single(&mut single, &mut slots, Direction::Auto, &known, &known));
        assert_eq!(single.amount, 0);
        assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), "minecraft:powder_snow_bucket");
    }
    #[test]
    fn powder_snow_bucket_input_slot_uses_single_item_limit() {
        for quick in [false, true] {
            let mut slots = [None, None];
            let mut held = Some(ItemStack::new("minecraft:powder_snow_bucket", 2));
            if quick { assert!(quick_input(&mut slots, &mut held, &known)); }
            else { assert!(click(&mut slots, &mut held, 0, false, &known)); }
            assert_eq!(slots[0].as_ref().unwrap().count, 1);
            assert_eq!(held.as_ref().unwrap().count, 1);
        }
    }
    #[test]
    fn powder_snow_bucket_transfer_refuses_failed_cases_without_mutation() {
        use horizon_world::nbt::Tag;
        let mut tank = SmelteryTank::new(1000); assert!(tank.insert("tconstruct:powdered_snow", 999));
        let mut slots = [Some(ItemStack::new("minecraft:bucket", 1)), None];
        let before = (tank.clone(), slots.clone());
        assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &known, &known));
        assert_eq!((tank.clone(), slots.clone()), before);
        assert!(tank.insert("tconstruct:powdered_snow", 1));
        let before = (tank.clone(), slots.clone());
        assert!(!transfer(&mut tank, &mut slots, Direction::EmptyItem, &known, &known));
        assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &|id| id != "minecraft:powder_snow_bucket", &known));
        assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &known, &|_| false));
        assert_eq!((tank.clone(), slots.clone()), before);
        slots[1] = Some(ItemStack::new("minecraft:stone", 1));
        let before = (tank.clone(), slots.clone());
        assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &known, &known));
        assert_eq!((tank, slots), before);
        let mut tagged = SmelteryTank::new(1000);
        assert!(tagged.insert_tagged("tconstruct:powdered_snow", Some(&Tag::compound([("example", Tag::Int(1))])), 1000));
        let mut slots = [Some(ItemStack::new("minecraft:bucket", 1)), None];
        let before = (tagged.clone(), slots.clone());
        assert!(!transfer(&mut tagged, &mut slots, Direction::FillItem, &known, &known));
        assert_eq!((tagged, slots), before);
        let mut tank = SmelteryTank::new(999);
        let mut slots = [Some(ItemStack::new("minecraft:powder_snow_bucket", 1)), None];
        let before = (tank.clone(), slots.clone());
        assert!(!transfer(&mut tank, &mut slots, Direction::EmptyItem, &known, &known));
        assert_eq!((tank, slots), before);
        let mut tank = SmelteryTank::new(1000);
        let mut bucket = ItemStack::new("minecraft:powder_snow_bucket", 1);
        bucket.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("example", Tag::Int(1))])));
        let mut slots = [Some(bucket), None];
        let before = (tank.clone(), slots.clone());
        assert!(!transfer(&mut tank, &mut slots, Direction::EmptyItem, &known, &known));
        assert_eq!((tank, slots), before);
    }
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
    fn milk_can_and_tank_views_require_known_untagged_contents() {
        use horizon_world::nbt::Tag;
        for fluid in ["minecraft:milk", "minecraft:flowing_milk"] {
            let empty = ItemStack::new("tconstruct:copper_can", 1);
            let can = can_set_fluid(&empty, Some(fluid)).unwrap();
            assert_eq!(can_display_fluid(&can, &known).as_deref(), Some(fluid));
            assert_eq!(can_display_fluid(&can, &|_| false), None);
            let tag = Tag::compound([("variant", Tag::Int(1))]);
            assert_eq!(can_display_fluid(&can_set_tagged(&empty, Some(fluid), Some(&tag)).unwrap(), &known), None);
            let mut tank = ItemStack::new("tconstruct:seared_lantern", 1);
            tank.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tconstruct:tank", Tag::compound([("fluid", Tag::String(fluid.into())), ("amount", Tag::Int(1))]))])));
            assert_eq!(tank_display_fluid(&tank, &known).as_deref(), Some(fluid));
            assert_eq!(tank_display_fluid(&tank, &|_| false), None);
            tank.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tconstruct:tank", Tag::compound([("fluid", Tag::String(fluid.into())), ("amount", Tag::Int(1)), ("fluid_tag", tag)]))])));
            assert_eq!(tank_display_fluid(&tank, &known), None);
        }
    }
    #[test]
    fn milk_bucket_empty_and_fill_are_atomic() {
        let mut tank = SmelteryTank::new(1000);
        let mut slots = [Some(ItemStack::new("minecraft:milk_bucket", 1)), None];
        assert!(transfer(&mut tank, &mut slots, Direction::EmptyItem, &known, &known));
        assert_eq!(tank.amount("minecraft:milk"), 1000);
        assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), "minecraft:bucket");
        let mut slots = [Some(ItemStack::new("minecraft:bucket", 1)), None];
        assert!(transfer(&mut tank, &mut slots, Direction::FillItem, &known, &known));
        assert_eq!(tank.total(), 0);
        assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), "minecraft:milk_bucket");
        let mut slots = [Some(ItemStack::new("minecraft:milk_bucket", 1)), None];
        let before = (tank.clone(), slots.clone());
        assert!(!transfer(&mut tank, &mut slots, Direction::EmptyItem, &known, &|_| false));
        assert_eq!((tank, slots), before);
    }
    #[test]
    fn flowing_bucket_filling_preserves_fluid_identity_and_failures() {
        use horizon_world::nbt::Tag;
        for (fluid, bucket) in [("minecraft:flowing_water", "minecraft:water_bucket"),
            ("minecraft:flowing_lava", "minecraft:lava_bucket"),
            ("minecraft:flowing_milk", "minecraft:milk_bucket"),
            ("tconstruct:flowing_molten_copper", "tconstruct:molten_copper_bucket")] {
            let mut tank = SmelteryTank::new(2000);
            assert!(tank.insert("minecraft:water", 500));
            assert!(tank.insert(fluid, 1000));
            let mut slots = [Some(ItemStack::new("minecraft:bucket", 2)), None];
            assert!(transfer_selected(&mut tank, &mut slots, Direction::FillItem, Some(fluid), &known, &known));
            assert_eq!(tank.amount(fluid), 0);
            assert_eq!(tank.amount("minecraft:water"), 500);
            assert_eq!(slots[0].as_ref().unwrap().count, 1);
            assert_eq!(slots[1].as_ref().unwrap().item.as_ref(), bucket);
            let mut single = super::super::tconstruct_source_melter_state::Tank::new(1000);
            assert!(single.insert(fluid, 1000));
            let mut one = [Some(ItemStack::new("minecraft:bucket", 1)), None];
            assert!(transfer_single(&mut single, &mut one, Direction::FillItem, &known, &known));
            assert_eq!(single.amount, 0);
            assert_eq!(one[1].as_ref().unwrap().item.as_ref(), bucket);
            let mut tank = SmelteryTank::new(1000); assert!(tank.insert(fluid, 999));
            let mut slots = [Some(ItemStack::new("minecraft:bucket", 1)), None];
            let before = (tank.clone(), slots.clone());
            assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &known, &known));
            assert_eq!((tank.clone(), slots.clone()), before);
            assert!(tank.insert(fluid, 1));
            let before = (tank.clone(), slots.clone());
            assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &|id| id != bucket, &known));
            assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &known, &|_| false));
            assert_eq!((tank.clone(), slots.clone()), before);
            slots[1] = Some(ItemStack::new("minecraft:stone", 1));
            let blocked = (tank.clone(), slots.clone());
            assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &known, &known));
            assert_eq!((tank, slots), blocked);
            let mut tank = SmelteryTank::new(1000);
            let tag = Tag::compound([("variant", Tag::Int(1))]);
            assert!(tank.insert_tagged(fluid, Some(&tag), 1000));
            let mut slots = [Some(ItemStack::new("minecraft:bucket", 1)), None];
            let before = (tank.clone(), slots.clone());
            assert!(!transfer(&mut tank, &mut slots, Direction::FillItem, &known, &known));
            assert_eq!((tank, slots), before);
        }
        assert_eq!(filled_bucket("tconstruct:flowing_unknown"), None);
        assert_eq!(filled_bucket("other:flowing_water"), None);
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
    #[test]
    fn melter_held_bucket_roundtrip_preserves_non_tank_metadata() {
        use horizon_world::nbt::Tag;
        let mut melter = super::super::tconstruct_source_melter_state::Melter::new();
        melter.slots[0] = Some(ItemStack::new("minecraft:iron_ore", 2));
        melter.progress = [17, 0, 0]; melter.fuel = 20;
        let Tag::Compound(mut fields) = melter.to_nbt() else { panic!() };
        fields.insert("future".into(), Tag::Long(99));
        if let Some(Tag::Compound(tank)) = fields.get_mut(&"tank".into()) { tank.insert("extension".into(), Tag::Byte(7)); }
        let raw = Tag::Compound(fields);
        let held = ItemStack::new("minecraft:water_bucket", 2);
        let plan = plan_melter_held(&raw, &held, &known, &known, None, false).unwrap();
        assert!(!plan.drained);
        assert_eq!(plan.remaining.unwrap().count, 1);
        assert_eq!(plan.result.unwrap().item.as_ref(), "minecraft:bucket");
        assert_eq!(plan.saved.get("tank").unwrap().get("amount"), Some(&Tag::Int(1000)));
        assert_eq!(plan.saved.get("tank").unwrap().get("extension"), Some(&Tag::Byte(7)));
        for key in ["slots", "progress", "phase", "fuel", "fuel_quality", "temperature", "rate", "future"] {
            assert_eq!(plan.saved.get(key), raw.get(key), "{key}");
        }
        assert_eq!(held.count, 2);
        assert_eq!(raw.get("tank").unwrap().get("amount"), Some(&Tag::Int(0)));
        let drained = plan_melter_held(&plan.saved, &ItemStack::new("minecraft:bucket", 1), &known, &known, None, false).unwrap();
        assert!(drained.drained); assert!(drained.remaining.is_none());
        assert_eq!(drained.result.unwrap().item.as_ref(), "minecraft:water_bucket");
        assert_eq!(drained.saved, raw);
    }
    #[test]
    fn melter_held_rejects_full_mismatch_unknown_and_opaque_inputs() {
        use super::super::tconstruct_source_melter_state::Melter;
        use horizon_world::nbt::Tag;
        let mut melter = Melter::new();
        assert!(melter.tank.insert("minecraft:water", 2000));
        let raw = melter.to_nbt();
        for item in ["minecraft:water_bucket", "minecraft:lava_bucket", "minecraft:stone"] {
            assert!(plan_melter_held(&raw, &ItemStack::new(item, 1), &known, &known, None, false).is_none());
        }
        let empty = Melter::new().to_nbt();
        let bucket = ItemStack::new("minecraft:water_bucket", 1);
        assert!(plan_melter_held(&empty, &bucket, &|_| false, &known, None, false).is_none());
        assert!(plan_melter_held(&empty, &bucket, &known, &|_| false, None, false).is_none());
        let mut named = bucket.clone();
        named.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("opaque", Tag::Int(1))])));
        assert!(plan_melter_held(&empty, &named, &known, &known, None, false).is_none());
        assert!(plan_melter_held(&Tag::Int(1), &bucket, &known, &known, None, false).is_none());
        assert_eq!(melter.to_nbt(), raw);
    }
    #[test]
    fn melter_held_rejects_lossy_saved_fluid_id_without_rewriting() {
        use super::super::tconstruct_source_melter_state::Melter;
        use horizon_world::nbt::{JavaString, Tag};
        let Tag::Compound(mut fields) = Melter::new().to_nbt() else { panic!() };
        fields.insert("tank".into(), Tag::compound([
            ("amount", Tag::Int(90)), ("fluid", Tag::String(JavaString(vec![0xd800]))),
        ]));
        let raw = Tag::Compound(fields);
        let held = ItemStack::new("tconstruct:copper_can", 1);
        // Even a registry accepting the lossy ID cannot authorize rewriting it.
        assert!(plan_melter_held(&raw, &held, &|_| true, &|_| true, None, false).is_none());
        assert_eq!(raw.get("tank").unwrap().get("fluid"), Some(&Tag::String(JavaString(vec![0xd800]))));
    }
    #[test]
    fn melter_held_rejects_retained_wire_and_noncanonical_lanterns() {
        use super::super::tconstruct_source_melter_state::Melter;
        use horizon_items::item_data::Wire;
        use horizon_world::nbt::{JavaString, Tag};
        let mut melter = Melter::new();
        assert!(melter.tank.insert("minecraft:water", 1000));
        let raw = melter.to_nbt();
        let mut held = ItemStack::new("tconstruct:seared_lantern", 1);
        held.data_mut().wire = Wire::new(held.clone(), std::sync::Arc::from([7u8]));
        assert!(held.wire_patch().is_some());
        assert!(plan_melter_held(&raw, &held, &|_| true, &|_| true, None, false).is_none());
        held.data_mut().set_other("minecraft:custom_name", Some(Tag::String("Named".into())));
        assert!(held.wire_patch().is_none());
        assert!(held.data().unwrap().wire.patch().is_some());
        assert!(plan_melter_held(&raw, &held, &|_| true, &|_| true, None, false).is_none());
        let mut held = ItemStack::new("tconstruct:seared_lantern", 1);
        held.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
            ("tconstruct:tank", Tag::compound([
                ("amount", Tag::Int(50)), ("fluid", Tag::String(JavaString(vec![0xd800]))),
            ])),
        ])));
        let empty = Melter::new().to_nbt();
        assert!(plan_melter_held(&empty, &held, &|_| true, &|_| true, None, false).is_none());
    }
    #[test]
    fn melter_held_keeps_tagged_fluids_separate() {
        use super::super::tconstruct_source_melter_state::Melter;
        use horizon_world::nbt::Tag;
        let mut melter = Melter::new();
        assert!(melter.tank.insert_tagged("minecraft:water", Some(&Tag::compound([("variant", Tag::Int(1))])), 1000));
        let raw = melter.to_nbt();
        for item in ["minecraft:water_bucket", "minecraft:bucket"] {
            assert!(plan_melter_held(&raw, &ItemStack::new(item, 1), &known, &known, None, false).is_none());
        }
    }

    #[test]
    fn honey_bottle_empties_one_item_and_roundtrips_exactly_250_mb() {
        for direction in [Direction::Auto, Direction::EmptyItem] {
            let mut tank = SmelteryTank::new(1000);
            let mut slots = [Some(ItemStack::new("minecraft:honey_bottle", 3)), None];
            assert!(accepts(slots[0].as_ref().unwrap(), &known));
            assert!(transfer(&mut tank, &mut slots, direction, &known, &known));
            assert_eq!(tank.amount("tconstruct:honey"), 250);
            assert_eq!(slots[0], Some(ItemStack::new("minecraft:honey_bottle", 2)));
            assert_eq!(slots[1], Some(ItemStack::new("minecraft:glass_bottle", 1)));
            slots = [slots[1].take(), None];
            assert!(transfer(&mut tank, &mut slots, Direction::FillItem, &known, &known));
            assert_eq!(tank.total(), 0);
            assert!(slots[0].is_none());
            assert_eq!(slots[1], Some(ItemStack::new("minecraft:honey_bottle", 1)));
        }
    }

    #[test]
    fn honey_bottle_input_click_and_shift_respect_stack_limit() {
        for quick in [false, true] {
            let mut slots = [None, None];
            let mut held = Some(ItemStack::new("minecraft:honey_bottle", 20));
            if quick { assert!(quick_input(&mut slots, &mut held, &known)); }
            else { assert!(click(&mut slots, &mut held, 0, false, &known)); }
            assert_eq!(slots[0], Some(ItemStack::new("minecraft:honey_bottle", 16)));
            assert_eq!(held, Some(ItemStack::new("minecraft:honey_bottle", 4)));
        }
    }

    #[test]
    fn honey_bottle_failure_paths_do_not_mutate_slots_or_tank() {
        use horizon_world::nbt::Tag;
        for case in 0..9 {
            let mut tank = SmelteryTank::new(if case == 0 { 249 } else { 1000 });
            let mut input = ItemStack::new("minecraft:honey_bottle", 2);
            if case == 5 { input.damage = 1; }
            if case == 6 { input.count = 0; }
            if case == 7 { input.data_mut().set_other("minecraft:custom_name", Some(Tag::String("Keep me".into()))); }
            if case == 8 { input.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("unknown", Tag::Int(7))]))); }
            let mut slots = [Some(input), (case == 1).then(|| ItemStack::new("minecraft:glass_bottle", 1))];
            let before = (tank.clone(), slots.clone());
            let direction = if case == 2 { Direction::FillItem } else { Direction::Auto };
            assert!(!transfer(&mut tank, &mut slots, direction,
                &|id| case != 3 || id != "minecraft:glass_bottle", &|_| case != 4), "case {case}");
            assert_eq!((tank, slots), before, "case {case}");
        }
    }

    #[test]
    fn honey_bottle_single_tank_rejects_mismatched_fluid() {
        use super::super::tconstruct_source_melter_state::Tank;
        let mut tank = Tank::new(1000);
        assert!(tank.insert("minecraft:water", 1));
        let mut slots = [Some(ItemStack::new("minecraft:honey_bottle", 1)), None];
        let before = (tank.clone(), slots.clone());
        assert!(!transfer_single(&mut tank, &mut slots, Direction::Auto, &known, &known));
        assert_eq!((tank, slots), before);
    }

    #[test]
    fn honey_bottle_melter_plan_preserves_machine_metadata_and_inputs() {
        use super::super::tconstruct_source_melter_state::Melter;
        use horizon_world::nbt::Tag;
        let Tag::Compound(mut fields) = Melter::new().to_nbt() else { panic!() };
        fields.insert("future".into(), Tag::Long(99));
        if let Some(Tag::Compound(tank)) = fields.get_mut(&"tank".into()) { tank.insert("extension".into(), Tag::Byte(7)); }
        let raw = Tag::Compound(fields);
        let held = ItemStack::new("minecraft:honey_bottle", 2);
        let before = (raw.clone(), held.clone());
        let plan = plan_melter_held(&raw, &held, &known, &known, None, false).unwrap();
        assert!(!plan.drained);
        assert_eq!(plan.remaining, Some(ItemStack::new("minecraft:honey_bottle", 1)));
        assert_eq!(plan.result, Some(ItemStack::new("minecraft:glass_bottle", 1)));
        let tank = plan.saved.get("tank").unwrap();
        assert_eq!(tank.get("amount"), Some(&Tag::Int(250)));
        assert_eq!(tank.get("fluid"), Some(&Tag::String("tconstruct:honey".into())));
        assert_eq!(tank.get("extension"), Some(&Tag::Byte(7)));
        for key in ["slots", "progress", "phase", "fuel", "fuel_quality", "temperature", "rate", "future"] {
            assert_eq!(plan.saved.get(key), raw.get(key), "{key}");
        }
        assert_eq!((raw, held), before);
    }

}
