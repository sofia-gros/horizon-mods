// Small melter state transitions. The caller supplies a source-matched recipe,
// a resolved fluid ID, and the fuel's temperature/rate; no fuel or recipe is
// invented here. Inventory stacks and the tank can be saved without losing
// components through Horizon's existing NBT item codec.
use horizon_items::{item_nbt::{item_from_nbt, item_to_nbt}, items::ItemStack};
use horizon_world::nbt::{Compound, Tag};

pub(super) const MELTER_CAPACITY: u32 = 24 * 90;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Tank { pub fluid: Option<String>, pub tag: Option<Tag>, pub amount: u32, pub capacity: u32 }
impl Tank {
    pub(super) fn new(capacity: u32) -> Self { Self { fluid: None, tag: None, amount: 0, capacity } }
    pub(super) fn insert(&mut self, fluid: &str, amount: u32) -> bool {
        self.insert_tagged(fluid, None, amount)
    }
    pub(super) fn insert_tagged(&mut self, fluid: &str, tag: Option<&Tag>, amount: u32) -> bool {
        if tag.is_some_and(|tag| !matches!(tag, Tag::Compound(_))) || amount == 0 || fluid.is_empty()
            || self.fluid.as_deref().is_some_and(|id| id != fluid || self.tag.as_ref() != tag)
            || amount > self.capacity.saturating_sub(self.amount) { return false; }
        self.fluid = Some(fluid.to_owned()); self.tag = tag.cloned(); self.amount += amount; true
    }
    pub(super) fn take(&mut self, amount: u32) -> Option<String> {
        self.take_tagged(None, amount)
    }
    pub(super) fn take_tagged(&mut self, tag: Option<&Tag>, amount: u32) -> Option<String> {
        if self.tag.as_ref() != tag || amount == 0 || amount > self.amount { return None; }
        let fluid = self.fluid.clone()?;
        self.amount -= amount;
        if self.amount == 0 { self.fluid = None; self.tag = None; }
        Some(fluid)
    }
    pub(super) fn to_nbt(&self) -> Tag {
        let mut value = Compound::new();
        if let Some(id) = &self.fluid { value.insert("fluid".into(), Tag::String(id.as_str().into())); }
        if let Some(tag) = &self.tag { value.insert("fluid_tag".into(), tag.clone()); }
        value.insert("amount".into(), Tag::Int(self.amount as i32));
        Tag::Compound(value)
    }
    /// Patch only owned fields; retain extension data when a machine mutates
    /// a tank it has already decoded. Malformed raw state fails closed.
    pub(super) fn to_nbt_preserving(&self, raw: &Tag) -> Option<Tag> {
        // The existing decoder is lossy; do not turn an invalid UTF-16 ID
        // into a different identifier while patching this tank.
        if let Some(Tag::String(id)) = raw.get("fluid") { id.to_string().ok()?; }
        Self::from_nbt(raw, self.capacity)?;
        horizon_client::pluto_nbt_storage::patch_owned_fields(raw, &self.to_nbt(), &["fluid", "fluid_tag", "amount"])
    }
    /// Preserve native fuel data; aliases are ambiguous, not disposable.
    pub(super) fn to_nbt_preserving_fuel(&self, raw: &Tag) -> Option<Tag> {
        for key in ["FluidName", "Amount", "Tag", "Tank", "Fluid"] {
            if raw.get(key).is_some() { return None; }
        }
        if Self::from_nbt(&self.to_nbt(), self.capacity).as_ref() != Some(self) { return None; }
        self.to_nbt_preserving(raw)
    }
    /// Stage a non-tagged recipe-fuel debit without modifying the raw tank.
    /// The caller may grant fuel only after this proposal succeeds, and must
    /// persist the machine and tank together before publishing either side.
    pub(super) fn drain_fuel_preserving(raw: &Tag, capacity: u32, amount: u32) -> Option<Tag> {
        let mut tank = Self::from_nbt(raw, capacity)?;
        if tank.to_nbt_preserving_fuel(raw).as_ref() != Some(raw) { return None; }
        tank.take(amount)?;
        tank.to_nbt_preserving_fuel(raw)
    }
    pub(super) fn from_nbt(tag: &Tag, capacity: u32) -> Option<Self> {
        let amount = match tag.get("amount")? { Tag::Int(n) => u32::try_from(*n).ok()?, _ => return None };
        if amount > capacity { return None; }
        let fluid = match tag.get("fluid") { Some(Tag::String(id)) => Some(id.to_string_lossy()), None => None, _ => return None };
        let metadata = match tag.get("fluid_tag") { None => None, Some(value @ Tag::Compound(_)) => Some(value.clone()), _ => return None };
        if (amount == 0) != fluid.is_none() || fluid.as_ref().is_some_and(String::is_empty) { return None; }
        if amount == 0 && metadata.is_some() { return None; }
        Some(Self { fluid, tag: metadata, amount, capacity })
    }
}

/// Accept only an unmodified material part, so consumption cannot drop unrelated data.
pub(super) fn plain_material_part(stack: &ItemStack) -> Option<String> {
    if stack.count != 1 || stack.damage != 0 || stack.wire_patch().is_some() { return None; }
    let Tag::String(material) = stack.data()?.other("minecraft:custom_data")?.get("Material")? else { return None; };
    let material = material.to_string_lossy().to_string();
    let mut expected = ItemStack::new(&stack.item, 1);
    expected.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("Material", Tag::String(material.as_str().into()))])));
    stack.same_item_same_components(&expected).then_some(material)
}

/// Permit only display-name and lore changes over the freshly assembled tool.
/// Every other component must match the pristine copy exactly.
pub(super) fn same_pristine_with_presentation(input: &ItemStack, pristine: &ItemStack) -> bool {
    if input.count != 1 || input.damage != 0 || input.wire_patch().is_some() { return false; }
    let name = input.data().and_then(|data| data.other("minecraft:custom_name"));
    if !matches!(name, None | Some(Tag::String(_)) | Some(Tag::Compound(_))) { return false; }
    let lore = input.data().and_then(|data| data.other("minecraft:lore"));
    if !match lore { None => true, Some(Tag::List(lines)) => lines.iter().all(|line| matches!(line, Tag::String(_) | Tag::Compound(_))), _ => false } { return false; }
    let Some(decoded) = horizon_items::item_nbt::item_from_nbt(&horizon_items::item_nbt::item_to_nbt(input)) else { return false; };
    if decoded.count != input.count || !decoded.same_item_same_components(input) { return false; }
    let mut expected = pristine.clone();
    for key in ["minecraft:custom_name", "minecraft:lore"] {
        if let Some(value) = input.data().and_then(|data| data.other(key)) {
            expected.data_mut().set_other(key, Some(value.clone()));
        }
    }
    input.same_item_same_components(&expected)
}

pub(super) fn presentation_preserved(input: &ItemStack, output: &ItemStack) -> bool {
    ["minecraft:custom_name", "minecraft:lore"].into_iter().all(|key|
        input.data().and_then(|data| data.other(key)) == output.data().and_then(|data| data.other(key)))
}

/// Read a native portable tank without discarding extension fields. Casting
/// persists both the input and reserved result, so require a lossless item too.
pub(super) fn portable_tank(stack: &ItemStack, capacity: u32) -> Option<Tank> {
    if stack.count != 1 || stack.damage != 0 || stack.wire_patch().is_some()
        || stack.data().is_some_and(|data| data.wire.patch().is_some()) { return None; }
    let decoded = item_from_nbt(&item_to_nbt(stack))?;
    if decoded.count != stack.count || decoded.damage != stack.damage || !decoded.same_item_same_components(stack) { return None; }
    let raw = match stack.data().and_then(|data| data.other("minecraft:custom_data")) {
        None => return Some(Tank::new(capacity)),
        Some(Tag::Compound(custom)) => match custom.get(&"tconstruct:tank".into()) {
            None => return Some(Tank::new(capacity)), Some(raw) => raw,
        },
        _ => return None,
    };
    for key in ["FluidName", "Amount", "Tag", "Tank", "Fluid"] {
        if raw.get(key).is_some() { return None; }
    }
    let tank = Tank::from_nbt(raw, capacity)?;
    (tank.to_nbt_preserving(raw).as_ref() == Some(raw)).then_some(tank)
}

