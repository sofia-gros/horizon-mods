//! The smeltery's tank (`SmelteryTank`): one capacity shared by any number of fluids, kept in the order they were first added. The first
//! fluid is the one that comes out when the tank is drained without saying which. Changes are recorded for the owner to send on.
use super::alloy::{AlloyTank, Fluid};
use crate::resource::ResourceId;

/// What happened to the tank, for the block entity to tell its clients and neighbours (`FluidChange`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    Added(Fluid),
    Changed(Fluid),
    Removed(Fluid),
    OrderChanged,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SmelteryTank {
    fluids: Vec<Fluid>,
    capacity: i32,
    contained: i32,
    changes: Vec<Change>,
    list_changed: bool,
}

impl SmelteryTank {
    pub fn new(capacity: i32) -> Self {
        Self { capacity, ..Self::default() }
    }

    pub fn fluids(&self) -> &[Fluid] {
        &self.fluids
    }
    pub fn capacity(&self) -> i32 {
        self.capacity
    }
    pub fn contained(&self) -> i32 {
        self.contained
    }
    pub fn set_capacity(&mut self, capacity: i32) {
        self.capacity = capacity;
    }

    pub fn remaining_space(&self) -> i32 {
        (self.capacity - self.contained).max(0)
    }

    /// The tanks a pipe sees: one for each fluid, and an empty one after them while there is room.
    pub fn tanks(&self) -> usize {
        self.fluids.len() + usize::from(self.contained < self.capacity)
    }

    pub fn fluid_in_tank(&self, index: usize) -> Option<&Fluid> {
        self.fluids.get(index)
    }

    /// How much tank `index` could hold: what it has plus the room left (the empty tank after the last has just the room).
    pub fn tank_capacity(&self, index: usize) -> i32 {
        let remaining = self.capacity - self.contained;
        match self.fluids.get(index) {
            Some(fluid) => fluid.amount + remaining,
            None if index == self.fluids.len() => remaining,
            None => 0,
        }
    }

    /// The changes since the last call, and whether the list of fluids (not only an amount) changed.
    pub fn take_changes(&mut self) -> (Vec<Change>, bool) {
        (std::mem::take(&mut self.changes), std::mem::take(&mut self.list_changed))
    }

    /// `moveFluidToBottom`: the fluid at `index` becomes the first, the one that comes out next.
    pub fn move_to_bottom(&mut self, index: usize) {
        if index < self.fluids.len() {
            let fluid = self.fluids.remove(index);
            self.fluids.insert(0, fluid);
            self.changes.push(Change::OrderChanged);
            self.list_changed = true;
        }
    }

    /// `fill`: as much as fits; with `execute` false only says how much would. Adds to the fluid already there or adds a new one last.
    pub fn fill(&mut self, fluid: &ResourceId, amount: i32, execute: bool) -> i32 {
        if self.contained >= self.capacity || amount <= 0 {
            return 0;
        }
        let usable = (self.capacity - self.contained).min(amount);
        if usable <= 0 || !execute {
            return usable.max(0);
        }
        self.contained += usable;
        if let Some(existing) = self.fluids.iter_mut().find(|f| f.id == *fluid) {
            existing.amount += usable;
            let now = existing.clone();
            self.changes.push(Change::Changed(now));
            if self.contained >= self.capacity {
                self.list_changed = true;
            }
            return usable;
        }
        let added = Fluid { id: fluid.clone(), amount: usable };
        self.fluids.push(added.clone());
        self.changes.push(Change::Added(added));
        self.list_changed = true;
        usable
    }

    fn take_from(&mut self, index: usize, max: i32, execute: bool) -> Fluid {
        let was_full = self.contained >= self.capacity;
        let drainable = max.min(self.fluids[index].amount);
        let out = Fluid { id: self.fluids[index].id.clone(), amount: drainable };
        if execute {
            self.fluids[index].amount -= drainable;
            self.contained -= drainable;
            if self.fluids[index].amount <= 0 {
                let removed = self.fluids.remove(index);
                self.changes.push(Change::Removed(removed));
                self.list_changed = true;
            } else {
                let now = self.fluids[index].clone();
                self.changes.push(Change::Changed(now));
                if was_full && self.contained < self.capacity {
                    self.list_changed = true;
                }
            }
        }
        out
    }

