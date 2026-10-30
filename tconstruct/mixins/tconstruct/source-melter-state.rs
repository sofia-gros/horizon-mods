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
    fn to_nbt(&self) -> Tag {
        let mut value = Compound::new();
        if let Some(id) = &self.fluid { value.insert("fluid".into(), Tag::String(id.as_str().into())); }
        value.insert("amount".into(), Tag::Int(self.amount as i32));
        Tag::Compound(value)
    }
    fn from_nbt(tag: &Tag, capacity: u32) -> Option<Self> {
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
    pub temperature: u32,
    pub rate: u32,
}
impl Melter {
    pub(super) fn new() -> Self {
        Self { slots: [None, None, None], progress: [0; 3], tank: Tank::new(MELTER_CAPACITY), phase: 0, fuel: 0, temperature: 0, rate: 0 }
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
            ("fuel", Tag::Long(self.fuel as i64)), ("temperature", Tag::Long(self.temperature as i64)), ("rate", Tag::Long(self.rate as i64))])
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
            fuel: number("fuel")?, temperature: number("temperature")?, rate: number("rate")? })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn tank_rejects_mixing_and_state_round_trips() {
        let mut m = Melter::new(); m.slots[1] = Some(ItemStack::new("minecraft:iron_ingot", 3));
        m.progress[1] = 7; m.fuel = 30; m.temperature = 800; m.rate = 2; m.phase = 2;
        assert!(m.tank.insert("tconstruct:molten_iron", 90));
        assert!(!m.tank.insert("tconstruct:molten_copper", 1));
        let copy = Melter::from_nbt(&m.to_nbt()).unwrap();
        assert_eq!(copy.to_nbt(), m.to_nbt());
    }
}
