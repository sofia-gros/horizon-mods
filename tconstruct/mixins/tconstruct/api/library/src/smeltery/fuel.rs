//! Fuel (`MeltingFuel` and `FuelModule`): a heating structure burns fuel to stay hot. A fuel recipe says how much of a fluid is one
//! portion, how long a portion burns, how hot it makes the structure and how fast it heats. The state keeps what is burning now.
use serde_json::Value;

use super::alloy::FluidMatch;
use crate::resource::ResourceId;

/// A fuel recipe. A solid fuel recipe has no fluid; its duration comes from the item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeltingFuel {
    pub id: ResourceId,
    /// The fluid and the amount that is one portion; None for the solid fuel recipe.
    pub input: Option<(FluidMatch, i32)>,
    pub duration: i32,
    pub temperature: i32,
    pub rate: i32,
}

impl MeltingFuel {
    pub fn from_json(id: ResourceId, json: &Value) -> Result<Self, String> {
        let positive = |key: &str| json.get(key).and_then(Value::as_i64).filter(|v| *v >= 1).map(|v| v as i32);
        let input = match json.get("fluid") {
            None => None,
            Some(fluid) => {
                let kind = if let Some(name) = fluid.get("fluid").and_then(Value::as_str) {
                    FluidMatch::Fluid(ResourceId::try_parse(name).ok_or("bad fluid id")?)
                } else {
                    FluidMatch::Tag(ResourceId::try_parse(fluid.get("tag").and_then(Value::as_str).ok_or("the fluid needs 'fluid' or 'tag'")?).ok_or("bad fluid tag id")?)
                };
                Some((kind, fluid.get("amount").and_then(Value::as_i64).filter(|a| *a >= 1).ok_or("the fluid needs a positive 'amount'")? as i32))
            }
        };
        let duration = positive("duration").unwrap_or(0);
        if input.is_some() && duration == 0 {
            return Err("Missing JSON field duration".to_owned());
        }
        Ok(Self { id, input, duration, temperature: positive("temperature").ok_or("a fuel needs a 'temperature' of at least 1")?, rate: positive("rate").ok_or("a fuel needs a 'rate' of at least 1")? })
    }
}

/// Finds the fuel recipe for a fluid and the portion size of it (`MeltingFuelLookup`).
pub trait FuelLookup {
    fn find(&self, fluid: &ResourceId) -> Option<(MeltingFuel, i32)>;
}

/// What is burning in a structure now (`FuelModule`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FuelState {
    pub fuel: i32,
    pub quality: i32,
    pub temperature: i32,
    pub rate: i32,
}

impl FuelState {
    pub fn has_fuel(&self) -> bool {
        self.fuel > 0
    }

    pub fn decrease(&mut self, amount: i32) {
        self.fuel = (self.fuel - amount).max(0);
    }

    /// `tryLiquidFuel` on the fluid in the first tank: the temperature the fuel gives (0 when there is no recipe or not a whole portion). With
    /// `consume`, `drain` takes the portion out of the tank and, when it did, the fuel is added and its quality, temperature and rate set.
    pub fn try_liquid_fuel(&mut self, tank: Option<(&ResourceId, i32)>, lookup: &dyn FuelLookup, consume: bool, drain: &mut dyn FnMut(i32) -> bool) -> i32 {
        let Some((fluid, amount)) = tank else { return 0 };
        let Some((recipe, portion)) = lookup.find(fluid) else { return 0 };
        if amount < portion {
            return 0;
        }
        if !consume {
            return recipe.temperature;
        }
        if !drain(portion) {
            return 0;
        }
        self.fuel += recipe.duration;
        self.quality = recipe.duration;
        self.temperature = recipe.temperature;
        self.rate = recipe.rate;
        self.temperature
    }

    /// Adds a solid fuel portion of `time` ticks with the solid fuel recipe's temperature and rate.
    pub fn add_solid(&mut self, time: i32, recipe: &MeltingFuel) {
        self.fuel += time;
        self.quality = time;
        self.temperature = recipe.temperature;
        self.rate = recipe.rate;
    }