/// Stage either recipe filling path against the exact current item tank.
/// Only owned tank fields change; input, custom data and components are kept.
pub(super) fn fill_portable(stack: &ItemStack, mut tank: Tank, fluid: &str, tag: Option<&Tag>, amount: u32) -> Option<ItemStack> {
    if portable_tank(stack, tank.capacity).as_ref() != Some(&tank) { return None; }
    if !tank.insert_tagged(fluid, tag, amount) { return None; }
    let mut custom = match stack.data().and_then(|data| data.other("minecraft:custom_data")) {
        None => Compound::new(), Some(Tag::Compound(data)) => data.clone(), _ => return None,
    };
    let next = match custom.get(&"tconstruct:tank".into()) {
        Some(raw) => tank.to_nbt_preserving(raw)?, None => tank.to_nbt(),
    };
    custom.insert("tconstruct:tank".into(), next);
    let mut result = stack.clone();
    result.data_mut().set_other("minecraft:custom_data", Some(Tag::Compound(custom)));
    let decoded = item_from_nbt(&item_to_nbt(&result))?;
    (decoded.count == result.count && decoded.damage == result.damage
        && decoded.same_item_same_components(&result)).then_some(result)
}

pub(super) fn fill_portable_tagged(stack: &ItemStack, tank: Tank, fluid: &str, tag: &Tag, amount: u32) -> Option<ItemStack> {
    fill_portable(stack, tank, fluid, Some(tag), amount)
}

