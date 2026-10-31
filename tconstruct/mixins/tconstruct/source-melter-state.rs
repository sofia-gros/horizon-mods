// Small melter state transitions. The caller supplies a source-matched recipe,
// a resolved fluid ID, and the fuel's temperature/rate; no fuel or recipe is
// invented here. Inventory stacks and the tank can be saved without losing
// components through Horizon's existing NBT item codec.
use horizon_items::{item_nbt::{item_from_nbt, item_to_nbt}, items::ItemStack};
use horizon_world::nbt::{Compound, Tag};

pub(super) const MELTER_CAPACITY: u32 = 24 * 90;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Tank { pub fluid: Option<String>, pub amount: u32, pub capacity: u32 }
impl Tank {
    pub(super) fn new(capacity: u32) -> Self { Self { fluid: None, amount: 0, capacity } }
    pub(super) fn insert(&mut self, fluid: &str, amount: u32) -> bool {
        if amount == 0 || fluid.is_empty() || self.fluid.as_deref().is_some_and(|id| id != fluid)
            || amount > self.capacity.saturating_sub(self.amount) { return false; }
        self.fluid = Some(fluid.to_owned()); self.amount += amount; true
    }
    pub(super) fn take(&mut self, amount: u32) -> Option<String> {
        if amount == 0 || amount > self.amount { return None; }
        let fluid = self.fluid.clone()?;
        self.amount -= amount;
        if self.amount == 0 { self.fluid = None; }
        Some(fluid)
    }
    pub(super) fn to_nbt(&self) -> Tag {
        let mut value = Compound::new();
        if let Some(id) = &self.fluid { value.insert("fluid".into(), Tag::String(id.as_str().into())); }
        value.insert("amount".into(), Tag::Int(self.amount as i32));
        Tag::Compound(value)
    }
    pub(super) fn from_nbt(tag: &Tag, capacity: u32) -> Option<Self> {
        let amount = match tag.get("amount")? { Tag::Int(n) => u32::try_from(*n).ok()?, _ => return None };
        if amount > capacity { return None; }
        let fluid = match tag.get("fluid") { Some(Tag::String(id)) => Some(id.to_string_lossy()), None => None, _ => return None };
        if (amount == 0) != fluid.is_none() || fluid.as_ref().is_some_and(String::is_empty) { return None; }
        Some(Self { fluid, amount, capacity })
    }
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
            if !self.tank.insert(fluid, amount) { self.progress[slot] = -1; return false; }
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
pub(super) struct Faucet { pub active: bool, pub stop_after_packet: bool, pub pending: Tank }
impl Faucet {
    pub(super) fn new() -> Self { Self { active: false, stop_after_packet: false, pending: Tank::new(90) } }
    pub(super) fn activate(&mut self, input: &mut Tank, output: &Tank) -> bool {
        if self.active { self.stop_after_packet = true; return true; }
        self.stop_after_packet = false;
        self.active = self.begin_packet(input, output);
        self.active
    }
    fn begin_packet(&mut self, input: &mut Tank, output: &Tank) -> bool {
        if self.pending.amount != 0 { return false; }
        let Some(fluid) = input.fluid.as_deref() else { return false; };
        if output.fluid.as_deref().is_some_and(|other| other != fluid) { return false; }
        let amount = input.amount.min(90).min(output.capacity.saturating_sub(output.amount));
        if amount == 0 { return false; }
        let Some(fluid) = input.take(amount) else { return false; };
        // An empty buffer with capacity 90 always accepts this validated amount.
        debug_assert!(self.pending.amount == 0 && amount <= self.pending.capacity);
        self.pending.fluid = Some(fluid); self.pending.amount = amount;
        true
    }
    pub(super) fn tick(&mut self, input: &mut Tank, output: Option<&mut Tank>) {
        if !self.active { return; }
        let Some(output) = output else { self.pending = Tank::new(90); self.active = false; self.stop_after_packet = false; return; };
        if self.pending.amount > 0 {
            let Some(fluid) = self.pending.fluid.as_deref() else { self.active = false; return; };
            if output.fluid.as_deref().is_some_and(|other| other != fluid) { return; }
            let amount = self.pending.amount.min(10).min(output.capacity.saturating_sub(output.amount));
            if amount == 0 { return; }
            let Some(fluid) = self.pending.take(amount) else { return; };
            let inserted = output.insert(&fluid, amount);
            debug_assert!(inserted);
        } else if self.stop_after_packet || !self.begin_packet(input, output) {
            self.active = false; self.stop_after_packet = false;
        }
    }
    pub(super) fn to_nbt(&self) -> Tag {
        Tag::compound([("active", Tag::Byte(self.active as i8)), ("stop", Tag::Byte(self.stop_after_packet as i8)), ("pending", self.pending.to_nbt())])
    }
    pub(super) fn from_nbt(tag: &Tag) -> Option<Self> {
        let boolean = |key| match tag.get(key)? { Tag::Byte(0) => Some(false), Tag::Byte(1) => Some(true), _ => None };
        let active = boolean("active")?; let stop_after_packet = boolean("stop")?;
        let pending = Tank::from_nbt(tag.get("pending")?, 90)?;
        if !active && (stop_after_packet || pending.amount != 0) { return None; }
        Some(Self { active, stop_after_packet, pending })
    }
}

#[cfg(test)]
mod faucet_tests {
    use super::*;
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
impl Casting {
    pub(super) fn new() -> Self {
        Self { input: None, output: None, tank: Tank::new(0), timer: 0, recipe: None, result: None,
            cooling_time: 0, consumes_cast: false, switch_slots: false, redstone: false }
    }
    /// Begin only after the caller has matched the cast, fluid and recipe.
    pub(super) fn prepare(&mut self, recipe: &str, amount: u32, cooling_time: u32, result: ItemStack, consumes_cast: bool, switch_slots: bool) -> bool {
        if self.recipe.is_some() || self.tank.amount != 0 || self.output.is_some() || recipe.is_empty()
            || amount == 0 || cooling_time == 0 || result.count == 0 { return false; }
        self.tank = Tank::new(amount); self.timer = 0; self.recipe = Some(recipe.to_owned()); self.result = Some(result);
        self.cooling_time = cooling_time; self.consumes_cast = consumes_cast; self.switch_slots = switch_slots;
        true
    }
    /// Cooling starts only after the precise amount is present. Revalidate the
    /// selected recipe against the current cast/fluid before consuming either.
    pub(super) fn tick(&mut self, valid: bool) -> bool {
        if self.recipe.is_none() || self.result.is_none() || self.tank.amount != self.tank.capacity || self.tank.amount == 0 { return false; }
        if !valid { self.timer = 0; return false; }
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