    /// `writeToTag`.
    pub fn to_tag(&self) -> [(&'static str, i32); 3] {
        [("fuel", self.fuel), ("temperature", self.temperature), ("rate", self.rate)]
    }

    /// `readFromTag`: only the values present are changed, and the rate comes with the temperature.
    pub fn read_tag(&mut self, fuel: Option<i32>, temperature: Option<i32>, rate: Option<i32>) {
        if let Some(fuel) = fuel {
            self.fuel = fuel;
        }
        if let Some(temperature) = temperature {
            self.temperature = temperature;
            self.rate = rate.unwrap_or(0);
        }
    }
}

/// The fuel tanks of a structure, in the order the structure lists them (`MultitankFuelModule`'s tank supplier).
pub trait FuelTanks {
    fn count(&self) -> usize;
    /// The fluid in tank `index` and its amount, None when it is empty or not there.
    fn fluid(&self, index: usize) -> Option<(ResourceId, i32)>;
    /// Takes `amount` of the fluid out of tank `index`; true when it all came out.
    fn drain(&mut self, index: usize, amount: i32) -> bool;
}

/// `MultitankFuelModule`: finds fuel in the structure's tanks, trying the tank that gave fuel last time first so a tank is emptied before
/// the next is started.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MultitankFuel {
    last: Option<usize>,
}

impl MultitankFuel {
    fn try_tank(&mut self, index: usize, state: &mut FuelState, tanks: &mut dyn FuelTanks, lookup: &dyn FuelLookup, consume: bool) -> i32 {
        let Some((fluid, amount)) = tanks.fluid(index) else { return 0 };
        let mut taken = false;
        let temperature = {
            let tanks = &mut *tanks;
            state.try_liquid_fuel(Some((&fluid, amount)), lookup, consume, &mut |portion| {
                taken = tanks.drain(index, portion);
                taken
            })
        };
        if temperature > 0 {
            self.last = Some(index);
        }
        temperature
    }

    /// `findFuel`: the temperature of the fuel found (0 for none). With `consume`, a portion is taken and the state set; when none is
    /// found the state's temperature and rate go to 0.
    pub fn find_fuel(&mut self, state: &mut FuelState, tanks: &mut dyn FuelTanks, lookup: &dyn FuelLookup, consume: bool) -> i32 {
        if let Some(last) = self.last.filter(|l| *l < tanks.count()) {
            let temperature = self.try_tank(last, state, tanks, lookup, consume);
            if temperature > 0 {
                return temperature;
            }
        }
        for index in 0..tanks.count() {
            if Some(index) == self.last {
                continue;
            }
            let temperature = self.try_tank(index, state, tanks, lookup, consume);
            if temperature > 0 {
                return temperature;
            }
        }
        if consume {
            state.temperature = 0;
            state.rate = 0;
        }
        0
    }
}

/// The item slots of a heater (`IItemHandler`) as solid fuel sees them.
pub trait FuelSlots {
    fn count(&self) -> usize;
    fn stack(&self, index: usize) -> Option<(ResourceId, u32)>;
    /// The burn time in ticks of the item as furnace fuel (0 when it does not burn).
    fn burn_time(&self, item: &ResourceId) -> i32;
    /// Takes one item out of slot `index`, giving back what it leaves (a bucket for a lava bucket); true when one came out.
    fn extract_one(&mut self, index: usize) -> bool;
}

/// `SolidFuelModule.trySolidFuel`: the first item that burns gives a quarter of its furnace burn time as fuel at the solid fuel recipe's
/// temperature and rate. Returns the temperature (0 when nothing burns).
pub fn try_solid_fuel(state: &mut FuelState, slots: &mut dyn FuelSlots, solid: &MeltingFuel, consume: bool) -> i32 {
    for index in 0..slots.count() {
        let Some((item, _)) = slots.stack(index) else { continue };
        let time = slots.burn_time(&item) / 4;
        if time > 0 {
            if consume && slots.extract_one(index) {
                state.add_solid(time, solid);
            }
            return solid.temperature;
        }
    }
    0
}