#[derive(Clone, Debug)]
pub(super) struct Melter {
    pub slots: [Option<ItemStack>; 3],
    // -1 means fully heated but waiting for room in the output tank.
    pub progress: [i32; 3],
    pub tank: Tank,
    pub phase: u8,
    pub fuel: u32,
    // Original fuel quality is the total duration used by the GUI flame.
    pub fuel_quality: u32,
    pub temperature: u32,
    pub rate: u32,
}
impl Melter {
    pub(super) fn new() -> Self {
        Self { slots: [None, None, None], progress: [0; 3], tank: Tank::new(MELTER_CAPACITY), phase: 0, fuel: 0, fuel_quality: 0, temperature: 0, rate: 0 }
    }
    /// GuiFuelModule reads stored temperature, not remaining or queued fuel.
    /// Fuel exhaustion can leave a positive residual temperature until lookup.
    pub(super) fn solid_fuel_tooltip_temperature(&self) -> Option<u32> {
        (self.temperature > 0).then_some(self.temperature)
    }
    /// A full output tank leaves a completed slot waiting without demanding
    /// additional fuel. It still retries output when space becomes available.
    pub(super) fn can_heat_slot(&self, slot: usize, temperature: u32, required_temp: u32) -> bool {
        self.slots.get(slot).is_some_and(Option::is_some)
            && self.progress.get(slot).is_some_and(|progress| *progress != -1)
            && temperature >= required_temp
    }
    /// Advance one source heat step. A finished item is consumed only after the
    /// entire output fits, and a blocked item stays hot instead of cooling.
    pub(super) fn heat_slot(&mut self, slot: usize, required_temp: u32, required_time: u32, fluid: &str, amount: u32) -> bool {
        if slot >= self.slots.len() || self.slots[slot].is_none() || required_time == 0 { return false; }
        if self.progress[slot] != -1 && (self.temperature < required_temp || self.rate == 0) { return false; }
        if self.progress[slot] == -1 || u32::try_from(self.progress[slot]).is_ok_and(|p| p >= required_time) {
            if amount != 0 && !self.tank.insert(fluid, amount) { self.progress[slot] = -1; return false; }
            let stack = self.slots[slot].as_mut().unwrap();
            stack.count -= 1;
            if stack.count == 0 { self.slots[slot] = None; }
            self.progress[slot] = 0;
            true
        } else {
            self.progress[slot] = self.progress[slot].saturating_add(self.rate.min(i32::MAX as u32) as i32);
            false
        }
    }
    pub(super) fn cool_slot(&mut self, slot: usize) {
        if let Some(progress) = self.progress.get_mut(slot) {
            if *progress > 0 { *progress = (*progress - 5).max(0); }
        }
    }
    pub(super) fn to_nbt(&self) -> Tag {
        let slots = self.slots.iter().map(|slot| slot.as_ref().map(item_to_nbt).unwrap_or_else(|| Tag::compound(Vec::<(&str, Tag)>::new()))).collect();
        Tag::compound([("slots", Tag::List(slots)), ("progress", Tag::IntArray(self.progress.to_vec())),
            ("tank", self.tank.to_nbt()), ("phase", Tag::Int(self.phase as i32)),
            ("fuel", Tag::Long(self.fuel as i64)), ("fuel_quality", Tag::Long(self.fuel_quality as i64)), ("temperature", Tag::Long(self.temperature as i64)), ("rate", Tag::Long(self.rate as i64))])
    }
    /// Preserve unrecognized machine and tank fields while updating the
    /// state owned by the Melter tick. Inventory slot serialization remains
    /// governed by the host item codec and is not an opaque-item migration.
    pub(super) fn to_nbt_preserving(&self, raw: &Tag) -> Option<Tag> {
        Self::from_nbt(raw)?;
        let tank = self.tank.to_nbt_preserving(raw.get("tank")?)?;
        let next = self.to_nbt();
        let keys = ["slots", "progress", "phase", "fuel", "fuel_quality", "temperature", "rate"];
        let mut updates = keys.into_iter().map(|key| Some((key, Some(next.get(key)?.clone()))))
            .collect::<Option<Vec<_>>>()?;
        updates.push(("tank", Some(tank)));
        horizon_client::pluto_nbt_storage::patch_compound(raw, updates)
    }
    pub(super) fn from_nbt(tag: &Tag) -> Option<Self> {
        let Tag::List(slots) = tag.get("slots")? else { return None; };
        if slots.len() != 3 { return None; }
        let mut inventory = [None, None, None];
        for (target, value) in inventory.iter_mut().zip(slots) {
            if matches!(value, Tag::Compound(map) if map.is_empty()) { continue; }
            *target = Some(item_from_nbt(value)?);
        }
        let Tag::IntArray(progress) = tag.get("progress")? else { return None; };
        let progress: [i32; 3] = progress.clone().try_into().ok()?;
        if progress.iter().any(|p| *p < -1) { return None; }
        let phase = match tag.get("phase")? { Tag::Int(n) if (0..4).contains(n) => *n as u8, _ => return None };
        let number = |key| match tag.get(key)? { Tag::Long(n) => u32::try_from(*n).ok(), _ => None };
        Some(Self { slots: inventory, progress, tank: Tank::from_nbt(tag.get("tank")?, MELTER_CAPACITY)?, phase,
            fuel: number("fuel")?, fuel_quality: number("fuel_quality").unwrap_or(number("fuel")?), temperature: number("temperature")?, rate: number("rate")? })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn solid_fuel_tooltip_uses_stored_temperature_across_reload() {
        for fuel in [0, 1, 100] {
            for temperature in [0, 1, 800, 1000] {
                let mut state = Melter::new();
                state.fuel = fuel; state.temperature = temperature;
                let expected = (temperature > 0).then_some(temperature);
                assert_eq!(state.solid_fuel_tooltip_temperature(), expected);
                let reloaded = Melter::from_nbt(&state.to_nbt()).unwrap();
                assert_eq!(reloaded.solid_fuel_tooltip_temperature(), expected);
                assert_eq!(state.fuel, fuel);
                assert_eq!(state.temperature, temperature);
            }
        }
    }
    #[test]
    fn zero_output_consumes_after_heating_without_changing_full_tank() {
        let mut m = Melter::new(); m.slots[0] = Some(ItemStack::new("minecraft:copper_ore", 1));
        m.temperature = 500; m.rate = 1; m.progress[0] = 50;
        assert!(m.tank.insert("tconstruct:molten_copper", MELTER_CAPACITY));
        let mut m = Melter::from_nbt(&m.to_nbt()).unwrap();
        assert!(m.heat_slot(0, 500, 50, "tconstruct:molten_copper", 0));
        assert!(m.slots[0].is_none()); assert_eq!(m.tank.amount, MELTER_CAPACITY);
    }
    #[test]
    fn fuel_quality_survives_reload_and_legacy_data() {
        let mut state = Melter::new(); state.fuel = 35; state.fuel_quality = 100;
        let mut saved = state.to_nbt();
        assert_eq!(Melter::from_nbt(&saved).unwrap().fuel_quality, 100);
        if let Tag::Compound(map) = &mut saved { map.retain(|key, _| key.to_string_lossy() != "fuel_quality"); }
        assert_eq!(Melter::from_nbt(&saved).unwrap().fuel_quality, 35);
    }
    #[test]
    fn full_tank_keeps_finished_item_and_completes_after_space_opens() {
        let mut m = Melter::new(); m.slots[0] = Some(ItemStack::new("minecraft:copper_ingot", 2));
        m.temperature = 500; m.rate = 1; m.progress[0] = 50;
        assert!(m.tank.insert("tconstruct:molten_copper", MELTER_CAPACITY));
        assert!(!m.heat_slot(0, 500, 50, "tconstruct:molten_copper", 90));
        assert_eq!(m.progress[0], -1); assert_eq!(m.slots[0].as_ref().unwrap().count, 2);
        assert_eq!(m.tank.take(90).as_deref(), Some("tconstruct:molten_copper"));
        assert!(m.heat_slot(0, 500, 50, "tconstruct:molten_copper", 90));
        assert_eq!(m.slots[0].as_ref().unwrap().count, 1); assert_eq!(m.tank.amount, MELTER_CAPACITY);
    }
    #[test]
    fn full_output_does_not_request_fuel_and_finishes_after_space_is_freed() {
        let mut melter = Melter::new();
        melter.slots[0] = Some(ItemStack::new("minecraft:copper_ingot", 1));
        melter.progress[0] = -1;
        assert!(melter.tank.insert("tconstruct:molten_copper", MELTER_CAPACITY));
        assert!(!melter.can_heat_slot(0, 800, 500));
        assert_eq!(melter.tank.take(90).as_deref(), Some("tconstruct:molten_copper"));
        assert!(melter.heat_slot(0, 500, 500, "tconstruct:molten_copper", 90));
        assert!(melter.slots[0].is_none());
        assert_eq!(melter.tank.amount, MELTER_CAPACITY);
    }
    #[test]
    fn tank_rejects_mixing_and_state_round_trips() {
        let mut m = Melter::new(); m.slots[1] = Some(ItemStack::new("minecraft:iron_ingot", 3));
        m.progress[1] = 7; m.fuel = 30; m.temperature = 800; m.rate = 2; m.phase = 2;
        assert!(m.tank.insert("tconstruct:molten_iron", 90));
        assert!(!m.tank.insert("tconstruct:molten_copper", 1));
        let copy = Melter::from_nbt(&m.to_nbt()).unwrap();
        assert_eq!(copy.to_nbt(), m.to_nbt());
    }
}

/// A source faucet reserves up to one ingot and pours a nugget each tick.
/// The in-flight fluid is kept separately so save/reload does not duplicate it.
#[derive(Clone, Debug)]
pub(super) struct Faucet { pub active: bool, pub stop_after_packet: bool, pub pending: Tank, original: Option<Tag> }
impl Faucet {
    pub(super) fn new() -> Self { Self { active: false, stop_after_packet: false, pending: Tank::new(90), original: None } }
    pub(super) fn activate(&mut self, input: &mut Tank, output: &Tank) -> bool {
        if self.active { self.stop_after_packet = true; return true; }
        self.stop_after_packet = false;
        self.active = self.begin_packet(input, output);
        self.active
    }
    fn begin_packet(&mut self, input: &mut Tank, output: &Tank) -> bool {
        if self.pending.amount != 0 { return false; }
        let Some(fluid) = input.fluid.as_deref() else { return false; };
        if output.fluid.as_deref().is_some_and(|other| other != fluid || output.tag != input.tag) { return false; }
        let amount = input.amount.min(90).min(output.capacity.saturating_sub(output.amount));
        if amount == 0 { return false; }
        let tag = input.tag.clone();
        let mut next_input = input.clone();
        let mut next_pending = self.pending.clone();
        let Some(fluid) = next_input.take_tagged(tag.as_ref(), amount) else { return false; };
        if !next_pending.insert_tagged(&fluid, tag.as_ref(), amount) { return false; }
        *input = next_input; self.pending = next_pending;
        true
    }
    pub(super) fn tick(&mut self, input: &mut Tank, output: Option<&mut Tank>) {
        if !self.active { return; }
        let Some(output) = output else { self.pending = Tank::new(90); self.active = false; self.stop_after_packet = false; return; };
        if self.pending.amount > 0 {
            let Some(fluid) = self.pending.fluid.as_deref() else { self.active = false; return; };
            if output.fluid.as_deref().is_some_and(|other| other != fluid || output.tag != self.pending.tag) { return; }
            let amount = self.pending.amount.min(10).min(output.capacity.saturating_sub(output.amount));
            if amount == 0 { return; }
            let tag = self.pending.tag.clone();
            let mut next_pending = self.pending.clone();
            let mut next_output = output.clone();
            let Some(fluid) = next_pending.take_tagged(tag.as_ref(), amount) else { return; };
            if !next_output.insert_tagged(&fluid, tag.as_ref(), amount) { return; }
            self.pending = next_pending; *output = next_output;
        } else if self.stop_after_packet || !self.begin_packet(input, output) {
            self.active = false; self.stop_after_packet = false;
        }
    }
    pub(super) fn to_nbt(&self) -> Tag {
        // The source serializer's empty-record fallback is local policy;
        // the shared patch API itself rejects non-compound records.
        let empty = Tag::Compound(Compound::new());
        let raw = match &self.original { Some(value @ Tag::Compound(_)) => value, _ => &empty };
        let pending_raw = match raw.get("pending") { Some(value @ Tag::Compound(_)) => value, _ => &empty };
        let pending = horizon_client::pluto_nbt_storage::patch_owned_fields(pending_raw, &self.pending.to_nbt(), &["fluid", "fluid_tag", "amount"])
            .expect("normalized pending record and tank serializer are compounds");
        horizon_client::pluto_nbt_storage::patch_compound(raw, [
            ("active", Some(Tag::Byte(self.active as i8))),
            ("stop", Some(Tag::Byte(self.stop_after_packet as i8))), ("pending", Some(pending)),
        ]).expect("normalized faucet record is a compound")
    }
    pub(super) fn from_nbt(tag: &Tag) -> Option<Self> {
        // These are owned fields in the fixed source Faucet format, not
        // extension metadata. Do not advance native state alongside stale
        // Forge state or guess a migration for its buffered/rendered fluid.
        for alias in ["state", "drained", "render_fluid", "lastRedstone"] {
            if tag.get(alias).is_some() { return None; }
        }
        let boolean = |key| match tag.get(key)? { Tag::Byte(0) => Some(false), Tag::Byte(1) => Some(true), _ => None };
        let active = boolean("active")?; let stop_after_packet = boolean("stop")?;
        let raw_pending = tag.get("pending")?;
        for alias in ["FluidName", "Amount", "Tag"] {
            if raw_pending.get(alias).is_some() { return None; }
        }
        let pending = Tank::from_nbt(raw_pending, 90)?;
        // Every caller, including legacy routes and renderers, must see an
        // exact fluid identity. Preserve genuine pending-tank extensions.
        if pending.to_nbt_preserving(raw_pending).as_ref() != Some(raw_pending) { return None; }
        if !active && (stop_after_packet || pending.amount != 0) { return None; }
        Some(Self { active, stop_after_packet, pending, original: Some(tag.clone()) })
    }
}

/// Distinguish a recognized but unreadable casting from a removed target.
pub(super) fn is_casting_block(name: &str) -> bool {
    matches!(name, "tconstruct:seared_table" | "tconstruct:scorched_table" | "tconstruct:seared_basin" | "tconstruct:scorched_basin")
}

/// A resumed casting may only rewrite the exact saved snapshots it decoded.
pub(super) fn safe_casting_resume(casting: &Casting, casting_raw: &Tag, faucet: &Faucet, faucet_raw: &Tag) -> bool {
    casting.roundtrips(casting_raw) && faucet.to_nbt() == *faucet_raw
}

/// Pending packets win over a new source when initializing a casting.
pub(super) fn casting_faucet_offer<'a>(casting: &Casting, faucet: &'a Faucet, input: &'a Tank) -> Option<&'a Tank> {
    if casting.recipe.is_some() { return None; }
    let offered = if faucet.pending.amount > 0 { &faucet.pending }
        else if !faucet.stop_after_packet { input } else { return None; };
    (offered.amount > 0).then_some(offered)
}

#[cfg(test)]
mod faucet_tests {
    use super::*;
    // A bounded recipe stub stands in for the world recipe lookup. All tank,
    // faucet, casting and offer transitions below use the production types.
    fn prepare_copper(casting: &mut Casting, faucet: &Faucet, input: &Tank) -> bool {
        let Some(offered) = casting_faucet_offer(casting, faucet, input) else { return false; };
        if offered.fluid.as_deref() != Some("tconstruct:molten_copper") { return false; }
        casting.prepare("test:copper", 90, 1, ItemStack::new("minecraft:copper_ingot", 1), false, false)
    }
    #[test]
    fn malformed_pending_packet_rejects_resume_without_mutating_snapshots() {
        let mut raw = Faucet::new().to_nbt();
        if let Tag::Compound(fields) = &mut raw {
            fields.insert("active".into(), Tag::Byte(1));
            fields.insert("pending".into(), Tag::compound([
                ("fluid", Tag::String(horizon_world::nbt::JavaString(vec![0xd800]))),
                ("amount", Tag::Int(10)),
            ]));
        }
        let before = raw.clone();
        // Refuse at the common decoder, before either guarded or legacy
        // routes can normalize the identifier or mutate a fluid destination.
        assert!(Faucet::from_nbt(&raw).is_none());
        assert_eq!(raw, before);
    }
    #[test]
    fn faucet_rejects_mixed_source_aliases_before_activation_or_tick() {
        for alias in ["state", "drained", "render_fluid", "lastRedstone"] {
            for value in [Tag::Byte(0), Tag::compound([("Amount", Tag::Int(10))]), Tag::String("invalid".into())] {
                let Tag::Compound(mut fields) = Faucet::new().to_nbt() else { panic!() };
                fields.insert(alias.into(), value);
                let raw = Tag::Compound(fields); let before = raw.clone();
                assert!(Faucet::from_nbt(&raw).is_none(), "{alias}");
                assert_eq!(raw, before);
            }
        }
        for alias in ["FluidName", "Amount", "Tag"] {
            let mut raw = Faucet::new().to_nbt();
            if let Tag::Compound(fields) = &mut raw {
                if let Some(Tag::Compound(pending)) = fields.get_mut(&"pending".into()) {
                    pending.insert(alias.into(), Tag::Int(1));
                }
            }
            assert!(Faucet::from_nbt(&raw).is_none(), "{alias}");
        }
    }
    #[test]
    fn faucet_native_extensions_survive_packet_drain_stop_and_reload() {
        let mut raw = Faucet::new().to_nbt();
        if let Tag::Compound(fields) = &mut raw {
            fields.insert("future".into(), Tag::Long(7));
            if let Some(Tag::Compound(pending)) = fields.get_mut(&"pending".into()) {
                pending.insert("future_pending".into(), Tag::Long(8));
            }
        }
        let mut faucet = Faucet::from_nbt(&raw).unwrap(); assert_eq!(faucet.to_nbt(), raw);
        let tag = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into()))]);
        let mut input = Tank::new(100); assert!(input.insert_tagged("tconstruct:potion", Some(&tag), 25));
        let mut output = Tank::new(100);
        assert!(faucet.activate(&mut input, &output));
        assert!(faucet.activate(&mut input, &output)); // Stop after the reserved packet.
        for _ in 0..4 {
            faucet.tick(&mut input, Some(&mut output));
            let saved = faucet.to_nbt();
            assert_eq!(saved.get("future"), Some(&Tag::Long(7)));
            assert_eq!(saved.get("pending").unwrap().get("future_pending"), Some(&Tag::Long(8)));
            faucet = Faucet::from_nbt(&saved).unwrap(); assert_eq!(faucet.to_nbt(), saved);
            assert_eq!(input.amount + faucet.pending.amount + output.amount, 25);
        }
        assert!(!faucet.active); assert!(!faucet.stop_after_packet);
        assert_eq!(output.amount, 25); assert_eq!(output.tag, Some(tag));
    }
    #[test]
    fn drained_casting_restarts_for_pending_packet_and_conserves_fluid() {
        let mut source = Tank::new(180); assert!(source.insert("tconstruct:molten_copper", 90));
        let mut casting = Casting::new(); let mut faucet = Faucet::new();
        assert!(prepare_copper(&mut casting, &faucet, &source));
        assert!(faucet.activate(&mut source, &casting.tank));
        faucet.tick(&mut source, Some(&mut casting.tank));
        assert_eq!((casting.tank.amount, faucet.pending.amount), (10, 80));
        assert!(casting.tank.take(10).is_some()); casting = Casting::new();
        assert!(prepare_copper(&mut casting, &faucet, &source));
        for _ in 0..8 { faucet.tick(&mut source, Some(&mut casting.tank)); }
        assert_eq!((source.amount, faucet.pending.amount, casting.tank.amount), (0, 0, 80));
    }
    #[test]
    fn completed_casting_waits_for_output_then_resumes_pending() {
        let mut source = Tank::new(90); assert!(source.insert("tconstruct:molten_copper", 90));
        let mut casting = Casting::new(); let mut faucet = Faucet::new();
        assert!(prepare_copper(&mut casting, &faucet, &source));
        assert!(faucet.activate(&mut source, &casting.tank));
        faucet.tick(&mut source, Some(&mut casting.tank));
        assert!(casting.tank.insert("tconstruct:molten_copper", 80));
        assert!(casting.tick(true));
        assert!(!prepare_copper(&mut casting, &faucet, &source));
        faucet.tick(&mut source, Some(&mut casting.tank));
        assert_eq!(faucet.pending.amount, 80);
        casting.output = None;
        assert!(prepare_copper(&mut casting, &faucet, &source));
        faucet.tick(&mut source, Some(&mut casting.tank));
        assert_eq!((faucet.pending.amount, casting.tank.amount), (70, 10));
    }
    #[test]
    fn pending_identity_wins_and_destroyed_output_discards_pending() {
        let mut source = Tank::new(90); assert!(source.insert("tconstruct:molten_copper", 90));
        let mut casting = Casting::new(); let mut faucet = Faucet::new();
        assert!(prepare_copper(&mut casting, &faucet, &source));
        assert!(faucet.activate(&mut source, &casting.tank));
        assert!(source.insert("tconstruct:molten_iron", 90));
        casting = Casting::new();
        assert_eq!(casting_faucet_offer(&casting, &faucet, &source).unwrap().fluid.as_deref(), Some("tconstruct:molten_copper"));
        assert!(prepare_copper(&mut casting, &faucet, &source));
        // A different fluid already in the table blocks the reserved packet.
        assert!(casting.tank.insert("tconstruct:molten_iron", 10));
        faucet.tick(&mut source, Some(&mut casting.tank));
        assert_eq!((faucet.pending.amount, casting.tank.amount), (90, 10));
        faucet.tick(&mut source, None);
        assert!(!faucet.active); assert_eq!(faucet.pending.amount, 0);
        assert_eq!(source.amount, 90);
    }
    #[test]
    fn unreadable_casting_preserves_packet_while_removed_block_discards_it() {
        for name in ["tconstruct:seared_table", "tconstruct:scorched_table", "tconstruct:seared_basin", "tconstruct:scorched_basin"] {
            let mut source = Tank::new(90); assert!(source.insert("tconstruct:molten_copper", 90));
            let output = Tank::new(90); let mut faucet = Faucet::new();
            assert!(faucet.activate(&mut source, &output));
            // The runtime leaves an unreadable recognized target untouched.
            if !is_casting_block(name) { faucet.tick(&mut source, None); }
            assert!(faucet.active); assert_eq!(faucet.pending.amount, 90);
            let mut restored = Faucet::from_nbt(&faucet.to_nbt()).unwrap();
            let mut repaired = Tank::new(90);
            restored.tick(&mut source, Some(&mut repaired));
            assert_eq!((restored.pending.amount, repaired.amount), (80, 10));
            // A known non-casting block means the output handler was removed.
            if !is_casting_block("minecraft:air") { faucet.tick(&mut source, None); }
            assert!(!faucet.active); assert_eq!(faucet.pending.amount, 0);
        }
    }
    #[test]
    fn faucet_reserves_and_pours_without_overfill_or_duplicate_after_save() {
        let mut source = Tank::new(2160); assert!(source.insert("tconstruct:molten_copper", 180));
        let mut output = Tank::new(90); let mut faucet = Faucet::new();
        assert!(faucet.activate(&mut source, &output));
        assert_eq!((source.amount, faucet.pending.amount), (90, 90));
        for _ in 0..4 { faucet.tick(&mut source, Some(&mut output)); }
        assert_eq!((source.amount, faucet.pending.amount, output.amount), (90, 50, 40));
        let mut restored = Faucet::from_nbt(&faucet.to_nbt()).unwrap();
        for _ in 0..5 { restored.tick(&mut source, Some(&mut output)); }
        assert_eq!((source.amount, restored.pending.amount, output.amount), (90, 0, 90));
        restored.tick(&mut source, Some(&mut output));
        assert!(!restored.active); assert_eq!(source.amount, 90);
    }
    #[test]
    fn removed_destination_clears_pending_and_stop_state() {
        let mut source = Tank::new(2160);
        assert!(source.insert("tconstruct:molten_copper", 180));
        let output = Tank::new(90);
        let mut faucet = Faucet::new();
        assert!(faucet.activate(&mut source, &output));
        faucet.stop_after_packet = true;
        faucet.tick(&mut source, None);
        let restored = Faucet::from_nbt(&faucet.to_nbt()).unwrap();
        assert!(!restored.active);
        assert!(!restored.stop_after_packet);
        assert_eq!(restored.pending.amount, 0);
        assert_eq!(source.amount, 90);
    }
    #[test]
    fn incompatible_or_full_output_cannot_drain_source() {
        let mut source = Tank::new(2160); assert!(source.insert("tconstruct:molten_copper", 90));
        let mut output = Tank::new(90); assert!(output.insert("tconstruct:molten_iron", 90));
        assert!(!Faucet::new().activate(&mut source, &output));
        assert_eq!(source.amount, 90);
    }
}

