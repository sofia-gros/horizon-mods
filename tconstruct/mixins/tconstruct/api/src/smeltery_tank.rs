//! The smeltery's multi-fluid tank and its alloying, from SlimeKnights SmelteryTank, SmelteryAlloyTank and MultiAlloyingModule
//! (MIT; see LICENSE): one capacity shared by every fluid, fluids kept in the order they arrived (the first one is what comes
//! out of a drain), and alloy recipes run over the tank on the third tick of the cycle. Fluid stacks here have no NBT: the
//! original compares `isFluidEqual`, which includes it, and no shipped recipe or fuel gives its output any.
use crate::{fluid_ingredient::FluidTags, melting_recipes::{Alloy, AlloyTank}};
use pluto_lib::{Identifier, Nbt, NbtCompound, NbtCompoundExt};

const TAG_FLUIDS: &str = "fluids";
const TAG_CAPACITY: &str = "capacity";

/// One fluid in the tank, in millibuckets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Fluid { pub fluid: Identifier, pub amount: i32 }

impl Fluid {
    pub(crate) fn new(fluid: Identifier, amount: i32) -> Self { Self { fluid, amount } }
}

/// `ISmelteryTankHandler.FluidChange`: what the tank tells its block entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Change { Added, Changed, Removed, OrderChanged }

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SmelteryTank {
    fluids: Vec<Fluid>,
    capacity: i32,
    contained: i32,
    /// The changes since the owner last took them (`notifyFluidsChanged`).
    events: Vec<Change>,
}

impl SmelteryTank {
    pub(crate) fn fluids(&self) -> &[Fluid] { &self.fluids }
    pub(crate) fn contained(&self) -> i32 { self.contained }
    pub(crate) fn capacity(&self) -> i32 { self.capacity }
    pub(crate) fn set_capacity(&mut self, capacity: i32) { self.capacity = capacity; }

    /// What changed since the last call, oldest first.
    pub(crate) fn take_events(&mut self) -> Vec<Change> { std::mem::take(&mut self.events) }

    /// `getRemainingSpace`: the room left, never negative (the capacity can shrink under the contents).
    pub(crate) fn remaining_space(&self) -> i32 { if self.contained >= self.capacity { 0 } else { self.capacity - self.contained } }

    /// `getTanks`: one per fluid, plus one more for the empty space while there is some.
    pub(crate) fn tanks(&self) -> usize { self.fluids.len() + usize::from(self.contained < self.capacity) }

    pub(crate) fn fluid_in_tank(&self, tank: usize) -> Option<&Fluid> { self.fluids.get(tank) }

    /// `getTankCapacity`: the empty segment is the space left; a fluid's tank is its amount and the space.
    pub(crate) fn tank_capacity(&self, tank: usize) -> i32 {
        let remaining = self.capacity - self.contained;
        if tank == self.fluids.len() { return remaining; }
        self.fluids.get(tank).map_or(0, |fluid| fluid.amount + remaining)
    }

    /// `moveFluidToBottom`: the fluid at `index` becomes the first, the one a drain takes.
    pub(crate) fn move_fluid_to_bottom(&mut self, index: usize) {
        if index < self.fluids.len() {
            let fluid = self.fluids.remove(index);
            self.fluids.insert(0, fluid);
            self.events.push(Change::OrderChanged);
        }
    }

    /// `fill`: as much as fits in the shared capacity; adds to the fluid already in the tank, or becomes a new last one.
    pub(crate) fn fill(&mut self, fluid: &Identifier, amount: i32, execute: bool) -> i32 {
        if self.contained >= self.capacity || amount <= 0 { return 0; }
        // could be negative if the smeltery size changes then you try filling it
        let usable = (self.capacity - self.contained).min(amount);
        if usable <= 0 { return 0; }
        if !execute { return usable; }
        self.contained += usable;
        if let Some(held) = self.fluids.iter_mut().find(|held| &held.fluid == fluid) {
            held.amount += usable;
            self.events.push(Change::Changed);
            return usable;
        }
        self.fluids.push(Fluid::new(fluid.clone(), usable));
        self.events.push(Change::Added);
        usable
    }

