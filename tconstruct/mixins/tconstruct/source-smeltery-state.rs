// Controller-owned melting state. Recipe and fuel lookup, structure discovery,
// live inventory synchronization and four-phase scheduling belong to the caller.
use horizon_items::{item_nbt::{item_from_nbt, item_to_nbt}, items::ItemStack};
use horizon_world::nbt::Tag;
use super::tconstruct_source_smeltery_tank::SmelteryTank;

#[derive(Clone, Debug)]
pub(super) struct Smeltery {
    pub slots: Vec<Option<ItemStack>>,
    pub progress: Vec<i32>,
    pub tank: SmelteryTank,
    pub fuel: u32,
    // Original fuel quality is the total duration used by the GUI flame.
    pub fuel_quality: u32,
    pub temperature: u32,
    pub rate: u32,
    pub fuel_cost: u32,
    pub phase: u8,
}
impl Smeltery {
    pub(super) fn new() -> Self {
        Self { slots: Vec::new(), progress: Vec::new(), tank: SmelteryTank::new(0), fuel: 0, fuel_quality: 0, temperature: 0, rate: 0, fuel_cost: 1, phase: 0 }
    }
    /// Resize with the structure. The caller must return or drop any displaced
    /// stacks; shrinking capacity does not discard already stored liquid.
    pub(super) fn resize(&mut self, volume: usize, capacity: u32) -> Vec<ItemStack> {
        let displaced = if volume < self.slots.len() {
            self.slots.drain(volume..).flatten().collect()
        } else { Vec::new() };
        self.slots.resize_with(volume, || None);
        self.progress.resize(volume, 0);
        self.tank.capacity = capacity;
        displaced
    }
    pub(super) fn can_heat_slot(&self, slot: usize, temperature: u32, required: u32) -> bool {
        self.slots.get(slot).is_some_and(Option::is_some)
            && self.progress.get(slot).is_some_and(|p| *p != -1) && temperature >= required
    }
    /// Source NO_SPACE (-1) keeps a finished item hot and retries it without
    /// consuming fuel when capacity is freed.
    pub(super) fn heat_slot(&mut self, slot: usize, required_temp: u32, required_time: u32, fluid: &str, amount: u32) -> bool {
        if self.slots.get(slot).is_none_or(Option::is_none) || required_time == 0 { return false; }
        let Some(progress) = self.progress.get_mut(slot) else { return false; };
        if *progress != -1 && (self.temperature < required_temp || self.rate == 0) { return false; }
        if *progress == -1 || u32::try_from(*progress).is_ok_and(|p| p >= required_time) {
            if !self.tank.insert(fluid, amount) { *progress = -1; return false; }
            let stack = self.slots[slot].as_mut().unwrap();
            stack.count -= 1;
            if stack.count == 0 { self.slots[slot] = None; }
            *progress = 0;
            true
        } else {
            *progress = progress.saturating_add(self.rate.min(i32::MAX as u32) as i32);
            false
        }
    }
    pub(super) fn heat_foundry_slot(&mut self, slot: usize, required_temp: u32, required_time: u32,
        primary: &super::tconstruct_source_smeltery_tank::FluidAmount,
        byproducts: &[super::tconstruct_source_smeltery_tank::FluidAmount]) -> bool {
        if self.slots.get(slot).is_none_or(Option::is_none) || required_time == 0 { return false; }
        let Some(progress) = self.progress.get_mut(slot) else { return false; };
        if *progress != -1 && (self.temperature < required_temp || self.rate == 0) { return false; }
        if *progress == -1 || u32::try_from(*progress).is_ok_and(|p| p >= required_time) {
            if !self.tank.foundry_output(primary, byproducts) { *progress = -1; return false; }
            let stack = self.slots[slot].as_mut().unwrap();
            stack.count -= 1;
            if stack.count == 0 { self.slots[slot] = None; }
            *progress = 0;
            true
        } else {
            *progress = progress.saturating_add(self.rate.min(i32::MAX as u32) as i32);
            false
        }
    }
    pub(super) fn cool_slot(&mut self, slot: usize) {
        if let Some(progress) = self.progress.get_mut(slot) { if *progress > 0 { *progress = (*progress - 5).max(0); } }
    }
    /// An invalid structure consumes already-burning fuel without heating;
    /// only after fuel runs out do its inputs begin cooling.
    pub(super) fn tick_invalid_structure(&mut self) {
        if self.phase == 1 && self.fuel == 0 {
            for slot in 0..self.slots.len() { self.cool_slot(slot); }
        }
        if self.phase == 3 && self.fuel > 0 {
            self.fuel = self.fuel.saturating_sub(self.fuel_cost);
        }
        self.phase = (self.phase + 1) % 4;
    }
    pub(super) fn to_nbt(&self) -> Tag {
        let slots = self.slots.iter().map(|slot| slot.as_ref().map(item_to_nbt).unwrap_or_else(|| Tag::compound(Vec::<(&str, Tag)>::new()))).collect();
        Tag::compound([("slots", Tag::List(slots)), ("progress", Tag::IntArray(self.progress.clone())),
            ("tank", self.tank.to_nbt()), ("fuel", Tag::Long(i64::from(self.fuel))), ("fuel_quality", Tag::Long(i64::from(self.fuel_quality))),
            ("temperature", Tag::Long(i64::from(self.temperature))), ("rate", Tag::Long(i64::from(self.rate))),
            ("fuel_cost", Tag::Long(i64::from(self.fuel_cost))), ("phase", Tag::Int(i32::from(self.phase)))])
    }
    pub(super) fn from_nbt(tag: &Tag) -> Option<Self> {
        let Tag::List(slots) = tag.get("slots")? else { return None; };
        let mut inventory = Vec::with_capacity(slots.len());
        for value in slots {
            inventory.push(if matches!(value, Tag::Compound(map) if map.is_empty()) { None } else { Some(item_from_nbt(value)?) });
        }
        let Tag::IntArray(progress) = tag.get("progress")? else { return None; };
        if progress.len() != inventory.len() || progress.iter().any(|value| *value < -1) { return None; }
        let number = |key| match tag.get(key)? { Tag::Long(n) => u32::try_from(*n).ok(), _ => None };
        let phase = match tag.get("phase")? { Tag::Int(n) if (0..4).contains(n) => *n as u8, _ => return None };
        Some(Self { slots: inventory, progress: progress.clone(), tank: SmelteryTank::from_nbt(tag.get("tank")?)?,
            fuel: number("fuel")?, fuel_quality: number("fuel_quality").unwrap_or(number("fuel")?), temperature: number("temperature")?, rate: number("rate")?, fuel_cost: number("fuel_cost").unwrap_or(1), phase })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fuel_quality_survives_reload_and_legacy_data() {
        let mut state = Smeltery::new(); state.fuel = 35; state.fuel_quality = 100;
        let mut saved = state.to_nbt();
        assert_eq!(Smeltery::from_nbt(&saved).unwrap().fuel_quality, 100);
        if let Tag::Compound(map) = &mut saved { map.retain(|key, _| key.to_string_lossy() != "fuel_quality"); }
        assert_eq!(Smeltery::from_nbt(&saved).unwrap().fuel_quality, 35);
    }
    #[test]
    fn invalid_structure_burns_existing_fuel_before_cooling() {
        let mut s = Smeltery::new(); s.resize(1, 1080);
        s.progress[0] = 20; s.fuel = 2; s.fuel_cost = 2; s.phase = 1;
        s.tick_invalid_structure(); assert_eq!(s.progress[0], 20);
        s.tick_invalid_structure(); s.tick_invalid_structure();
        assert_eq!(s.fuel, 0); assert_eq!(s.progress[0], 20);
        s.tick_invalid_structure(); s.tick_invalid_structure();
        assert_eq!(s.progress[0], 15);
        assert_eq!(Smeltery::from_nbt(&s.to_nbt()).unwrap().fuel_cost, 2);
    }
    #[test]
    fn foundry_consumes_input_only_after_primary_and_byproduct_are_staged() {
        use super::super::tconstruct_source_smeltery_tank::FluidAmount;
        let mut s = Smeltery::new(); s.resize(1, 205);
        s.slots[0] = Some(ItemStack::new("minecraft:copper_ore", 1));
        s.temperature = 1000; s.rate = 10; s.progress[0] = 500;
        let primary = FluidAmount { id: "copper".into(), amount: 180 };
        let extra = [FluidAmount { id: "gold".into(), amount: 30 }];
        assert!(s.heat_foundry_slot(0, 500, 500, &primary, &extra));
        assert!(s.slots[0].is_none());
        assert_eq!((s.tank.amount("copper"), s.tank.amount("gold")), (180, 25));
    }
    #[test]
    fn blocked_output_and_storage_round_trip() {
        let mut s = Smeltery::new(); s.resize(1, 90); s.slots[0] = Some(ItemStack::new("minecraft:copper_ingot", 1));
        s.temperature = 1000; s.rate = 10; s.progress[0] = 500;
        assert!(s.tank.insert("tconstruct:molten_copper", 90));
        assert!(!s.heat_slot(0, 500, 500, "tconstruct:molten_copper", 90));
        assert!(!s.can_heat_slot(0, 1000, 500));
        let mut s = Smeltery::from_nbt(&s.to_nbt()).unwrap();
        assert!(s.tank.take("tconstruct:molten_copper", 90));
        assert!(s.heat_slot(0, 500, 500, "tconstruct:molten_copper", 90));
        assert!(s.slots[0].is_none());
    }
    #[test]
    fn shrink_returns_displaced_items_and_keeps_fluid() {
        let mut s = Smeltery::new(); s.resize(2, 2160);
        s.slots[1] = Some(ItemStack::new("minecraft:copper_ingot", 2));
        assert!(s.tank.insert("tconstruct:molten_copper", 1800));
        assert_eq!(s.resize(1, 1080)[0].count, 2);
        assert_eq!(s.tank.total(), 1800); assert_eq!(s.progress.len(), 1);
    }
    #[test]
    fn two_inputs_produce_source_amethyst_bronze_without_losing_components() {
        use super::super::tconstruct_source_smeltery_tank::FluidAmount;
        let mut s = Smeltery::new(); s.resize(2, 2160);
        s.slots[0] = Some(ItemStack::new("minecraft:copper_ingot", 1));
        s.slots[1] = Some(ItemStack::new("minecraft:amethyst_shard", 1));
        s.temperature = 1000; s.rate = 10; s.progress = vec![500, 650];
        assert!(s.heat_slot(0, 500, 500, "tconstruct:molten_copper", 90));
        assert!(s.heat_slot(1, 950, 650, "tconstruct:molten_amethyst", 100));
        let f = |id: &str, amount| FluidAmount { id: id.into(), amount };
        assert!(s.tank.alloy(&[f("tconstruct:molten_copper", 90), f("tconstruct:molten_amethyst", 100)], &f("tconstruct:molten_amethyst_bronze", 90)));
        assert_eq!(s.tank.amount("tconstruct:molten_amethyst_bronze"), 90);
        assert!(s.slots.iter().all(Option::is_none));
        assert_eq!(Smeltery::from_nbt(&s.to_nbt()).unwrap().tank, s.tank);
    }
}