/// Two source casting slots and their in-progress recipe. Callers select the
/// recipe against the loaded source data before filling the tank.
#[derive(Clone, Debug)]
pub(super) struct Casting {
    pub input: Option<ItemStack>, pub output: Option<ItemStack>,
    pub tank: Tank, pub timer: u32, pub recipe: Option<String>,
    pub result: Option<ItemStack>, pub cooling_time: u32,
    pub consumes_cast: bool, pub switch_slots: bool, pub redstone: bool,
}

/// Check before serialization: the NBT codec cannot represent retained wire,
/// including stale backing whose public wire_patch() is already None.
pub(super) fn casting_item_save_safe(item: &ItemStack) -> bool {
    horizon_client::pluto_stack_storage::encode_item_lossless(item).is_some()
}

/// Patch just one slot while keeping unknown machine fields byte-for-byte.
pub(super) fn patch_casting_slot(saved: &Tag, key: &str, before: &Option<ItemStack>, after: &Option<ItemStack>) -> Option<Tag> {
    if !matches!(key, "input" | "output") { return None; }
    match before {
        None if saved.get(key).is_none() => {},
        Some(item) if casting_item_save_safe(item) && saved.get(key) == Some(&item_to_nbt(item)) => {},
        _ => return None,
    }
    let Tag::Compound(mut fields) = saved.clone() else { return None; };
    match after {
        None => { fields.remove(&key.into()); },
        Some(item) => {
            if !casting_item_save_safe(item) { return None; }
            let raw = item_to_nbt(item);
            let decoded = item_from_nbt(&raw)?;
            if decoded.count != item.count || !decoded.same_item_same_components(item) { return None; }
            fields.insert(key.into(), raw);
        },
    }
    Some(Tag::Compound(fields))
}
impl Casting {
    pub(super) fn items_save_safe(&self) -> bool {
        [&self.input, &self.output, &self.result].into_iter().flatten().all(casting_item_save_safe)
    }
    /// Patch native casting fields without discarding extensions. An opaque
    /// item may remain byte-for-byte in its slot, but must not be transformed
    /// or consumed through a decoder that cannot represent it losslessly.
    pub(super) fn to_nbt_preserving(&self, raw: &Tag) -> Option<Tag> {
        if !self.items_save_safe() { return None; }
        let before = Self::from_nbt(raw)?;
        if let Some(Tag::String(id)) = raw.get("recipe") { id.to_string().ok()?; }
        let Tag::Compound(fields) = raw else { return None; };
        // These source/Forge aliases are not native extension metadata. Their
        // relationship with the owned native fields is not yet interpreted.
        for alias in ["Items", "Tank", "Fluid", "Recipe", "coolingTime", "lastRedstone"] {
            if fields.contains_key(&alias.into()) { return None; }
        }
        let raw_tank = raw.get("tank")?;
        let Tag::Compound(tank) = raw_tank else { return None; };
        if let Some(Tag::String(id)) = tank.get(&"fluid".into()) { id.to_string().ok()?; }
        for alias in ["FluidName", "Amount", "Tag"] {
            if tank.contains_key(&alias.into()) { return None; }
        }
        // Capacity may shrink to zero on completion; validate old data using
        // the old capacity, then patch only the three native tank fields.
        let tank = horizon_client::pluto_nbt_storage::patch_owned_fields(raw_tank, &self.tank.to_nbt(), &["fluid", "fluid_tag", "amount"])?;
        let Tag::Compound(next) = self.to_nbt() else { return None; };
        let mut updates = Vec::new();
        for (key, old_item, item) in [("input", &before.input, &self.input), ("output", &before.output, &self.output), ("result", &before.result, &self.result)] {
            let unchanged = match (old_item, item) {
                (Some(old), Some(new)) => old.count == new.count && old.damage == new.damage && old.same_item_same_components(new),
                (None, None) => true,
                _ => false,
            };
            if unchanged { continue; }
            if let Some(old) = old_item {
                if raw.get(key) != Some(&item_to_nbt(old)) { return None; }
            }
            if let Some(item) = item {
                let encoded = item_to_nbt(item);
                let decoded = item_from_nbt(&encoded)?;
                if item.wire_patch().is_some() || decoded.count != item.count || decoded.damage != item.damage || !decoded.same_item_same_components(item) { return None; }
            }
            updates.push((key, next.get(&key.into()).cloned()));
        }
        for key in ["recipe", "capacity", "timer", "cooling", "consume", "switch", "redstone"] {
            updates.push((key, next.get(&key.into()).cloned()));
        }
        updates.push(("tank", Some(tank)));
        horizon_client::pluto_nbt_storage::patch_compound(raw, updates)
    }
    /// The existing serializer replaces the compound. Do not introduce a new
    /// interaction when that rewrite would discard or normalize saved data.
    pub(super) fn roundtrips(&self, raw: &Tag) -> bool { self.to_nbt() == *raw }

