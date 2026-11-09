//! A heating structure's tick (`SmelteryBlockEntity.heat` and `HeatingStructureBlockEntity.cool`): what happens in each of the twenty ticks
//! of a second, put together from the parts: fuel, melting slots, the tank and alloying. Everything the world answers is in [`Rules`].
use super::alloy::{AlloyRecipe, AlloyTank, FluidMatch, FluidTags};
use super::fuel::FuelState;
use super::melting::{MeltingRecipe, MeltingRecipes, MeltingSlot, OreRate};
use super::tank::{SmelteryAlloyTank, SmelteryTank};
use crate::resource::ResourceId;

/// Where fuel comes from (`FuelModule.findFuel`): the structure's fuel tanks or the heater's item slots.
pub trait FuelSource {
    /// The temperature the next fuel would give (0 when there is none). With `consume`, takes a portion and sets the state.
    fn find_fuel(&self, state: &mut FuelState, consume: bool) -> i32;
}

/// What the structure needs from the game each tick.
pub struct Rules<'a> {
    pub melting: &'a dyn MeltingRecipes,
    pub alloys: &'a [AlloyRecipe],
    pub tags: &'a dyn FluidTags,
    pub fuel_source: &'a dyn FuelSource,
    pub ore_rate: OreRate,
}

/// A smeltery's working state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Heating {
    /// Counts 0 to 19 and starts over each second.
    pub tick: i32,
    pub fuel: FuelState,
    pub tank: SmelteryTank,
    pub slots: Vec<MeltingSlot>,
    /// How much fuel one four-tick round burns: bigger structures burn more.
    pub fuel_rate: i32,
    /// Only a structure with a fuel tank works; without one it only cools.
    pub has_tanks: bool,
    /// Whether the controller shows as burning.
    pub active: bool,
}

/// `BLOCKS_PER_FUEL`: every this many blocks of wall make the fuel cost one more.
pub const BLOCKS_PER_FUEL: i32 = 15;

impl Heating {
    /// `setStructure`: the tank holds `capacity_per_block` for each inner block, there is a slot for each, and the fuel rate follows the
    /// walls (two of the x and the z wall and one of the floor).
    pub fn resize(&mut self, dx: i32, dy: i32, dz: i32, capacity_per_block: i32) {
        self.tank.set_capacity(capacity_per_block * dx * dy * dz);
        // a smaller structure loses its last slots (the original drops their items; the caller must do that first)
        self.slots.resize((dx * dy * dz) as usize, MeltingSlot::default());
        self.fuel_rate = 1 + (2 * (dx * dy) + 2 * (dy * dz) + (dx * dz)) / BLOCKS_PER_FUEL;
    }

    /// `tryFillTank` and `handleByproducts`: the result goes in whole or not at all; then the byproducts, as far as there is room.
    fn take_result(tank: &mut SmelteryTank, recipe: &MeltingRecipe, rules: &Rules<'_>, resolve: &dyn Fn(&FluidMatch) -> Option<ResourceId>) -> bool {
        let Some(fluid) = resolve(&recipe.result.fluid) else { return false };
        let amount = recipe.output_amount(&rules.ore_rate);
        if tank.fill(&fluid, amount, false) != amount {
            return false;
        }
        tank.fill(&fluid, amount, true);
        for (byproduct, amount) in recipe.byproducts.iter().zip(recipe.byproduct_amounts(&OreRate::FOUNDRY_BYPRODUCT)) {
            if let Some(fluid) = resolve(&byproduct.fluid) {
                tank.fill(&fluid, amount, true);
            }
        }
        true
    }

    fn heat_slots(&mut self, temperature: i32, rate: i32, rules: &Rules<'_>) {
        let tags = rules.tags;
        let resolve = |m: &FluidMatch| match m {
            FluidMatch::Fluid(id) => Some(id.clone()),
            FluidMatch::Tag(tag) => tags.members(tag).into_iter().next(),
        };
        for slot in &mut self.slots {
            let tank = &mut self.tank;
            slot.heat(temperature, rate, rules.melting, &mut |recipe| Self::take_result(tank, recipe, rules, &resolve));
        }
    }