/// `SolidFuelModule.findFuel`: the heater's tank first, then its items; when nothing is found and fuel is consumed, the heat goes to 0.
pub fn find_heater_fuel(state: &mut FuelState, tank: Option<(&ResourceId, i32)>, lookup: &dyn FuelLookup, drain: &mut dyn FnMut(i32) -> bool, slots: &mut dyn FuelSlots, solid: &MeltingFuel, consume: bool) -> i32 {
    let mut temperature = state.try_liquid_fuel(tank, lookup, consume, drain);
    if temperature == 0 {
        temperature = try_solid_fuel(state, slots, solid, consume);
    }
    if temperature == 0 && consume {
        state.temperature = 0;
        state.rate = 0;
    }
    temperature
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn id(text: &str) -> ResourceId {
        ResourceId::try_parse(text).unwrap()
    }

    struct Lava;
    impl FuelLookup for Lava {
        fn find(&self, fluid: &ResourceId) -> Option<(MeltingFuel, i32)> {
            let json = json!({"fluid": {"fluid": "minecraft:lava", "amount": 50}, "duration": 100, "temperature": 1000, "rate": 10});
            (fluid.to_string() == "minecraft:lava").then(|| (MeltingFuel::from_json(id("tconstruct:lava"), &json).unwrap(), 50))
        }
    }

    #[test]
    fn a_whole_portion_of_lava_adds_its_duration_and_sets_the_heat() {
        let mut state = FuelState::default();
        let lava = id("minecraft:lava");
        let mut drained = 0;
        let temperature = state.try_liquid_fuel(Some((&lava, 120)), &Lava, true, &mut |a| {
            drained += a;
            true
        });
        assert_eq!((temperature, drained), (1000, 50));
        assert_eq!(state, FuelState { fuel: 100, quality: 100, temperature: 1000, rate: 10 });
        assert!(state.has_fuel());
    }

    #[test]
    fn asking_without_consuming_changes_nothing_and_a_partial_portion_gives_nothing() {
        let mut state = FuelState::default();
        let lava = id("minecraft:lava");
        assert_eq!(state.try_liquid_fuel(Some((&lava, 50)), &Lava, false, &mut |_| panic!("not consumed")), 1000);
        assert_eq!(state, FuelState::default());
        assert_eq!(state.try_liquid_fuel(Some((&lava, 49)), &Lava, true, &mut |_| true), 0);
        assert_eq!(state.try_liquid_fuel(Some((&id("minecraft:water"), 500)), &Lava, true, &mut |_| true), 0);
        assert_eq!(state.try_liquid_fuel(None, &Lava, true, &mut |_| true), 0);
    }

    #[test]
    fn fuel_burns_down_to_zero_and_the_tag_round_trips() {
        let mut state = FuelState { fuel: 30, quality: 100, temperature: 1000, rate: 10 };
        state.decrease(50);
        assert_eq!(state.fuel, 0);
        assert!(!state.has_fuel());
        let mut loaded = FuelState::default();
        let tag = FuelState { fuel: 7, temperature: 800, rate: 4, ..FuelState::default() }.to_tag();
        loaded.read_tag(Some(tag[0].1), Some(tag[1].1), Some(tag[2].1));
        assert_eq!(loaded, FuelState { fuel: 7, quality: 0, temperature: 800, rate: 4 });
    }

    struct Tanks(Vec<Option<(ResourceId, i32)>>);
    impl FuelTanks for Tanks {
        fn count(&self) -> usize {
            self.0.len()
        }
        fn fluid(&self, index: usize) -> Option<(ResourceId, i32)> {
            self.0[index].clone()
        }
        fn drain(&mut self, index: usize, amount: i32) -> bool {
            match self.0[index].as_mut() {
                Some((_, have)) if *have >= amount => {
                    *have -= amount;
                    true
                }
                _ => false,
            }
        }
    }

    #[test]
    fn several_tanks_are_tried_in_order_and_the_one_that_gave_fuel_last_is_tried_first() {
        let mut tanks = Tanks(vec![Some((id("minecraft:water"), 500)), Some((id("minecraft:lava"), 120)), Some((id("minecraft:lava"), 500))]);
        let (mut state, mut finder) = (FuelState::default(), MultitankFuel::default());
        assert_eq!(finder.find_fuel(&mut state, &mut tanks, &Lava, true), 1000, "the water is not fuel; the second tank is");
        assert_eq!(tanks.0[1].as_ref().map(|t| t.1), Some(70));
        // the next time the same tank is used first, while it still holds a portion
        finder.find_fuel(&mut state, &mut tanks, &Lava, true);
        assert_eq!((tanks.0[1].as_ref().map(|t| t.1), tanks.0[2].as_ref().map(|t| t.1)), (Some(20), Some(500)));
        // 20 is under a portion, so the third tank gives
        finder.find_fuel(&mut state, &mut tanks, &Lava, true);
        assert_eq!(tanks.0[2].as_ref().map(|t| t.1), Some(450));
    }

    #[test]
    fn with_no_fuel_the_heat_goes_to_zero_only_when_fuel_is_consumed() {
        let mut tanks = Tanks(vec![Some((id("minecraft:water"), 500)), None]);
        let mut state = FuelState { fuel: 0, quality: 0, temperature: 800, rate: 5 };
        let mut finder = MultitankFuel::default();
        assert_eq!(finder.find_fuel(&mut state, &mut tanks, &Lava, false), 0);
        assert_eq!((state.temperature, state.rate), (800, 5));
        assert_eq!(finder.find_fuel(&mut state, &mut tanks, &Lava, true), 0);
        assert_eq!((state.temperature, state.rate), (0, 0));
    }

    struct Slots(Vec<Option<(ResourceId, u32)>>);
    impl FuelSlots for Slots {
        fn count(&self) -> usize {
            self.0.len()
        }
        fn stack(&self, index: usize) -> Option<(ResourceId, u32)> {
            self.0[index].clone()
        }
        fn burn_time(&self, item: &ResourceId) -> i32 {
            match item.to_string().as_str() {
                "minecraft:coal" => 1600,
                "minecraft:stick" => 100,
                _ => 0,
            }
        }
        fn extract_one(&mut self, index: usize) -> bool {
            let Some((_, count)) = self.0[index].as_mut() else { return false };
            *count -= 1;
            if *count == 0 {
                self.0[index] = None;
            }
            true
        }
    }

    #[test]
    fn solid_fuel_burns_a_quarter_of_the_furnace_time_and_the_tank_goes_first() {
        let solid = MeltingFuel::from_json(id("tconstruct:solid"), &json!({"temperature": 800, "rate": 5})).unwrap();
        let mut slots = Slots(vec![Some((id("minecraft:dirt"), 3)), Some((id("minecraft:coal"), 2))]);
        let mut state = FuelState::default();
        let lava = id("minecraft:lava");
        // an empty tank: the coal burns, 1600 / 4 = 400 ticks
        assert_eq!(find_heater_fuel(&mut state, None, &Lava, &mut |_| true, &mut slots, &solid, true), 800);
        assert_eq!((state.fuel, state.quality, state.rate, slots.0[1].as_ref().map(|s| s.1)), (400, 400, 5, Some(1)));
        // lava in the tank is used before the items
        let mut drained = 0;
        let hot = find_heater_fuel(&mut state, Some((&lava, 50)), &Lava, &mut |a| {
            drained = a;
            true
        }, &mut slots, &solid, true);
        assert_eq!((hot, drained, slots.0[1].as_ref().map(|s| s.1)), (1000, 50, Some(1)));
        // nothing burns: the heat goes to 0
        let mut nothing = Slots(vec![Some((id("minecraft:dirt"), 1))]);
        assert_eq!(find_heater_fuel(&mut state, None, &Lava, &mut |_| true, &mut nothing, &solid, true), 0);
        assert_eq!((state.temperature, state.rate), (0, 0));
        // a stick burns 100 / 4 = 25 ticks
        let mut sticks = Slots(vec![Some((id("minecraft:stick"), 1))]);
        let mut fresh = FuelState::default();
        try_solid_fuel(&mut fresh, &mut sticks, &solid, true);
        assert_eq!(fresh.fuel, 25);
    }

    #[test]
    fn a_liquid_fuel_needs_a_duration_and_a_solid_one_does_not() {
        let liquid = json!({"fluid": {"fluid": "minecraft:lava", "amount": 50}, "temperature": 1000, "rate": 10});
        assert_eq!(MeltingFuel::from_json(id("a:b"), &liquid), Err("Missing JSON field duration".to_owned()));
        let solid = json!({"temperature": 800, "rate": 5});
        assert_eq!(MeltingFuel::from_json(id("a:c"), &solid).unwrap().input, None);
    }
}