    pub(super) fn new() -> Self {
        Self { input: None, output: None, tank: Tank::new(0), timer: 0, recipe: None, result: None,
            cooling_time: 0, consumes_cast: false, switch_slots: false, redstone: false }
    }
    // Source sided inventory: insert into input with output clear, or extract
    // only output. Reject pending work conservatively even before fluid arrives.
    pub(super) fn hopper_push_allowed(&self) -> bool {
        self.tank.amount == 0 && self.recipe.is_none() && self.input.is_none() && self.output.is_none()
    }
    pub(super) fn hopper_pull_allowed(&self) -> bool {
        self.tank.amount == 0 && self.recipe.is_none() && self.output.as_ref().is_some_and(|s| s.count > 0)
    }
    /// Stage exactly one item; the caller validates destination limits, runs
    /// callbacks, rechecks snapshots and commits both inventories together.
    pub(super) fn hopper_transfer(&self, hopper: &[Option<ItemStack>], index: usize, push: bool) -> Option<(Vec<Option<ItemStack>>, Self)> {
        let mut next = self.clone();
        let mut slots = hopper.to_vec();
        let target = slots.get_mut(index)?;
        if push {
            if !self.hopper_push_allowed() { return None; }
            let source = target.as_mut().filter(|s| s.count > 0 && s.wire_patch().is_none())?;
            let mut one = source.clone(); one.count = 1;
            source.count -= 1;
            if source.count == 0 { *target = None; }
            next.input = Some(one);
        } else {
            if !self.hopper_pull_allowed() { return None; }
            let source = next.output.as_mut().filter(|s| s.wire_patch().is_none())?;
            let mut one = source.clone(); one.count = 1;
            if let Some(dest) = target.as_mut() {
                if dest.count == 0 || dest.wire_patch().is_some() || !dest.same_item_same_components(&one) { return None; }
                dest.count = dest.count.checked_add(1)?;
            } else { *target = Some(one); }
            source.count -= 1;
            if source.count == 0 { next.output = None; }
        }
        Some((slots, next))
    }
    /// Recover an oversized external insertion without cancelling a pour.
    /// Leave one source-valid cast in place and return the excess unchanged.
    pub(super) fn recover_excess_input(&mut self) -> Option<ItemStack> {
        if self.recipe.is_none() { return None; }
        let input = self.input.as_mut()?;
        if input.count <= 1 { return None; }
        let mut excess = input.clone();
        excess.count -= 1;
        input.count = 1;
        Some(excess)
    }
    /// Begin only after the caller has matched the cast, fluid and recipe.
    pub(super) fn prepare(&mut self, recipe: &str, amount: u32, cooling_time: u32, result: ItemStack, consumes_cast: bool, switch_slots: bool) -> bool {
        if self.recipe.is_some() || self.tank.amount != 0 || self.output.is_some() || recipe.is_empty()
            || self.input.as_ref().is_some_and(|stack| stack.count != 1)
            || amount == 0 || cooling_time == 0 || result.count == 0 { return false; }
        self.tank = Tank::new(amount); self.timer = 0; self.recipe = Some(recipe.to_owned()); self.result = Some(result);
        self.cooling_time = cooling_time; self.consumes_cast = consumes_cast; self.switch_slots = switch_slots;
        true
    }
    /// Cooling starts only after the precise amount is present. Revalidate the
    /// selected recipe against the current cast/fluid before consuming either.
    pub(super) fn tick(&mut self, valid: bool) -> bool {
        if self.recipe.is_none() || self.result.is_none() || self.tank.amount != self.tank.capacity || self.tank.amount == 0 { return false; }
        // Another inventory path may have filled the output while pouring.
        if !valid || self.output.is_some() || self.input.as_ref().is_some_and(|stack| stack.count != 1) { self.timer = 0; return false; }
        self.timer = self.timer.saturating_add(1);
        if self.timer < self.cooling_time { return false; }
        let Some(result) = self.result.take() else { return false; };
        if self.switch_slots != self.redstone {
            self.output = if self.consumes_cast { None } else { self.input.take() };
            self.input = Some(result);
        } else {
            if self.consumes_cast { self.input = None; }
            self.output = Some(result);
        }
        self.tank = Tank::new(0); self.timer = 0; self.recipe = None; self.cooling_time = 0;
        true
    }
    pub(super) fn to_nbt(&self) -> Tag {
        let mut data = Compound::new();
        for (name, stack) in [("input", &self.input), ("output", &self.output), ("result", &self.result)] {
            if let Some(stack) = stack { data.insert(name.into(), item_to_nbt(stack)); }
        }
        if let Some(recipe) = &self.recipe { data.insert("recipe".into(), Tag::String(recipe.as_str().into())); }
        data.insert("tank".into(), self.tank.to_nbt()); data.insert("capacity".into(), Tag::Long(self.tank.capacity as i64));
        data.insert("timer".into(), Tag::Long(self.timer as i64)); data.insert("cooling".into(), Tag::Long(self.cooling_time as i64));
        data.insert("consume".into(), Tag::Byte(self.consumes_cast as i8)); data.insert("switch".into(), Tag::Byte(self.switch_slots as i8));
        data.insert("redstone".into(), Tag::Byte(self.redstone as i8)); Tag::Compound(data)
    }
    pub(super) fn from_nbt(tag: &Tag) -> Option<Self> {
        let item = |key| match tag.get(key) { Some(value) => Some(Some(item_from_nbt(value)?)), None => Some(None) };
        let number = |key| match tag.get(key)? { Tag::Long(n) => u32::try_from(*n).ok(), _ => None };
        let boolean = |key| match tag.get(key)? { Tag::Byte(0) => Some(false), Tag::Byte(1) => Some(true), _ => None };
        let recipe = match tag.get("recipe") { Some(Tag::String(id)) => Some(id.to_string_lossy()), None => None, _ => return None };
        let result = item("result")?; let cooling_time = number("cooling")?;
        let tank = Tank::from_nbt(tag.get("tank")?, number("capacity")?)?;
        if recipe.is_some() != result.is_some() || recipe.is_some() != (cooling_time > 0) || recipe.is_some() != (tank.capacity > 0) { return None; }
        Some(Self { input: item("input")?, output: item("output")?, tank, timer: number("timer")?, recipe, result, cooling_time,
            consumes_cast: boolean("consume")?, switch_slots: boolean("switch")?, redstone: boolean("redstone")? })
    }
}

