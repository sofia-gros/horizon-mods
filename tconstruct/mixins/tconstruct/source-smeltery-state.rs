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
    pub fuel_view: super::tconstruct_source_smeltery_fuel::FuelView,
    pub temperature: u32,
    pub rate: u32,
    pub fuel_cost: u32,
    pub phase: u8,
    pub entity_phase: u8,
}
impl Smeltery {
    pub(super) fn new() -> Self {
        Self { slots: Vec::new(), progress: Vec::new(), tank: SmelteryTank::new(0), fuel: 0, fuel_quality: 0, fuel_view: Default::default(), temperature: 0, rate: 0, fuel_cost: 1, phase: 0, entity_phase: 0 }
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
            if amount != 0 && !self.tank.insert(fluid, amount) { *progress = -1; return false; }
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
        self.entity_phase = (self.entity_phase + 1) % 20;
    }
    pub(super) fn to_nbt(&self) -> Tag {
        let slots = self.slots.iter().map(|slot| slot.as_ref().map(item_to_nbt).unwrap_or_else(|| Tag::compound(Vec::<(&str, Tag)>::new()))).collect();
        Tag::compound([("slots", Tag::List(slots)), ("progress", Tag::IntArray(self.progress.clone())),
            ("tank", self.tank.to_nbt()), ("fuel", Tag::Long(i64::from(self.fuel))), ("fuel_quality", Tag::Long(i64::from(self.fuel_quality))), ("fuel_view", self.fuel_view.to_nbt()),
            ("temperature", Tag::Long(i64::from(self.temperature))), ("rate", Tag::Long(i64::from(self.rate))),
            ("fuel_cost", Tag::Long(i64::from(self.fuel_cost))), ("phase", Tag::Int(i32::from(self.phase))), ("entity_phase", Tag::Int(i32::from(self.entity_phase)))])
    }
    /// A tank-only controller update must not serialize its inventory, fuel,
    /// progress or unknown fields. Reject callers that also changed other state.
    pub(super) fn tank_to_nbt_preserving(&self, raw: &Tag) -> Option<Tag> {
        let before = Self::from_nbt(raw)?;
        let mut unchanged = self.clone(); unchanged.tank = before.tank.clone();
        if unchanged.to_nbt() != before.to_nbt() { return None; }
        for alias in ["Tank", "Fluids"] { if raw.get(alias).is_some() { return None; } }
        let tank = self.tank.to_nbt_preserving(raw.get("tank")?)?;
        let Tag::Compound(mut fields) = raw.clone() else { return None; };
        fields.insert("tank".into(), tank);
        Some(Tag::Compound(fields))
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
        let entity_phase = match tag.get("entity_phase") { None => 0, Some(Tag::Int(n)) if (0..20).contains(n) => *n as u8, _ => return None };
        Some(Self { slots: inventory, progress: progress.clone(), tank: SmelteryTank::from_nbt(tag.get("tank")?)?,
            fuel: number("fuel")?, fuel_quality: number("fuel_quality").unwrap_or(number("fuel")?), fuel_view: tag.get("fuel_view").and_then(super::tconstruct_source_smeltery_fuel::FuelView::from_nbt).unwrap_or_default(), temperature: number("temperature")?, rate: number("rate")?, fuel_cost: number("fuel_cost").unwrap_or(1), phase, entity_phase })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fuel_view_survives_metadata_roundtrip() {
        let mut state = Smeltery::new();
        state.fuel_view.has_tank = true;
        state.fuel_view.fluid = Some("minecraft:lava".into());
        state.fuel_view.amount = 500;
        state.fuel_view.capacity = 4000;
        let restored = Smeltery::from_nbt(&state.to_nbt()).unwrap();
        assert_eq!(restored.fuel_view.fluid.as_deref(), Some("minecraft:lava"));
        assert_eq!((restored.fuel_view.amount, restored.fuel_view.capacity), (500, 4000));
    }
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
    fn zero_primary_consumes_and_fills_foundry_byproduct() {
        use super::super::tconstruct_source_smeltery_tank::FluidAmount;
        let mut s = Smeltery::new(); s.resize(1, 100);
        s.slots[0] = Some(ItemStack::new("minecraft:copper_ore", 1));
        s.temperature = 500; s.rate = 1; s.progress[0] = 50;
        let mut s = Smeltery::from_nbt(&s.to_nbt()).unwrap();
        let primary = FluidAmount { id: "copper".into(), tag: None, amount: 0 };
        let extra = [FluidAmount { id: "gold".into(), tag: None, amount: 30 }];
        assert!(s.heat_foundry_slot(0, 500, 50, &primary, &extra));
        assert!(s.slots[0].is_none()); assert_eq!(s.tank.amount("gold"), 30);
    }
    #[test]
    fn zero_smeltery_output_consumes_once_even_when_full() {
        let mut s = Smeltery::new(); s.resize(1, 10);
        s.slots[0] = Some(ItemStack::new("minecraft:copper_ore", 2));
        s.temperature = 500; s.rate = 1; s.progress[0] = 50;
        assert!(s.tank.insert("copper", 10));
        assert!(s.heat_slot(0, 500, 50, "copper", 0));
        let mut s = Smeltery::from_nbt(&s.to_nbt()).unwrap();
        assert_eq!(s.slots[0].as_ref().unwrap().count, 1);
        assert!(!s.heat_slot(0, 500, 50, "copper", 0));
        assert_eq!(s.slots[0].as_ref().unwrap().count, 1);
        assert_eq!(s.tank.total(), 10);
    }
    #[test]
    fn zero_foundry_primary_discards_unfilled_byproducts_as_upstream_does() {
        use super::super::tconstruct_source_smeltery_tank::FluidAmount;
        for (occupied, expected_gold) in [(90, 10), (100, 0)] {
            let mut s = Smeltery::new(); s.resize(1, 100);
            s.slots[0] = Some(ItemStack::new("minecraft:copper_ore", 2));
            s.temperature = 500; s.rate = 1; s.progress[0] = 50;
            assert!(s.tank.insert("copper", occupied));
            let primary = FluidAmount { id: "copper".into(), tag: None, amount: 0 };
            let extra = [FluidAmount { id: "gold".into(), tag: None, amount: 30 }];
            assert!(s.heat_foundry_slot(0, 500, 50, &primary, &extra));
            let mut s = Smeltery::from_nbt(&s.to_nbt()).unwrap();
            assert_eq!(s.tank.amount("gold"), expected_gold);
            assert_eq!(s.slots[0].as_ref().unwrap().count, 1);
            assert!(!s.heat_foundry_slot(0, 500, 50, &primary, &extra));
            assert_eq!(s.slots[0].as_ref().unwrap().count, 1);
        }
    }
    #[test]
    fn blocked_foundry_primary_preserves_input_and_retries_after_reload() {
        use super::super::tconstruct_source_smeltery_tank::FluidAmount;
        let mut s = Smeltery::new(); s.resize(1, 100);
        s.slots[0] = Some(ItemStack::new("minecraft:copper_ore", 1));
        s.temperature = 500; s.rate = 1; s.progress[0] = 50;
        assert!(s.tank.insert("iron", 90));
        let primary = FluidAmount { id: "copper".into(), tag: None, amount: 20 };
        let extra = [FluidAmount { id: "gold".into(), tag: None, amount: 30 }];
        assert!(!s.heat_foundry_slot(0, 500, 50, &primary, &extra));
        assert_eq!(s.progress[0], -1);
        assert_eq!(s.slots[0].as_ref().unwrap().count, 1);
        assert_eq!((s.tank.amount("iron"), s.tank.amount("copper"), s.tank.amount("gold")), (90, 0, 0));
        let mut s = Smeltery::from_nbt(&s.to_nbt()).unwrap();
        assert!(s.tank.take("iron", 40));
        assert!(s.heat_foundry_slot(0, 500, 50, &primary, &extra));
        assert!(s.slots[0].is_none());
        assert_eq!((s.tank.amount("iron"), s.tank.amount("copper"), s.tank.amount("gold")), (50, 20, 30));
    }
    #[test]
    fn foundry_consumes_input_after_primary_and_partial_byproduct_fill() {
        use super::super::tconstruct_source_smeltery_tank::FluidAmount;
        let mut s = Smeltery::new(); s.resize(1, 205);
        s.slots[0] = Some(ItemStack::new("minecraft:copper_ore", 1));
        s.temperature = 1000; s.rate = 10; s.progress[0] = 500;
        let primary = FluidAmount { id: "copper".into(), tag: None, amount: 180 };
        let extra = [FluidAmount { id: "gold".into(), tag: None, amount: 30 }];
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
        let f = |id: &str, amount| FluidAmount { id: id.into(), tag: None, amount };
        assert!(s.tank.alloy(&[f("tconstruct:molten_copper", 90), f("tconstruct:molten_amethyst", 100)], &f("tconstruct:molten_amethyst_bronze", 90)));
        assert_eq!(s.tank.amount("tconstruct:molten_amethyst_bronze"), 90);
        assert!(s.slots.iter().all(Option::is_none));
        assert_eq!(Smeltery::from_nbt(&s.to_nbt()).unwrap().tank, s.tank);
    }
}


#[cfg(test)]
mod tank_preservation_tests {
    use super::*;
    fn fixture() -> (Smeltery,Tag) {
        let mut state=Smeltery::new();state.resize(1,4000);state.slots[0]=Some(ItemStack::new("minecraft:stone",3));
        state.progress[0]=17;state.fuel=33;state.temperature=800;
        assert!(state.tank.insert("minecraft:water",2000));assert!(state.tank.insert("minecraft:lava",1000));
        let mut raw=state.to_nbt();if let Tag::Compound(fields)=&mut raw {
            fields.insert("future".into(),Tag::Int(7));fields.insert("heat_codes".into(),Tag::IntArray(vec![5]));
            if let Some(Tag::List(slots))=fields.get_mut(&"slots".into()){if let Tag::Compound(item)=&mut slots[0]{item.insert("future_item".into(),Tag::Int(9));}}
            if let Some(Tag::Compound(tank))=fields.get_mut(&"tank".into()){
                tank.insert("future_tank".into(),Tag::Int(8));
                if let Some(Tag::List(rows))=tank.get_mut(&"fluids".into()){for (index,row) in rows.iter_mut().enumerate(){if let Tag::Compound(row)=row{row.insert("future_row".into(),Tag::Int(index as i32));}}}
            }
        }
        (state,raw)
    }
    #[test]
    fn tank_only_selection_keeps_raw_controller_and_item_fields() {
        let (mut state,raw)=fixture();assert_eq!(state.tank_to_nbt_preserving(&raw),Some(raw.clone()));
        assert!(state.tank.move_to_bottom(1));assert!(state.tank.take("minecraft:lava",250));
        let saved=state.tank_to_nbt_preserving(&raw).unwrap();
        let Tag::Compound(fields)=&raw else{panic!()};for (key,value) in fields{if key.to_string_lossy()!="tank"{assert_eq!(saved.get(&key.to_string_lossy()),Some(value));}}
        let tank=saved.get("tank").unwrap();assert_eq!(tank.get("future_tank"),Some(&Tag::Int(8)));
        let Tag::List(rows)=tank.get("fluids").unwrap() else{panic!()};assert_eq!(rows[0].get("future_row"),Some(&Tag::Int(1)));
        assert_eq!(rows[0].get("amount"),Some(&Tag::Long(750)));assert_eq!(rows[1].get("future_row"),Some(&Tag::Int(0)));
    }
    #[test]
    fn tank_only_mode_processing_keeps_machine_inventory_separate() {
        use super::super::tconstruct_source_fluid_container::{Direction,transfer_selected_with_color};
        let (mut state,raw)=fixture();let mut containers=[Some(ItemStack::new("minecraft:water_bucket",1)),None];
        assert!(transfer_selected_with_color(&mut state.tank,&mut containers,Direction::Auto.next(),None,&|_|true,&|_|true,None,false));
        let saved=state.tank_to_nbt_preserving(&raw).unwrap();assert_eq!(saved.get("slots"),raw.get("slots"));
        assert_eq!(state.tank.amount("minecraft:water"),3000);assert!(containers[0].is_none());assert_eq!(&*containers[1].as_ref().unwrap().item,"minecraft:bucket");
    }
    #[test]
    fn tank_only_patch_refuses_unrelated_changes_aliases_and_opaque_removal() {
        for mode in 0..8{let(mut state,mut raw)=fixture();match mode{
            0=>state.slots[0].as_mut().unwrap().count+=1,
            1=>state.progress[0]+=1,
            2=>state.fuel+=1,
            3=>state.temperature+=1,
            4=>state.phase=1,
            5|6=>if let Tag::Compound(fields)=&mut raw{fields.insert(if mode==5{"Tank"}else{"Fluids"}.into(),Tag::Int(1));},
            7=>{assert!(state.tank.take("minecraft:lava",1000));},
            _=>unreachable!(),
        }assert!(state.tank_to_nbt_preserving(&raw).is_none());}
    }
}
