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

    #[test]
    fn a_liquid_fuel_needs_a_duration_and_a_solid_one_does_not() {
        let liquid = json!({"fluid": {"fluid": "minecraft:lava", "amount": 50}, "temperature": 1000, "rate": 10});
        assert_eq!(MeltingFuel::from_json(id("a:b"), &liquid), Err("Missing JSON field duration".to_owned()));
        let solid = json!({"temperature": 800, "rate": 5});
        assert_eq!(MeltingFuel::from_json(id("a:c"), &solid).unwrap().input, None);
    }
}