#[cfg(test)]
mod casting_tests {
    use super::*;
    #[test]
    fn casting_roundtrip_rejects_unknown_and_lossy_saved_data() {
        let casting = Casting::new(); let raw = casting.to_nbt();
        assert!(casting.roundtrips(&raw));
        let mut future = raw.clone();
        if let Tag::Compound(fields) = &mut future { fields.insert("future".into(), Tag::Int(9)); }
        assert!(!Casting::from_nbt(&future).unwrap().roundtrips(&future));
        let mut invalid = raw;
        if let Tag::Compound(fields) = &mut invalid {
            fields.insert("recipe".into(), Tag::String(horizon_world::nbt::JavaString(vec![0xd800])));
            fields.insert("result".into(), item_to_nbt(&ItemStack::new("minecraft:stone", 1)));
            fields.insert("cooling".into(), Tag::Long(1)); fields.insert("capacity".into(), Tag::Long(90));
        }
        assert!(!Casting::from_nbt(&invalid).unwrap().roundtrips(&invalid));
    }
    #[test]
    fn slimeskull_result_survives_partial_fill_reload_and_revalidation() {
        let mut c = Casting::new(); c.input = Some(ItemStack::new("minecraft:skeleton_skull", 1));
        let mut result = ItemStack::new("tconstruct:slime_helmet", 1);
        result.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_materials", Tag::List(vec![
            Tag::String("tconstruct:bone".into()), Tag::String("tconstruct:earthslime".into())
        ]))])));
        assert!(c.prepare("test:skull", 1250, 2, result, true, false));
        assert!(c.tank.insert("tconstruct:earth_slime", 1249));
        assert!(!c.tick(true)); assert!(c.input.is_some());
        assert!(c.tank.insert("tconstruct:earth_slime", 1));
        let mut c = Casting::from_nbt(&c.to_nbt()).unwrap();
        assert!(!c.tick(false)); assert_eq!(c.tank.amount, 1250); assert!(c.input.is_some());
        assert!(!c.tick(true)); assert!(c.tick(true));
        assert!(c.input.is_none()); assert_eq!(c.tank.amount, 0);
        let materials = c.output.as_ref().unwrap().data().unwrap().other("minecraft:custom_data").unwrap().get("tic_materials").unwrap();
        assert_eq!(materials, &Tag::List(vec![Tag::String("tconstruct:bone".into()), Tag::String("tconstruct:earthslime".into())]));
    }
    #[test]
    fn hopper_only_inserts_one_empty_input_and_extracts_idle_output() {
        let mut c = Casting::new();
        assert!(c.hopper_push_allowed()); assert!(!c.hopper_pull_allowed());
        c.input = Some(ItemStack::new("minecraft:book", 1));
        assert!(!c.hopper_push_allowed()); assert!(!c.hopper_pull_allowed());
        c.output = Some(ItemStack::new("minecraft:stone", 2));
        assert!(!c.hopper_push_allowed()); assert!(c.hopper_pull_allowed());
        c.tank.amount = 1;
        assert!(!c.hopper_push_allowed()); assert!(!c.hopper_pull_allowed());
        c.tank.amount = 0; c.recipe = Some("test:pending".into());
        assert!(!c.hopper_push_allowed()); assert!(!c.hopper_pull_allowed());
    }
    #[test]
    fn hopper_transfer_stages_one_preserving_other_slots_and_components() {
        let mut book = ItemStack::new("minecraft:book", 3);
        book.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("marker", Tag::Int(7))])));
        let hopper = vec![None, Some(book.clone()), Some(ItemStack::new("minecraft:stone", 1))];
        let table = Casting::new();
        let (next, table) = table.hopper_transfer(&hopper, 1, true).unwrap();
        assert_eq!(next[1].as_ref().unwrap().count, 2);
        assert!(table.input.as_ref().unwrap().same_item_same_components(&book));
        assert_eq!(table.input.as_ref().unwrap().count, 1);
        assert_eq!(next[2], hopper[2]); assert_eq!(hopper[1].as_ref().unwrap().count, 3);
        assert!(table.hopper_transfer(&next, 1, true).is_none());
        let mut table = Casting::new(); table.output = Some(book.clone());
        let (next, table) = table.hopper_transfer(&hopper, 1, false).unwrap();
        assert_eq!(next[1].as_ref().unwrap().count, 4);
        assert_eq!(table.output.as_ref().unwrap().count, 2);
        assert!(table.hopper_transfer(&hopper, 2, false).is_none());
        assert!(table.hopper_transfer(&hopper, 9, false).is_none());
    }
    #[test]
    fn hopper_slot_patch_preserves_unrelated_raw_data_and_rejects_stale_slots() {
        let book = ItemStack::new("minecraft:book", 1);
        let mut c = Casting::new(); c.input = Some(book.clone());
        let Tag::Compound(mut raw) = c.to_nbt() else { panic!() };
        raw.insert("future_field".into(), Tag::Int(37));
        let saved = Tag::Compound(raw);
        let patched = patch_casting_slot(&saved, "input", &Some(book.clone()), &None).unwrap();
        assert_eq!(patched.get("future_field"), Some(&Tag::Int(37)));
        assert!(patched.get("input").is_none());
        assert!(patch_casting_slot(&saved, "input", &None, &None).is_none());
        assert!(patch_casting_slot(&saved, "unknown", &None, &None).is_none());
        let mut output = ItemStack::new("minecraft:book", 3);
        output.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("marker", Tag::Int(7))])));
        let mut table = Casting::new(); table.output = Some(output.clone());
        let saved = table.to_nbt();
        let mut remaining = output.clone(); remaining.count = 2;
        let patched = patch_casting_slot(&saved, "output", &Some(output), &Some(remaining.clone())).unwrap();
        let restored = Casting::from_nbt(&patched).unwrap();
        assert_eq!(restored.output.as_ref().unwrap().count, 2);
        assert!(restored.output.as_ref().unwrap().same_item_same_components(&remaining));
    }
    #[test]
    fn mighty_book_casting_waits_on_invalid_stacks_and_preserves_data_across_reload() {
        let mut book = ItemStack::new("minecraft:book", 2);
        book.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("marker", Tag::Int(7))])));
        let mut c = Casting::new(); c.input = Some(book.clone());
        let result = ItemStack::new("tconstruct:mighty_smelting", 1);
        assert!(!c.prepare("tconstruct:common/mighty_smelting", 250, 89, result.clone(), true, false));
        assert_eq!(c.tank.capacity, 0);
        assert_eq!(c.input.as_ref().unwrap().count, 2);
        book.count = 1; c.input = Some(book.clone());
        assert!(c.prepare("tconstruct:common/mighty_smelting", 250, 89, result, true, false));
        assert!(c.tank.insert("tconstruct:seared_stone", 249));
        assert!(!c.tick(true)); assert_eq!(c.input.as_ref().unwrap().count, 1);
        assert!(c.tank.insert("tconstruct:seared_stone", 1));
        let mut c = Casting::from_nbt(&c.to_nbt()).unwrap();
        assert!(c.input.as_ref().unwrap().same_item_same_components(&book));
        assert!(!c.tick(false)); assert_eq!(c.tank.amount, 250);
        c.input.as_mut().unwrap().count = 2;
        assert!(!c.tick(true)); assert_eq!(c.input.as_ref().unwrap().count, 2);
        assert_eq!(c.tank.amount, 250); assert!(c.output.is_none());
        let mut c = Casting::from_nbt(&c.to_nbt()).unwrap();
        assert_eq!(c.input.as_ref().unwrap().count, 2);
        assert_eq!(c.input.as_ref().unwrap().data().unwrap().other("minecraft:custom_data").unwrap().get("marker"), Some(&Tag::Int(7)));
        assert!(!c.tick(true)); assert_eq!(c.tank.amount, 250);
        let excess = c.recover_excess_input().unwrap();
        assert_eq!(excess.count, 1);
        assert!(excess.same_item_same_components(&book));
        assert_eq!(c.input.as_ref().unwrap().count, 1);
        assert_eq!(c.tank.amount, 250);
        assert!(c.recover_excess_input().is_none());
        for _ in 0..88 { assert!(!c.tick(true)); }
        assert!(c.tick(true)); assert!(c.input.is_none());
        assert_eq!(c.tank.amount, 0);
        assert_eq!((&*c.output.as_ref().unwrap().item, c.output.as_ref().unwrap().count), ("tconstruct:mighty_smelting", 1));
    }
    #[test]
    fn excess_recovery_only_changes_a_pending_oversized_input() {
        let mut c = Casting::new();
        c.input = Some(ItemStack::new("minecraft:book", 3));
        assert!(c.recover_excess_input().is_none());
        assert_eq!(c.input.as_ref().unwrap().count, 3);
        c.input.as_mut().unwrap().count = 1;
        assert!(c.prepare("test:book", 250, 89, ItemStack::new("tconstruct:mighty_smelting", 1), true, false));
        assert!(c.tank.insert("tconstruct:seared_stone", 100));
        c.input.as_mut().unwrap().count = 3;
        let excess = c.recover_excess_input().unwrap();
        assert_eq!((excess.count, c.input.as_ref().unwrap().count, c.tank.amount, c.timer), (2, 1, 100, 0));
        assert_eq!(c.recipe.as_deref(), Some("test:book"));
    }
    #[test]
    fn partially_filled_cast_resumes_and_preserves_reusable_cast() {
        let mut c = Casting::new(); c.input = Some(ItemStack::new("tconstruct:ingot_cast", 1));
        assert!(c.prepare("tconstruct:copper_ingot", 90, 2, ItemStack::new("minecraft:copper_ingot", 1), false, false));
        assert!(c.tank.insert("tconstruct:molten_copper", 40)); assert!(!c.tick(true));
        let mut c = Casting::from_nbt(&c.to_nbt()).unwrap();
        assert!(c.tank.insert("tconstruct:molten_copper", 50)); assert!(!c.tick(true)); assert!(c.tick(true));
        assert_eq!(c.input.as_ref().unwrap().item.as_ref(), "tconstruct:ingot_cast");
        assert_eq!(c.output.as_ref().unwrap().item.as_ref(), "minecraft:copper_ingot");
        assert_eq!(c.tank.amount, 0);
    }
    #[test]
    fn occupied_output_and_invalid_recipe_do_not_consume_inputs_or_fluid() {
        let mut c = Casting::new(); c.output = Some(ItemStack::new("minecraft:stone", 1));
        assert!(!c.prepare("tconstruct:cast", 90, 1, ItemStack::new("minecraft:copper_ingot", 1), true, false));
        c.output = None; c.input = Some(ItemStack::new("tconstruct:ingot_sand_cast", 1));
        assert!(c.prepare("tconstruct:cast", 90, 1, ItemStack::new("minecraft:copper_ingot", 1), true, false));
        assert!(c.tank.insert("tconstruct:molten_copper", 90)); assert!(!c.tick(false));
        assert_eq!(c.tank.amount, 90); assert!(c.input.is_some()); assert!(c.output.is_none());
        assert!(c.tick(true)); assert!(c.input.is_none()); assert!(c.output.is_some());
    }
}