    fn take(&mut self, index: usize, wanted: i32, execute: bool) -> Option<Fluid> {
        let held = &self.fluids[index];
        let drainable = wanted.min(held.amount);
        if drainable <= 0 { return None; }
        let drained = Fluid::new(held.fluid.clone(), drainable);
        if execute {
            self.fluids[index].amount -= drainable;
            self.contained -= drainable;
            if self.fluids[index].amount <= 0 {
                self.fluids.remove(index);
                self.events.push(Change::Removed);
            } else {
                self.events.push(Change::Changed);
            }
        }
        Some(drained)
    }

    /// `drain(maxDrain)`: from the first fluid only.
    pub(crate) fn drain(&mut self, max: i32, execute: bool) -> Option<Fluid> {
        if self.fluids.is_empty() { return None; }
        self.take(0, max, execute)
    }

    /// `drain(FluidStack)`: from the named fluid, wherever it is in the tank.
    pub(crate) fn drain_fluid(&mut self, fluid: &Identifier, amount: i32, execute: bool) -> Option<Fluid> {
        let index = self.fluids.iter().position(|held| &held.fluid == fluid)?;
        self.take(index, amount, execute)
    }

    /// `setFluids`, from the client's sync packet: replaces the list; the first fluid changing is an order change.
    pub(crate) fn set_fluids(&mut self, fluids: Vec<Fluid>) {
        let old_first = self.fluids.first().map(|fluid| fluid.fluid.clone());
        self.fluids = fluids;
        self.contained = self.fluids.iter().map(|fluid| fluid.amount).fold(0, i32::wrapping_add);
        if old_first != self.fluids.first().map(|fluid| fluid.fluid.clone()) { self.events.push(Change::OrderChanged); }
    }

    /// `write`: the fluids in order and the capacity (Forge's `FluidName` and `Amount`).
    pub(crate) fn write(&self) -> NbtCompound {
        let list = self.fluids.iter().map(|fluid| {
            let mut tag = NbtCompound::new();
            tag.insert("FluidName".to_owned(), Nbt::String(fluid.fluid.to_string()));
            tag.insert("Amount".to_owned(), Nbt::Int(fluid.amount));
            Nbt::Compound(tag)
        }).collect();
        let mut tag = NbtCompound::new();
        tag.insert(TAG_FLUIDS.to_owned(), Nbt::List(list));
        tag.insert(TAG_CAPACITY.to_owned(), Nbt::Int(self.capacity));
        tag
    }

    /// `read`: an entry with no known fluid or no amount is dropped.
    pub(crate) fn read(&mut self, tag: &NbtCompound) {
        self.fluids.clear();
        self.contained = 0;
        for entry in tag.get(TAG_FLUIDS).and_then(Nbt::as_list).unwrap_or(&[]).iter().filter_map(Nbt::as_compound) {
            let amount = entry.get_int("Amount", 0) as i32;
            let Some(fluid) = entry.get_string("FluidName").and_then(|name| Identifier::parse(name).ok()) else { continue };
            if amount > 0 && fluid.to_string() != "minecraft:empty" {
                self.contained += amount;
                self.fluids.push(Fluid::new(fluid, amount));
            }
        }
        self.capacity = tag.get_int(TAG_CAPACITY, 0) as i32;
    }
}

/// `SmelteryAlloyTank`: the tank as an alloy recipe sees it, at a temperature the caller sets from the fuel.
pub(crate) struct SmelteryAlloy<'a> { pub tank: &'a mut SmelteryTank, pub temperature: i32 }