    /// `drain(maxDrain)`: up to `max` of the first fluid.
    pub fn drain_first(&mut self, max: i32, execute: bool) -> Option<Fluid> {
        if self.fluids.is_empty() {
            return None;
        }
        Some(self.take_from(0, max, execute))
    }

    /// `drain(fluid)`: up to `max` of that fluid.
    pub fn drain_fluid(&mut self, fluid: &ResourceId, max: i32, execute: bool) -> Option<Fluid> {
        let index = self.fluids.iter().position(|f| f.id == *fluid)?;
        Some(self.take_from(index, max, execute))
    }

    /// `setFluids`: replaces the contents (what the server sends a client); the order counts as changed when the first fluid is not the same.
    pub fn set_fluids(&mut self, fluids: Vec<Fluid>) {
        let old_first = self.fluids.first().map(|f| f.id.clone());
        self.contained = fluids.iter().map(|f| f.amount).sum();
        self.fluids = fluids;
        if old_first != self.fluids.first().map(|f| f.id.clone()) {
            self.changes.push(Change::OrderChanged);
            self.list_changed = true;
        }
    }

    /// `write` and `read`: the fluids and the capacity, as plain values.
    pub fn to_values(&self) -> (Vec<(String, i32)>, i32) {
        (self.fluids.iter().map(|f| (f.id.to_string(), f.amount)).collect(), self.capacity)
    }

    pub fn from_values(fluids: &[(String, i32)], capacity: i32) -> Self {
        let mut tank = Self::new(capacity);
        for (id, amount) in fluids {
            if let (Some(id), true) = (ResourceId::try_parse(id), *amount > 0) {
                tank.contained += amount;
                tank.fluids.push(Fluid { id, amount: *amount });
            }
        }
        tank
    }
}

/// A smeltery's tank as the alloy recipes see it (`SmelteryAlloyTank`), at the structure's temperature.
pub struct SmelteryAlloyTank<'a> {
    pub tank: &'a mut SmelteryTank,
    pub temperature: i32,
}