#[cfg(test)]
mod slimelytra_tests {
    use super::*;

#[test]
fn slimelytra_stays_atomic_through_rejected_and_reloaded_pours() {
    let mut result = ItemStack::new("tconstruct:slime_wings", 1);
    result.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
        ("tic_materials", Tag::List(vec![Tag::String("tconstruct:earthslime".into())])),
        ("tic_max_damage", Tag::Int(222)),
    ])));
    let mut basin = Casting::new();
    basin.input = Some(ItemStack::new("minecraft:elytra", 1));
    basin.output = Some(ItemStack::new("minecraft:stone", 1));
    assert!(!basin.prepare("test:slimelytra", 2000, 2, result.clone(), true, false));
    assert_eq!(basin.tank.amount, 0); assert!(basin.input.is_some());
    basin.output = None;
    assert!(basin.prepare("test:slimelytra", 2000, 2, result, true, false));
    assert!(basin.tank.insert("tconstruct:molten_slime", 1999));
    assert!(!basin.tick(true)); assert!(basin.input.is_some());
    assert!(basin.tank.insert("tconstruct:molten_slime", 1));
    let mut basin = Casting::from_nbt(&basin.to_nbt()).unwrap();
    assert!(!basin.tick(false)); assert!(basin.input.is_some()); assert_eq!(basin.tank.amount, 2000);
    basin.output = Some(ItemStack::new("minecraft:stone", 1));
    assert!(!basin.tick(true)); assert!(basin.input.is_some()); assert_eq!(basin.tank.amount, 2000);
    assert_eq!(&*basin.output.as_ref().unwrap().item, "minecraft:stone");
    basin.output = None;
    assert!(!basin.tick(true)); assert!(basin.tick(true));
    assert!(basin.input.is_none()); assert_eq!(basin.tank.amount, 0);
    let output = basin.output.unwrap();
    assert_eq!(&*output.item, "tconstruct:slime_wings");
    assert_eq!(output.data().unwrap().other("minecraft:custom_data").unwrap().get("tic_max_damage"), Some(&Tag::Int(222)));
}

}

