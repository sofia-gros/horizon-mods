//! The heating rules of the melter, smeltery and foundry, from SlimeKnights MeltingModule, FuelModule,
//! SolidFuelModule, SmelteryBlockEntity, FoundryBlockEntity and MelterBlockEntity (MIT; see LICENSE): how an item
//! slot heats and cools, how fuel is consumed, and the capacity and fuel cost a structure's size gives.
//! These are the state machines only. Finding recipes, moving fluids, the world ticker and saving are host work.
#![allow(dead_code)]
use crate::fluid_values::INGOT;

/// MeltingModule.NO_SPACE: the item finished heating but its fluid did not fit.
pub(crate) const NO_SPACE: i32 = -1;

/// What the output tank did with a finished item (MeltingModule.onItemFinishedHeating).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Finished {
    /// No recipe matches the item any more: the slot clears.
    NoRecipe,
    /// The fluid went into the tank: the slot empties.
    Filled,
    /// The tank had no room: the slot keeps the item and waits.
    NoSpace,
}

/// One slot of a melting inventory: time in the same unit as the fuel rate, a recipe's `time` times 10.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct MeltingSlot { pub occupied: bool, pub current_time: i32, pub required_time: i32, pub required_temp: i32 }

impl MeltingSlot {
    fn reset(&mut self) { self.current_time = 0; self.required_time = 0; self.required_temp = 0; }

    /// setStack: `recipe` is the (time, temperature) of the recipe that matches the new item, if any;
    /// `stacks_with_old` says the new item is the same kind as the one already there, which keeps progress.
    pub(crate) fn set_stack(&mut self, item: Option<(Option<(i32, i32)>, bool)>) {
        match item {
            None => { self.occupied = false; self.reset(); }
            Some((recipe, stacks_with_old)) => {
                if !self.occupied || !stacks_with_old { self.current_time = 0; }
                self.occupied = true;
                let (time, temperature) = recipe.unwrap_or((0, 0));
                self.required_time = time.wrapping_mul(10);
                self.required_temp = temperature;
            }
        }
    }

    /// canHeatItem: a recipe is set, the slot is not waiting for space, and the structure is hot enough.
    pub(crate) fn can_heat(&mut self, temperature: i32) -> bool {
        if self.required_time > 0 {
            if !self.occupied { self.reset(); return false; }
            return self.current_time != NO_SPACE && temperature >= self.required_temp;
        }
        false
    }

    fn finish(&mut self, finished: &mut dyn FnMut() -> Finished) -> bool {
        match finished() {
            Finished::NoRecipe => true,
            Finished::Filled => { self.occupied = false; self.reset(); true }
            Finished::NoSpace => { self.current_time = NO_SPACE; false }
        }
    }

    /// heatItem: once the time is reached the item finishes, otherwise progress grows by the fuel's rate.
    pub(crate) fn heat(&mut self, temperature: i32, rate: i32, finished: &mut dyn FnMut() -> Finished) {
        if self.current_time == NO_SPACE || self.can_heat(temperature) {
            if self.current_time == NO_SPACE || self.current_time >= self.required_time {
                if self.finish(finished) { self.reset(); }
            } else {
                self.current_time = self.current_time.wrapping_add(rate);
            }
        }
    }

    /// coolItem: a finished item that had no space tries again; a partly heated item loses 5 a pass.
    pub(crate) fn cool(&mut self, finished: &mut dyn FnMut() -> Finished) {
        if self.current_time == NO_SPACE {
            if self.finish(finished) { self.reset(); }
        } else if self.current_time > 0 && self.required_time > 0 {
            self.current_time -= 5;
        }
    }
}

/// FuelModule state: `fuel` counts down by the structure's fuel rate; quality, temperature and rate describe the
/// last source that was burned.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Fuel { pub fuel: i32, pub quality: i32, pub temperature: i32, pub rate: i32 }

impl Fuel {
    pub(crate) fn has_fuel(&self) -> bool { self.fuel > 0 }
    pub(crate) fn decrease(&mut self, amount: i32) { self.fuel = (self.fuel - amount).max(0); }

    /// tryLiquidFuel: a liquid fuel needs `amount` of the fluid; on `consume` the caller drains that much and the
    /// fuel gains the recipe's duration. Returns the fuel temperature, 0 when the tank holds too little.
    pub(crate) fn try_liquid(&mut self, available: i32, amount: i32, duration: i32, temperature: i32, rate: i32, consume: bool) -> i32 {
        if available < amount { return 0; }
        if consume { self.burn(duration, temperature, rate); }
        temperature
    }

    /// trySolidFuel: one item of burn time `burn_ticks` (the vanilla fuel time) gives a quarter of it, at the
    /// temperature and rate of the shipped solid fuel recipe. Returns that temperature.
    pub(crate) fn try_solid(&mut self, burn_ticks: i32, temperature: i32, rate: i32, consume: bool) -> i32 {
        let time = burn_ticks / 4;
        if time <= 0 { return 0; }
        if consume { self.burn(time, temperature, rate); }
        temperature
    }

    fn burn(&mut self, duration: i32, temperature: i32, rate: i32) {
        self.fuel += duration;
        self.quality = duration;
        self.temperature = temperature;
        self.rate = rate;
    }

    /// findFuel(true) found nothing: the client is told there is no heat.
    pub(crate) fn none_found(&mut self) { self.temperature = 0; self.rate = 0; }
}