impl AlloyTank for SmelteryAlloy<'_> {
    fn tanks(&self) -> usize { self.tank.tanks() }
    fn fluid_in_tank(&self, tank: usize) -> Option<(Identifier, i32)> { self.tank.fluid_in_tank(tank).map(|fluid| (fluid.fluid.clone(), fluid.amount)) }
    fn temperature(&self) -> i32 { self.temperature }
    /// The fluid fits if the net gain fits in the empty space.
    fn can_fit(&self, output: &(Identifier, i32), removed: i32) -> bool { output.1 - removed <= self.tank.remaining_space() }
    /// The smeltery drains the fluid wherever it is, whatever the index says.
    fn drain(&mut self, _: usize, fluid: &Identifier, amount: i32) -> i32 { self.tank.drain_fluid(fluid, amount, true).map_or(0, |drained| drained.amount) }
    fn fill(&mut self, fluid: &Identifier, amount: i32) -> i32 { self.tank.fill(fluid, amount, true) }
}

/// `MultiAlloyingModule`: the alloy recipes whose fluids are all in the tank, remembered until the tank gains a fluid, run
/// together every cycle. A recipe that stops matching leaves the list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Alloying { last: Option<Vec<usize>> }

impl Alloying {
    /// `clearCachedRecipes`, called when the tank gains a new fluid.
    pub(crate) fn clear_cached_recipes(&mut self) { self.last = None; }

    fn recipes(&mut self, alloys: &[Alloy], tank: &SmelteryAlloy<'_>, tags: &FluidTags) -> &mut Vec<usize> {
        self.last.get_or_insert_with(|| {
            let in_tag = |fluid: &Identifier, tag: &Identifier| tags.contains(fluid, tag);
            alloys.iter().enumerate().filter(|(_, alloy)| alloy.matches(tank, &in_tag)).map(|(index, _)| index).collect()
        })
    }