    fn cool_slots(&mut self, rules: &Rules<'_>) {
        let tags = rules.tags;
        let resolve = |m: &FluidMatch| match m {
            FluidMatch::Fluid(id) => Some(id.clone()),
            FluidMatch::Tag(tag) => tags.members(tag).into_iter().next(),
        };
        for slot in &mut self.slots {
            let tank = &mut self.tank;
            slot.cool(rules.melting, &mut |recipe| Self::take_result(tank, recipe, rules, &resolve));
        }
    }

    fn can_heat(&mut self, temperature: i32) -> bool {
        self.slots.iter_mut().any(|slot| slot.can_heat(temperature))
    }

    fn can_alloy(&mut self, temperature: i32, rules: &Rules<'_>) -> bool {
        let view = SmelteryAlloyTank { tank: &mut self.tank, temperature };
        rules.alloys.iter().any(|r| r.matches(&view, rules.tags) && r.can_perform(&view, rules.tags))
    }

    fn do_alloy(&mut self, temperature: i32, rules: &Rules<'_>) {
        // the original tries the matching recipes in a shuffled order and stops at the first that runs; here they go in the given order
        let mut view = SmelteryAlloyTank { tank: &mut self.tank, temperature };
        for recipe in rules.alloys {
            if recipe.matches(&view, rules.tags) && recipe.can_perform(&view, rules.tags) {
                recipe.perform(&mut view as &mut dyn AlloyTank, rules.tags);
                break;
            }
        }
    }

    /// `cool`: with no working structure, every four ticks the items cool when there is no fuel, and fuel burns down.
    fn cool_structure(&mut self, rules: &Rules<'_>) {
        match self.tick % 4 {
            1 if !self.fuel.has_fuel() => self.cool_slots(rules),
            3 if self.fuel.has_fuel() && self.fuel_rate > 0 => self.fuel.decrease(self.fuel_rate),
            _ => {}
        }
    }