/// A melting inventory is capped at this many slots (MeltingModuleInventory.MAX_SIZE).
pub(crate) const MAX_MELTING_SLOTS: i32 = (i16::MAX as i32 - 7) / 3;

/// The melter: one tank of 24 ingots and three item slots.
pub(crate) const MELTER_CAPACITY: i32 = INGOT * 24;
pub(crate) const MELTER_SLOTS: i32 = 3;

/// What an inner size of a heating structure gives: tank capacity, melting slots and the fuel used each pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Structure { pub capacity: i32, pub slots: i32, pub fuel_rate: i32 }

/// SmelteryBlockEntity.setStructure: 12 ingots per inner block; every 15 blocks of the walls and floor adds 1 to the fuel rate.
pub(crate) fn smeltery(dx: i32, dy: i32, dz: i32) -> Structure {
    Structure {
        capacity: INGOT * 12 * dx * dy * dz,
        slots: (dx * dy * dz).min(MAX_MELTING_SLOTS),
        fuel_rate: 1 + (2 * (dx * dy) + 2 * (dy * dz) + dx * dz) / 15,
    }
}

/// FoundryBlockEntity.setStructure: the tank counts the walls and floor too, at 8 ingots a block; every 18 blocks adds 1 to the fuel rate.
pub(crate) fn foundry(dx: i32, dy: i32, dz: i32) -> Structure {
    Structure {
        capacity: INGOT * 8 * (dx + 2) * (dy + 1) * (dz + 2),
        slots: (dx * dy * dz).min(MAX_MELTING_SLOTS),
        fuel_rate: 1 + (2 * ((dx + 2) * dy) + 2 * (dy * dz) + (dx + 2) * (dz + 2)) / 18,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A slot holding an iron ingot: recipe time 60, 800 C, so 600 required.
    fn iron() -> MeltingSlot {
        let mut slot = MeltingSlot::default();
        slot.set_stack(Some((Some((60, 800)), false)));
        slot
    }

    #[test]
    fn an_item_heats_by_the_fuel_rate_then_waits_for_room_and_cools_when_unfueled() {
        let mut slot = iron();
        assert_eq!((slot.required_time, slot.required_temp), (600, 800));
        assert!(!slot.can_heat(799) && slot.can_heat(800));
        // lava burns at rate 10: 60 passes of four ticks reach the time, the 61st finishes
        let mut full = || Finished::NoSpace;
        for _ in 0..60 { slot.heat(1000, 10, &mut full); }
        assert_eq!(slot.current_time, 600);
        slot.heat(1000, 10, &mut full);
        assert_eq!(slot.current_time, NO_SPACE, "the tank was full: the item waits");
        assert!(!slot.can_heat(1000), "a waiting slot is not offered more heat");
        // cooling retries the output; once there is room the slot empties
        let mut room = || Finished::Filled;
        slot.cool(&mut room);
        assert_eq!(slot, MeltingSlot::default());
    }

    #[test]
    fn a_partly_heated_item_loses_five_a_pass_and_a_new_kind_of_item_restarts() {
        let mut slot = iron();
        slot.heat(1000, 10, &mut || Finished::Filled);
        slot.heat(1000, 10, &mut || Finished::Filled);
        assert_eq!(slot.current_time, 20);
        slot.cool(&mut || Finished::Filled);
        assert_eq!(slot.current_time, 15);
        // the same kind keeps progress, a different one starts over, an empty slot forgets it all
        slot.set_stack(Some((Some((80, 900)), true)));
        assert_eq!((slot.current_time, slot.required_time), (15, 800));
        slot.set_stack(Some((None, false)));
        assert_eq!((slot.current_time, slot.required_time, slot.occupied), (0, 0, true));
        assert!(!slot.can_heat(2000), "an item with no recipe never heats");
    }

    #[test]
    fn fuel_is_burned_per_item_or_fluid_and_costs_by_structure_size() {
        let mut fuel = Fuel::default();
        // lava: 50 mB for a duration of 100 at 1000 C, rate 10; a tank of 49 is not enough
        assert_eq!(fuel.try_liquid(49, 50, 100, 1000, 10, true), 0);
        assert_eq!(fuel.try_liquid(50, 50, 100, 1000, 10, false), 1000);
        assert!(!fuel.has_fuel(), "a look does not burn");
        assert_eq!(fuel.try_liquid(50, 50, 100, 1000, 10, true), 1000);
        assert_eq!((fuel.fuel, fuel.quality, fuel.rate), (100, 100, 10));
        // coal burns 1600 ticks: a quarter is 400 passes, at the solid fuel's 800 C and rate 8
        assert_eq!(fuel.try_solid(1600, 800, 8, true), 800);
        assert_eq!((fuel.fuel, fuel.quality, fuel.rate), (500, 400, 8));
        assert_eq!(fuel.try_solid(3, 800, 8, true), 0, "less than four ticks of burn time is no fuel");
        fuel.decrease(1000);
        assert_eq!(fuel.fuel, 0);
        // a 3x3x3 smeltery holds 27 blocks of 12 ingots and burns 1 + (18 + 18 + 9) / 15 = 4 a pass
        assert_eq!(smeltery(3, 3, 3), Structure { capacity: 29160, slots: 27, fuel_rate: 4 });
        // the foundry's tank includes the walls and floor: 8 * 90 * 5 * 4 * 5 and 1 + (30 + 18 + 25) / 18
        assert_eq!(foundry(3, 3, 3), Structure { capacity: 72000, slots: 27, fuel_rate: 5 });
    }
}