    /// `iterateRecipes`: runs `visit` on each recipe that still matches, dropping those that do not; true as soon as one answers true.
    fn iterate(&mut self, alloys: &[Alloy], tank: &mut SmelteryAlloy<'_>, tags: &FluidTags, visit: &mut dyn FnMut(&Alloy, &mut SmelteryAlloy<'_>) -> bool) -> bool {
        if self.recipes(alloys, tank, tags).is_empty() { return false; }
        let in_tag = |fluid: &Identifier, tag: &Identifier| tags.contains(fluid, tag);
        let list = self.last.as_mut().expect("filled above");
        let mut index = 0;
        while index < list.len() {
            let alloy = &alloys[list[index]];
            if alloy.matches(&*tank, &in_tag) {
                if visit(alloy, tank) { return true; }
                index += 1;
            } else {
                list.remove(index);
            }
        }
        false
    }

    /// `canAlloy`: whether any matching recipe could run now (temperature, amounts and room).
    pub(crate) fn can_alloy(&mut self, alloys: &[Alloy], tank: &mut SmelteryAlloy<'_>, tags: &FluidTags) -> bool {
        self.iterate(alloys, tank, tags, &mut |alloy, tank| alloy.can_perform(&*tank, tags))
    }

    /// `doAlloy`: shuffles the cached recipes (so mutually exclusive ones do not depend on name order) and runs every one that can.
    /// `random(bound)` is `Random.nextInt(bound)`.
    pub(crate) fn do_alloy(&mut self, alloys: &[Alloy], tank: &mut SmelteryAlloy<'_>, tags: &FluidTags, random: &mut dyn FnMut(usize) -> usize) {
        let list = self.recipes(alloys, tank, tags);
        if list.is_empty() { return; }
        // Collections.shuffle
        for i in (2..=list.len()).rev() { list.swap(i - 1, random(i)); }
        self.iterate(alloys, tank, tags, &mut |alloy, tank| { alloy.perform(tank, tags); false });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }

    fn tank(capacity: i32) -> SmelteryTank {
        let mut tank = SmelteryTank::default();
        tank.set_capacity(capacity);
        tank
    }

    #[test]
    fn fluids_share_one_capacity_and_come_out_in_arrival_order() {
        let mut tank = tank(1000);
        assert_eq!(tank.tanks(), 1, "only the empty space");
        assert_eq!(tank.fill(&id("a:iron"), 600, false), 600);
        assert_eq!(tank.fluids().len(), 0, "a simulated fill changes nothing");
        assert_eq!(tank.fill(&id("a:iron"), 600, true), 600);
        assert_eq!(tank.fill(&id("a:gold"), 600, true), 400, "only what still fits");
        assert_eq!(tank.fill(&id("a:copper"), 10, true), 0, "full");
        assert_eq!((tank.tanks(), tank.contained(), tank.remaining_space()), (2, 1000, 0), "no empty segment when full");
        assert_eq!(tank.take_events(), vec![Change::Added, Change::Added]);
        // a drain takes the first fluid, a typed drain the named one
        assert_eq!(tank.drain(250, true), Some(Fluid::new(id("a:iron"), 250)));
        assert_eq!(tank.drain_fluid(&id("a:gold"), 1000, false), Some(Fluid::new(id("a:gold"), 400)), "a look does not drain");
        assert_eq!(tank.drain_fluid(&id("a:gold"), 1000, true), Some(Fluid::new(id("a:gold"), 400)));
        assert_eq!(tank.fluids(), &[Fluid::new(id("a:iron"), 350)]);
        assert_eq!(tank.take_events(), vec![Change::Changed, Change::Removed]);
        assert_eq!(tank.drain_fluid(&id("a:gold"), 1, true), None);
        // iron 350 of 1000: its own tank is its amount plus the space, the empty segment is the space
        assert_eq!((tank.tank_capacity(0), tank.tank_capacity(1), tank.tank_capacity(2)), (1000, 650, 0));
        assert_eq!(tank.drain(1000, true), Some(Fluid::new(id("a:iron"), 350)));
        assert_eq!(tank.drain(10, true), None);
    }

    #[test]
    fn a_shrunk_capacity_leaves_no_room_and_no_negative_space() {
        let mut tank = tank(1000);
        tank.fill(&id("a:iron"), 900, true);
        tank.set_capacity(500);
        assert_eq!((tank.remaining_space(), tank.fill(&id("a:iron"), 10, true), tank.tanks()), (0, 0, 1));
        assert_eq!(tank.contained(), 900);
        tank.drain(500, true);
        assert_eq!(tank.remaining_space(), 100);
    }

    #[test]
    fn the_first_fluid_can_be_changed_by_a_click_and_the_tank_saves_and_reloads() {
        let mut tank = tank(2000);
        tank.fill(&id("a:iron"), 100, true);
        tank.fill(&id("a:gold"), 200, true);
        tank.take_events();
        tank.move_fluid_to_bottom(1);
        assert_eq!(tank.fluids().iter().map(|fluid| fluid.fluid.to_string()).collect::<Vec<_>>(), ["a:gold", "a:iron"]);
        assert_eq!(tank.take_events(), vec![Change::OrderChanged]);
        tank.move_fluid_to_bottom(7);
        assert!(tank.take_events().is_empty(), "no such fluid");
        let saved = tank.write();
        let mut loaded = SmelteryTank::default();
        loaded.read(&saved);
        assert_eq!((loaded.fluids(), loaded.contained(), loaded.capacity()), (tank.fluids(), 300, 2000));
        // the client's packet replaces the list; a different first fluid is an order change
        let mut client = SmelteryTank::default();
        client.set_fluids(tank.fluids().to_vec());
        assert_eq!((client.contained(), client.take_events()), (300, vec![Change::OrderChanged]));
        client.set_fluids(vec![Fluid::new(id("a:gold"), 50), Fluid::new(id("a:iron"), 50)]);
        assert!(client.take_events().is_empty());
        // bad entries are dropped on reading
        let mut bad = NbtCompound::new();
        bad.insert("fluids".to_owned(), Nbt::List(vec![Nbt::Compound(NbtCompound::new())]));
        loaded.read(&bad);
        assert!(loaded.fluids().is_empty());
    }

    fn alloy(name: &str, temperature: i32, inputs: serde_json::Value, result: serde_json::Value) -> Alloy {
        let fields = json!({"type": "tconstruct:alloy", "temperature": temperature, "inputs": inputs, "result": result});
        Alloy::decode(&id(name), fields.as_object().unwrap()).unwrap()
    }

    #[test]
    fn alloys_run_when_the_fuel_is_hot_enough_and_the_cache_is_dropped_when_a_fluid_is_added() {
        let alloys = vec![
            alloy("a:bronze", 800, json!([{"amount": 30, "fluid": "a:copper"}, {"amount": 10, "fluid": "a:tin"}]), json!({"amount": 40, "fluid": "a:bronze"})),
            alloy("a:rose", 900, json!([{"amount": 10, "fluid": "a:copper"}, {"amount": 10, "fluid": "a:gold"}]), json!({"amount": 20, "fluid": "a:rose"})),
        ];
        let tags = FluidTags::default();
        let mut tank = tank(1000);
        tank.fill(&id("a:copper"), 100, true);
        tank.fill(&id("a:tin"), 100, true);
        let mut module = Alloying::default();
        let mut cold = SmelteryAlloy { tank: &mut tank, temperature: 700 };
        assert!(!module.can_alloy(&alloys, &mut cold, &tags), "too cold");
        let mut warm = SmelteryAlloy { tank: &mut tank, temperature: 800 };
        assert!(module.can_alloy(&alloys, &mut warm, &tags));
        let mut zero = |_: usize| 0;
        module.do_alloy(&alloys, &mut warm, &tags, &mut zero);
        assert_eq!(tank.fluids(), &[Fluid::new(id("a:copper"), 70), Fluid::new(id("a:tin"), 90), Fluid::new(id("a:bronze"), 40)]);
        // gold arrives: the cached list lacks the rose recipe until the tank says it gained a fluid
        tank.fill(&id("a:gold"), 50, true);
        let mut hot = SmelteryAlloy { tank: &mut tank, temperature: 1000 };
        module.do_alloy(&alloys, &mut hot, &tags, &mut zero);
        assert!(!tank.fluids().iter().any(|fluid| fluid.fluid == id("a:rose")), "the recipe list was cached before gold was there");
        module.clear_cached_recipes();
        let mut hot = SmelteryAlloy { tank: &mut tank, temperature: 1000 };
        module.do_alloy(&alloys, &mut hot, &tags, &mut zero);
        let rose = tank.fluids().iter().find(|fluid| fluid.fluid == id("a:rose")).expect("rose gold");
        assert_eq!(rose.amount, 20);
        assert_eq!(tank.contained(), tank.fluids().iter().map(|fluid| fluid.amount).sum::<i32>());
        // a recipe that no longer matches leaves the cached list
        tank.drain_fluid(&id("a:gold"), 1000, true);
        let mut hot = SmelteryAlloy { tank: &mut tank, temperature: 1000 };
        module.can_alloy(&alloys, &mut hot, &tags);
        assert!(!module.last.as_ref().unwrap().contains(&1), "rose gold lost its gold and left the list");
    }

    #[test]
    fn the_output_must_fit_the_net_gain_and_the_shuffle_is_collections_shuffle() {
        let mut tank = tank(60);
        tank.fill(&id("a:one"), 30, true);
        tank.fill(&id("a:two"), 30, true);
        let alloys = vec![alloy("a:x", 1, json!([{"amount": 30, "fluid": "a:one"}, {"amount": 30, "fluid": "a:two"}]), json!({"amount": 60, "fluid": "a:three"}))];
        let mut module = Alloying::default();
        let mut hot = SmelteryAlloy { tank: &mut tank, temperature: 10 };
        assert!(module.can_alloy(&alloys, &mut hot, &FluidTags::default()), "60 made from 60 is no net gain, a full tank is fine");
        // shuffling three entries with draws 1 then 0 swaps (2,1) then (1,0)
        let mut list = vec![0usize, 1, 2];
        let mut draws = [1usize, 0].into_iter();
        for i in (2..=list.len()).rev() { list.swap(i - 1, draws.next().unwrap()); }
        assert_eq!(list, vec![2, 0, 1]);
    }
}