    /// One tick of a structure that is whole (`heat`): in four phases that each run every fourth tick.
    pub fn heat_tick(&mut self, rules: &Rules<'_>) {
        if !self.has_tanks {
            self.cool_structure(rules);
        } else {
            match self.tick % 4 {
                0 => {
                    if !self.fuel.has_fuel() {
                        let possible = rules.fuel_source.find_fuel(&mut self.fuel.clone(), false);
                        if self.can_heat(possible) || self.can_alloy(possible, rules) {
                            rules.fuel_source.find_fuel(&mut self.fuel, true);
                        }
                    }
                }
                1 => {
                    if self.fuel.has_fuel() {
                        self.heat_slots(self.fuel.temperature, self.fuel.rate, rules);
                    } else {
                        self.cool_slots(rules);
                    }
                }
                2 => {
                    if self.fuel.has_fuel() {
                        self.do_alloy(self.fuel.temperature, rules);
                    }
                }
                _ => {
                    self.active = self.fuel.has_fuel();
                    self.fuel.decrease(self.fuel_rate);
                }
            }
        }
        self.tick = (self.tick + 1) % 20;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::condition::EmptyContext;
    use serde_json::json;

    fn id(text: &str) -> ResourceId {
        ResourceId::try_parse(text).unwrap()
    }

    struct Recipes;
    impl MeltingRecipes for Recipes {
        fn recipe_for(&self, item: &ResourceId) -> Option<MeltingRecipe> {
            let json = json!({"type": "tconstruct:melting", "ingredient": {"item": "a:iron_ingot"}, "result": {"amount": 90, "fluid": "a:molten_iron"}, "temperature": 800, "time": 6});
            (item.to_string() == "a:iron_ingot").then(|| MeltingRecipe::from_json(id("a:iron"), &json, &EmptyContext).unwrap().unwrap())
        }
    }

    struct NoTags;
    impl FluidTags for NoTags {
        fn members(&self, _: &ResourceId) -> Vec<ResourceId> {
            Vec::new()
        }
    }

    /// A fuel tank holding `left` millibuckets of lava, one portion of 50 giving 100 ticks of fuel at 1000 degrees.
    struct LavaTank(std::cell::Cell<i32>);
    impl FuelSource for LavaTank {
        fn find_fuel(&self, state: &mut FuelState, consume: bool) -> i32 {
            if self.0.get() < 50 {
                return 0;
            }
            if consume {
                self.0.set(self.0.get() - 50);
                state.fuel += 100;
                state.quality = 100;
                state.temperature = 1000;
                state.rate = 10;
            }
            1000
        }
    }

    fn working(slots: usize) -> Heating {
        let mut heating = Heating { has_tanks: true, fuel_rate: 1, slots: vec![MeltingSlot::default(); slots], ..Heating::default() };
        heating.tank.set_capacity(4000);
        heating
    }

    #[test]
    fn an_ingot_is_heated_with_fuel_melted_and_its_fluid_ends_in_the_tank() {
        let mut heating = working(2);
        heating.slots[0].set_stack(Some((id("a:iron_ingot"), 1)), &Recipes);
        let lava = LavaTank(std::cell::Cell::new(200));
        let rules = Rules { melting: &Recipes, alloys: &[], tags: &NoTags, fuel_source: &lava, ore_rate: OreRate::SMELTERY };
        for _ in 0..100 {
            heating.heat_tick(&rules);
        }
        assert_eq!(heating.tank.fluids().iter().map(|f| (f.id.to_string(), f.amount)).collect::<Vec<_>>(), vec![("a:molten_iron".to_owned(), 90)]);
        assert!(heating.slots[0].item.is_none(), "the ingot is gone");
        assert_eq!(lava.0.get(), 150, "one portion of fuel was enough");
    }

    #[test]
    fn nothing_to_melt_or_alloy_means_no_fuel_is_taken() {
        let mut heating = working(1);
        let lava = LavaTank(std::cell::Cell::new(200));
        let rules = Rules { melting: &Recipes, alloys: &[], tags: &NoTags, fuel_source: &lava, ore_rate: OreRate::SMELTERY };
        for _ in 0..40 {
            heating.heat_tick(&rules);
        }
        assert_eq!((lava.0.get(), heating.fuel.fuel), (200, 0));
    }

    #[test]
    fn a_full_tank_holds_the_melted_item_in_its_slot_until_there_is_room() {
        let mut heating = working(1);
        heating.tank.set_capacity(50);
        heating.slots[0].set_stack(Some((id("a:iron_ingot"), 1)), &Recipes);
        let lava = LavaTank(std::cell::Cell::new(500));
        let rules = Rules { melting: &Recipes, alloys: &[], tags: &NoTags, fuel_source: &lava, ore_rate: OreRate::SMELTERY };
        for _ in 0..120 {
            heating.heat_tick(&rules);
        }
        assert!(heating.slots[0].item.is_some() && heating.tank.contained() == 0, "90 do not fit in 50");
        heating.tank.set_capacity(200);
        for _ in 0..8 {
            heating.heat_tick(&rules);
        }
        assert_eq!((heating.slots[0].item.is_none(), heating.tank.contained()), (true, 90));
    }

    #[test]
    fn a_structure_without_a_fuel_tank_only_burns_down_and_cools() {
        let mut heating = working(1);
        heating.has_tanks = false;
        heating.fuel = FuelState { fuel: 30, quality: 100, temperature: 1000, rate: 10 };
        heating.fuel_rate = 5;
        let lava = LavaTank(std::cell::Cell::new(500));
        let rules = Rules { melting: &Recipes, alloys: &[], tags: &NoTags, fuel_source: &lava, ore_rate: OreRate::SMELTERY };
        for _ in 0..8 {
            heating.heat_tick(&rules);
        }
        assert_eq!(heating.fuel.fuel, 20, "two rounds of 5 burned");
        assert_eq!(lava.0.get(), 500, "and no new fuel was looked for");
    }

    #[test]
    fn the_size_of_the_structure_sets_the_capacity_the_slots_and_the_fuel_rate() {
        let mut heating = Heating::default();
        heating.resize(3, 4, 3, 1000);
        assert_eq!((heating.tank.capacity(), heating.slots.len()), (36_000, 36));
        // walls: 2*(3*4) + 2*(4*3) + 3*3 = 57; 57 / 15 = 3; plus 1
        assert_eq!(heating.fuel_rate, 4);
    }
}