impl AlloyTank for SmelteryAlloyTank<'_> {
    fn tanks(&self) -> usize {
        self.tank.tanks()
    }
    fn fluid(&self, index: usize) -> Option<Fluid> {
        self.tank.fluid_in_tank(index).cloned()
    }
    fn temperature(&self) -> i32 {
        self.temperature
    }
    fn can_fit(&self, _fluid: &ResourceId, amount: i32, drained: i32) -> bool {
        amount - drained <= self.tank.remaining_space()
    }
    fn drain(&mut self, index: usize, amount: i32) -> i32 {
        let Some(id) = self.tank.fluid_in_tank(index).map(|f| f.id.clone()) else { return 0 };
        self.tank.drain_fluid(&id, amount, true).map_or(0, |f| f.amount)
    }
    fn fill(&mut self, fluid: &ResourceId, amount: i32) -> i32 {
        self.tank.fill(fluid, amount, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::smeltery::alloy::{AlloyRecipe, FluidTags};
    use serde_json::json;

    fn id(text: &str) -> ResourceId {
        ResourceId::try_parse(text).unwrap()
    }

    #[test]
    fn fluids_share_one_capacity_and_a_second_fill_adds_to_the_same_fluid() {
        let mut tank = SmelteryTank::new(1000);
        assert_eq!(tank.fill(&id("a:iron"), 600, true), 600);
        assert_eq!(tank.fill(&id("a:gold"), 600, true), 400, "only the room left fits");
        assert_eq!(tank.fill(&id("a:iron"), 10, true), 0, "full");
        assert_eq!((tank.contained(), tank.remaining_space(), tank.tanks()), (1000, 0, 2));
        assert_eq!(tank.fluids().iter().map(|f| f.amount).collect::<Vec<_>>(), vec![600, 400]);
    }

    #[test]
    fn simulating_a_fill_changes_nothing() {
        let mut tank = SmelteryTank::new(100);
        assert_eq!(tank.fill(&id("a:iron"), 500, false), 100);
        assert_eq!((tank.contained(), tank.fluids().len(), tank.take_changes()), (0, 0, (vec![], false)));
    }

    #[test]
    fn draining_without_a_fluid_takes_the_first_and_an_emptied_fluid_leaves_the_list() {
        let mut tank = SmelteryTank::new(1000);
        tank.fill(&id("a:iron"), 50, true);
        tank.fill(&id("a:gold"), 50, true);
        tank.take_changes();
        assert_eq!(tank.drain_first(80, true), Some(Fluid { id: id("a:iron"), amount: 50 }));
        assert_eq!(tank.fluids().len(), 1);
        let (changes, list) = tank.take_changes();
        assert!(list && matches!(changes.as_slice(), [Change::Removed(f)] if f.id == id("a:iron")));
        assert_eq!(tank.drain_fluid(&id("a:gold"), 20, true).map(|f| f.amount), Some(20));
        assert_eq!(tank.drain_fluid(&id("a:lead"), 20, true), None);
    }

    #[test]
    fn moving_a_fluid_to_the_bottom_makes_it_the_next_to_come_out_and_the_capacities_follow_the_room() {
        let mut tank = SmelteryTank::new(300);
        tank.fill(&id("a:iron"), 100, true);
        tank.fill(&id("a:gold"), 50, true);
        tank.move_to_bottom(1);
        assert_eq!(tank.fluid_in_tank(0).map(|f| f.id.to_string()), Some("a:gold".to_owned()));
        assert_eq!((tank.tank_capacity(0), tank.tank_capacity(2), tank.tank_capacity(3)), (50 + 150, 150, 0));
    }

    #[test]
    fn the_saved_form_round_trips_and_drops_empty_fluids() {
        let mut tank = SmelteryTank::new(500);
        tank.fill(&id("a:iron"), 120, true);
        let (fluids, capacity) = tank.to_values();
        let mut values = fluids.clone();
        values.push(("a:empty".to_owned(), 0));
        let loaded = SmelteryTank::from_values(&values, capacity);
        assert_eq!((loaded.contained(), loaded.capacity(), loaded.fluids().len()), (120, 500, 1));
    }

    #[test]
    fn an_alloy_recipe_runs_on_the_smeltery_tank() {
        struct Tags;
        impl FluidTags for Tags {
            fn members(&self, tag: &ResourceId) -> Vec<ResourceId> {
                match tag.to_string().as_str() {
                    "forge:molten_copper" => vec![id("a:copper")],
                    "forge:molten_tin" => vec![id("a:tin")],
                    "forge:molten_bronze" => vec![id("a:bronze")],
                    _ => vec![],
                }
            }
        }
        let recipe = AlloyRecipe::from_json(id("a:bronze"), &json!({"type": "tconstruct:alloy", "inputs": [{"amount": 270, "tag": "forge:molten_copper"}, {"amount": 90, "tag": "forge:molten_tin"}],
            "result": {"amount": 360, "tag": "forge:molten_bronze"}, "temperature": 700}), &crate::condition::EmptyContext).unwrap().unwrap();
        let mut tank = SmelteryTank::new(1000);
        tank.fill(&id("a:copper"), 300, true);
        tank.fill(&id("a:tin"), 100, true);
        let mut view = SmelteryAlloyTank { tank: &mut tank, temperature: 800 };
        assert!(recipe.perform(&mut view, &Tags));
        let amounts: Vec<(String, i32)> = tank.fluids().iter().map(|f| (f.id.to_string(), f.amount)).collect();
        assert_eq!(amounts, vec![("a:copper".to_owned(), 30), ("a:tin".to_owned(), 10), ("a:bronze".to_owned(), 360)]);
    }
}