#[cfg(test)]
mod tagged_portable_tests {
    use super::*;
    #[test]
    fn tagged_fill_preserves_components_and_requires_matching_fluid_data() {
        let mut stack = ItemStack::new("tconstruct:seared_lantern", 1);
        stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("other", Tag::Int(17))])));
        let tag = Tag::compound([("Potion", Tag::String("minecraft:healing".into()))]);
        let other = Tag::compound([("Potion", Tag::String("minecraft:poison".into()))]);
        let output = fill_portable_tagged(&stack, portable_tank(&stack, 50).unwrap(), "tconstruct:potion", &tag, 10).unwrap();
        let custom = output.data().unwrap().other("minecraft:custom_data").unwrap();
        assert_eq!(custom.get("other"), Some(&Tag::Int(17)));
        let tank = portable_tank(&output, 50).unwrap();
        assert_eq!((tank.amount, tank.tag.as_ref()), (10, Some(&tag)));
        assert!(fill_portable_tagged(&output, tank.clone(), "tconstruct:potion", &other, 10).is_none());
        assert!(fill_portable_tagged(&output, tank.clone(), "minecraft:water", &tag, 10).is_none());
        assert!(fill_portable_tagged(&output, tank.clone(), "tconstruct:potion", &tag, 41).is_none());
        assert_eq!(fill_portable_tagged(&output, tank, "tconstruct:potion", &tag, 40).and_then(|s| portable_tank(&s, 50)).unwrap().amount, 50);
        assert!(stack.data().unwrap().other("minecraft:custom_data").unwrap().get("tconstruct:tank").is_none());
    }
    #[test]
    fn tagged_casting_roundtrips_and_waits_for_full_reserved_amount() {
        let input = ItemStack::new("tconstruct:seared_lantern", 1);
        let metadata = Tag::compound([("Potion", Tag::String("minecraft:healing".into()))]);
        let result = fill_portable_tagged(&input, portable_tank(&input, 50).unwrap(), "tconstruct:potion", &metadata, 10).unwrap();
        let mut casting = Casting::new();
        casting.input = Some(input);
        assert!(casting.prepare("test:lantern", 10, 5, result, true, false));
        assert!(casting.tank.insert_tagged("tconstruct:potion", Some(&metadata), 9));
        assert!(!casting.tick(true));
        assert_eq!(casting.tank.amount, 9);
        assert!(casting.tank.insert_tagged("tconstruct:potion", Some(&metadata), 1));
        let mut restored = Casting::from_nbt(&casting.to_nbt()).unwrap();
        assert_eq!(restored.tank.tag.as_ref(), Some(&metadata));
        for _ in 0..4 { assert!(!restored.tick(true)); }
        assert!(!restored.tick(false));
        for _ in 0..4 { assert!(!restored.tick(true)); }
        assert!(restored.tick(true));
        let output = restored.output.as_ref().unwrap();
        let tank = portable_tank(output, 50).unwrap();
        assert_eq!((tank.amount, tank.tag.as_ref()), (10, Some(&metadata)));
        assert!(restored.input.is_none());
        assert_eq!(restored.tank.amount, 0);
    }
    #[test]
    fn tagged_fill_preserves_extensions_and_rejects_non_single_items() {
        let mut stack = ItemStack::new("tconstruct:seared_lantern", 1);
        stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tconstruct:tank", Tag::compound([("amount", Tag::Int(0)), ("unknown", Tag::Int(1))]))])));
        let output = fill_portable_tagged(&stack, portable_tank(&stack, 50).unwrap(), "tconstruct:potion", &Tag::compound([("Potion", Tag::String("minecraft:healing".into()))]), 10).unwrap();
        assert_eq!(output.data().unwrap().other("minecraft:custom_data").unwrap().get("tconstruct:tank").unwrap().get("unknown"), Some(&Tag::Int(1)));
        let mut two = ItemStack::new("tconstruct:seared_lantern", 2);
        assert!(portable_tank(&two, 50).is_none());
        two.count = 1;
        assert!(portable_tank(&two, 50).is_some());
    }
}

#[cfg(test)]
mod first_material_input_tests {
    use super::*;
    #[test]
    fn only_plain_single_material_parts_are_accepted() {
        let mut part = ItemStack::new("tconstruct:laces", 1);
        part.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("Material", Tag::String("tconstruct:vine".into()))])));
        assert_eq!(plain_material_part(&part).as_deref(), Some("tconstruct:vine"));
        let mut extra = part.clone();
        extra.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("Material", Tag::String("tconstruct:vine".into())), ("other", Tag::Int(1))])));
        assert!(plain_material_part(&extra).is_none());
        let mut named = part.clone(); named.data_mut().set_other("minecraft:custom_name", Some(Tag::String("name".into())));
        assert!(plain_material_part(&named).is_none());
        let mut stack = part.clone(); stack.count = 2; assert!(plain_material_part(&stack).is_none());
        let mut damaged = part; damaged.damage = 1; assert!(plain_material_part(&damaged).is_none());
    }
}

#[cfg(test)]
mod swap_presentation_tests {
    use super::*;
    #[test]
    fn name_and_lore_are_the_only_allowed_differences() {
        let pristine = ItemStack::new("tconstruct:slime_helmet", 1);
        assert!(same_pristine_with_presentation(&pristine, &pristine));
        let mut input = pristine.clone();
        input.data_mut().set_other("minecraft:custom_name", Some(Tag::String("My helmet".into())));
        input.data_mut().set_other("minecraft:lore", Some(Tag::List(vec![Tag::String("Keepsake".into())])));
        assert!(same_pristine_with_presentation(&input, &pristine));
        assert!(presentation_preserved(&input, &input));
        let decoded = horizon_items::item_nbt::item_from_nbt(&horizon_items::item_nbt::item_to_nbt(&input)).unwrap();
        assert!(presentation_preserved(&input, &decoded));
        assert!(input.same_item_same_components(&decoded));
        assert!(!presentation_preserved(&input, &pristine));
        let mut changed = input.clone(); changed.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("unknown", Tag::Int(1))])));
        assert!(!same_pristine_with_presentation(&changed, &pristine));
        let mut wrong_item = input.clone(); wrong_item.item = "minecraft:stone".into();
        assert!(!same_pristine_with_presentation(&wrong_item, &pristine));
        let mut corrupt = input.clone(); corrupt.data_mut().set_other("minecraft:lore", Some(Tag::Int(3)));
        assert!(!same_pristine_with_presentation(&corrupt, &pristine));
        corrupt.data_mut().set_other("minecraft:lore", Some(Tag::List(vec![Tag::Int(3)])));
        assert!(!same_pristine_with_presentation(&corrupt, &pristine));
        corrupt = input.clone(); corrupt.data_mut().custom_name = Some(Tag::Int(3));
        assert!(!same_pristine_with_presentation(&corrupt, &pristine));
        let mut damaged = input.clone(); damaged.damage = 1;
        assert!(!same_pristine_with_presentation(&damaged, &pristine));
        let mut stacked = input; stacked.count = 2;
        assert!(!same_pristine_with_presentation(&stacked, &pristine));
    }
}


#[cfg(test)]
mod compound_patch_tests {
    use super::*;
    use horizon_world::nbt::JavaString;

    #[test]
    fn tank_patch_preserves_opaque_extension_keys_and_removes_emptied_owned_fields() {
        let mut tank = Tank::new(90); assert!(tank.insert("example:fluid", 90));
        let mut raw = tank.to_nbt();
        let key = JavaString(vec![0xd800]);
        let opaque = Tag::String(JavaString(vec![0xdc00]));
        if let Tag::Compound(fields) = &mut raw { fields.insert(key.clone(), opaque.clone()); }
        assert_eq!(tank.take(90).as_deref(), Some("example:fluid"));
        let saved = tank.to_nbt_preserving(&raw).unwrap();
        assert!(saved.get("fluid").is_none()); assert_eq!(saved.get("amount"), Some(&Tag::Int(0)));
        let Tag::Compound(fields) = saved else { panic!("compound"); };
        assert_eq!(fields.get(&key), Some(&opaque));
        assert!(raw.get("fluid").is_some());
    }

    #[test]
    fn faucet_keeps_its_explicit_empty_record_fallback() {
        let expected = Faucet::new().to_nbt();
        let mut faucet = Faucet::new(); faucet.original = Some(Tag::Int(9));
        assert_eq!(faucet.to_nbt(), expected);
        faucet.original = Some(Tag::compound([("pending", Tag::Int(9)), ("future", Tag::Int(3))]));
        let saved = faucet.to_nbt();
        assert_eq!(saved.get("pending"), expected.get("pending"));
        assert_eq!(saved.get("future"), Some(&Tag::Int(3)));
    }
}
