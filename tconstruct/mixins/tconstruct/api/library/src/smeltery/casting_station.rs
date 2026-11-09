//! A casting table or basin at work (`CastingBlockEntity` and `CastingFluidHandler`): fluid poured in picks a recipe, the tank is the size
//! of that recipe's fluid and takes only that fluid, and when it is full the cooling time runs; then the result goes to a slot.
use super::alloy::Fluid;
use super::casting::{CastingKind, CastingRecipe, CastingTags};
use crate::resource::ResourceId;

/// An item in a slot.
pub type Item = (ResourceId, u32);

/// The casting recipes of one kind of station, and the tags they are read with.
pub struct Recipes<'a> {
    pub list: &'a [CastingRecipe],
    pub kind: CastingKind,
    pub tags: &'a dyn CastingTags,
}

impl Recipes<'_> {
    /// The first recipe of this kind for the cast item (None for an empty slot) and the fluid.
    pub fn find(&self, cast: Option<&ResourceId>, fluid: &ResourceId) -> Option<&CastingRecipe> {
        self.list.iter().find(|r| r.kind == self.kind && r.matches(cast, fluid, self.tags))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CastingStation {
    /// The slot a cast goes in, and where a finished item sits when the recipe switches slots.
    pub input: Option<Item>,
    /// The slot the result goes to.
    pub output: Option<Item>,
    /// A table that only works with a cast in it; a basin does not need one.
    pub require_cast: bool,
    /// Redstone flips which slot the result goes to.
    pub last_redstone: bool,
    fluid: Option<Fluid>,
    capacity: i32,
    filter: Option<ResourceId>,
    current: Option<CastingRecipe>,
    timer: i32,
    cooling_time: i32,
}

impl CastingStation {
    pub fn new(require_cast: bool) -> Self {
        Self { input: None, output: None, require_cast, last_redstone: false, fluid: None, capacity: 0, filter: None, current: None, timer: 0, cooling_time: -1 }
    }

    pub fn fluid(&self) -> Option<&Fluid> {
        self.fluid.as_ref()
    }
    pub fn timer(&self) -> i32 {
        self.timer
    }
    pub fn cooling_time(&self) -> i32 {
        self.cooling_time
    }
    pub fn is_casting(&self) -> bool {
        self.current.is_some()
    }

    /// The size of the tank: the current recipe's fluid, or what is in it.
    pub fn capacity(&self) -> i32 {
        if self.capacity == 0 { self.fluid.as_ref().map_or(0, |f| f.amount) } else { self.capacity }
    }

    /// `initNewCasting`: how much fluid the recipe for this fluid and the items in the slots takes (0 when there is none), and with
    /// `execute` the recipe is taken as current. A finished item sitting in the output slot is moved to the input to be cast over.
    fn init_new_casting(&mut self, fluid: &ResourceId, execute: bool, recipes: &Recipes<'_>) -> i32 {
        if self.current.is_some() || (self.input.is_some() && self.output.is_some()) {
            return 0;
        }
        if self.output.is_none() {
            if self.input.is_none() && self.require_cast {
                return 0;
            }
            let Some(recipe) = recipes.find(self.input.as_ref().map(|i| &i.0), fluid).cloned() else { return 0 };
            let amount = recipe.fluid_amount;
            if execute {
                self.current = Some(recipe);
            }
            amount
        } else {
            let Some(recipe) = recipes.find(self.output.as_ref().map(|i| &i.0), fluid).cloned() else { return 0 };
            let amount = recipe.fluid_amount;
            if execute {
                self.current = Some(recipe);
                self.input = self.output.take();
            }
            amount
        }
    }

    /// `onContentsChanged`: when the tank is full and there is a recipe, cooling starts (at least 0 ticks).
    fn on_contents_changed(&mut self) {
        self.cooling_time = match (&self.fluid, &self.current) {
            (Some(f), Some(recipe)) if f.amount >= self.capacity() => recipe.cooling_time.max(0),
            _ => -1,
        };
    }

    /// `reset`: forgets the recipe, empties the tank.
    pub fn reset(&mut self) {
        self.timer = 0;
        self.current = None;
        self.capacity = 0;
        self.fluid = None;
        self.filter = None;
        self.on_contents_changed();
    }

    /// `CastingFluidHandler.fill`.
    pub fn fill(&mut self, fluid: &ResourceId, amount: i32, execute: bool, recipes: &Recipes<'_>) -> i32 {
        if amount <= 0 || self.filter.as_ref().is_some_and(|f| f != fluid) {
            return 0;
        }
        let mut capacity = self.capacity;
        if self.capacity == 0 {
            capacity = self.init_new_casting(fluid, execute, recipes);
            if capacity <= 0 {
                return 0;
            }
            if execute {
                self.capacity = capacity;
                self.filter = Some(fluid.clone());
            }
        }
        match self.fluid.as_mut() {
            None => {
                let taken = capacity.min(amount);
                if execute {
                    self.fluid = Some(Fluid { id: fluid.clone(), amount: taken });
                    self.on_contents_changed();
                }
                taken
            }
            Some(held) if held.id != *fluid => 0,
            Some(held) => {
                let space = capacity - held.amount;
                if space <= 0 {
                    return 0;
                }
                let taken = space.min(amount);
                if execute {
                    held.amount += taken;
                    self.on_contents_changed();
                }
                taken
            }
        }
    }

    /// `CastingFluidHandler.drain`: taking all of it out resets the station.
    pub fn drain(&mut self, max: i32, execute: bool) -> Option<Fluid> {
        let held = self.fluid.as_ref()?;
        let drained = held.amount.min(max);
        if drained <= 0 {
            return None;
        }
        let out = Fluid { id: held.id.clone(), amount: drained };
        if execute {
            let left = held.amount - drained;
            if left <= 0 {
                self.reset();
            } else {
                self.fluid = Some(Fluid { id: out.id.clone(), amount: left });
                self.on_contents_changed();
            }
        }
        Some(out)
    }

    /// `swap`: with nothing cast, the two slots trade places.
    pub fn swap(&mut self) {
        if self.current.is_none() {
            std::mem::swap(&mut self.input, &mut self.output);
        }
    }

    /// `serverTick`: while cooling, count; when the time is up, check the recipe still holds (else look for another, and start over if
    /// there is none or it needs more fluid than is there), then put the result in the slot the redstone chooses.
    pub fn tick(&mut self, recipes: &Recipes<'_>) {
        if self.current.is_none() || self.cooling_time < 0 {
            return;
        }
        self.timer += 1;
        if self.timer < self.cooling_time {
            return;
        }
        let Some(fluid) = self.fluid.clone() else { return };
        let still = self.current.as_ref().is_some_and(|r| r.matches(self.input.as_ref().map(|i| &i.0), &fluid.id, recipes.tags));
        if !still {
            self.current = recipes.find(self.input.as_ref().map(|i| &i.0), &fluid.id).cloned();
            if self.current.as_ref().is_none_or(|r| r.fluid_amount > fluid.amount) {
                self.timer = 0;
                return;
            }
        }
        let Some(recipe) = self.current.clone() else { return };
        let Some(result) = recipe.result_item(recipes.tags) else {
            self.timer = 0;
            return;
        };
        if recipe.switch_slots != self.last_redstone {
            if !recipe.cast_consumed {
                self.output = self.input.clone();
            }
            self.input = Some(result);
        } else {
            if recipe.cast_consumed {
                self.input = None;
            }
            self.output = Some(result);
        }
        self.reset();
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

    struct Tags;
    impl CastingTags for Tags {
        fn items(&self, _: &ResourceId) -> Vec<ResourceId> {
            Vec::new()
        }
        fn fluids(&self, _: &ResourceId) -> Vec<ResourceId> {
            Vec::new()
        }
    }

    fn recipe(json: serde_json::Value) -> CastingRecipe {
        CastingRecipe::from_json(id("a:r"), &json, &EmptyContext).unwrap().unwrap()
    }

    fn ingot() -> CastingRecipe {
        recipe(json!({"type": "tconstruct:casting_table", "cast": {"item": "a:ingot_cast"}, "cooling_time": 5, "fluid": {"amount": 90, "fluid": "a:molten_iron"}, "result": {"item": "a:iron_ingot"}}))
    }

    fn station_with_cast() -> CastingStation {
        let mut station = CastingStation::new(true);
        station.input = Some((id("a:ingot_cast"), 1));
        station
    }

    #[test]
    fn pouring_a_full_recipe_amount_cools_and_gives_the_item_and_keeps_the_cast() {
        let list = [ingot()];
        let recipes = Recipes { list: &list, kind: CastingKind::Table, tags: &Tags };
        let mut station = station_with_cast();
        assert_eq!(station.fill(&id("a:molten_iron"), 200, true, &recipes), 90, "takes the recipe's amount");
        assert_eq!((station.capacity(), station.cooling_time()), (90, 5));
        for _ in 0..5 {
            station.tick(&recipes);
        }
        assert_eq!(station.output, Some((id("a:iron_ingot"), 1)));
        assert_eq!(station.input, Some((id("a:ingot_cast"), 1)), "the cast stays");
        assert!(station.fluid().is_none() && !station.is_casting());
    }

    #[test]
    fn a_partial_pour_waits_for_the_rest_and_another_fluid_is_refused() {
        let list = [ingot()];
        let recipes = Recipes { list: &list, kind: CastingKind::Table, tags: &Tags };
        let mut station = station_with_cast();
        assert_eq!(station.fill(&id("a:molten_iron"), 30, true, &recipes), 30);
        assert_eq!(station.cooling_time(), -1, "not full yet");
        station.tick(&recipes);
        assert_eq!(station.timer(), 0);
        assert_eq!(station.fill(&id("a:molten_gold"), 60, true, &recipes), 0, "the tank takes only the recipe's fluid");
        assert_eq!(station.fill(&id("a:molten_iron"), 100, true, &recipes), 60);
        assert_eq!(station.cooling_time(), 5);
    }

    #[test]
    fn a_table_that_needs_a_cast_takes_no_fluid_without_one_and_a_basin_does() {
        let list = [ingot(), recipe(json!({"type": "tconstruct:casting_basin", "cooling_time": 3, "fluid": {"amount": 810, "fluid": "a:molten_iron"}, "result": {"item": "a:iron_block"}}))];
        let table = Recipes { list: &list, kind: CastingKind::Table, tags: &Tags };
        let mut empty_table = CastingStation::new(true);
        assert_eq!(empty_table.fill(&id("a:molten_iron"), 90, true, &table), 0);
        let basin = Recipes { list: &list, kind: CastingKind::Basin, tags: &Tags };
        let mut basin_station = CastingStation::new(false);
        assert_eq!(basin_station.fill(&id("a:molten_iron"), 1000, true, &basin), 810);
        for _ in 0..3 {
            basin_station.tick(&basin);
        }
        assert_eq!(basin_station.output, Some((id("a:iron_block"), 1)));
    }

    #[test]
    fn a_consumed_cast_is_used_up_and_switching_slots_puts_the_result_where_the_cast_was() {
        let consumed = recipe(json!({"type": "tconstruct:casting_table", "cast": {"item": "a:glass"}, "cast_consumed": true, "cooling_time": 1, "fluid": {"amount": 10, "fluid": "a:molten_iron"}, "result": {"item": "a:tinted"}}));
        let list = [consumed];
        let recipes = Recipes { list: &list, kind: CastingKind::Table, tags: &Tags };
        let mut station = CastingStation::new(true);
        station.input = Some((id("a:glass"), 1));
        station.fill(&id("a:molten_iron"), 10, true, &recipes);
        station.tick(&recipes);
        assert_eq!((station.input.clone(), station.output.clone()), (None, Some((id("a:tinted"), 1))));

        let switching = recipe(json!({"type": "tconstruct:casting_table", "cast": {"item": "a:ingot_cast"}, "switch_slots": true, "cooling_time": 1, "fluid": {"amount": 10, "fluid": "a:molten_iron"}, "result": {"item": "a:iron_ingot"}}));
        let list = [switching];
        let recipes = Recipes { list: &list, kind: CastingKind::Table, tags: &Tags };
        let mut station = station_with_cast();
        station.fill(&id("a:molten_iron"), 10, true, &recipes);
        station.tick(&recipes);
        assert_eq!((station.input.clone(), station.output.clone()), (Some((id("a:iron_ingot"), 1)), Some((id("a:ingot_cast"), 1))), "the result goes to the input, the cast to the output");
    }

    #[test]
    fn taking_the_cast_away_while_it_cools_ends_the_casting_without_a_result() {
        let list = [ingot()];
        let recipes = Recipes { list: &list, kind: CastingKind::Table, tags: &Tags };
        let mut station = station_with_cast();
        station.fill(&id("a:molten_iron"), 90, true, &recipes);
        station.input = None;
        for _ in 0..6 {
            station.tick(&recipes);
        }
        assert_eq!((station.output.clone(), station.timer()), (None, 0));
    }

    #[test]
    fn draining_everything_resets_and_a_finished_item_can_be_cast_over() {
        let list = [ingot()];
        let recipes = Recipes { list: &list, kind: CastingKind::Table, tags: &Tags };
        let mut station = station_with_cast();
        station.fill(&id("a:molten_iron"), 50, true, &recipes);
        assert_eq!(station.drain(100, true).map(|f| f.amount), Some(50));
        assert!(!station.is_casting() && station.capacity() == 0);
        // an item in the output slot with an empty input is moved to the input to be cast over
        let mut again = CastingStation::new(true);
        again.output = Some((id("a:ingot_cast"), 1));
        assert_eq!(again.fill(&id("a:molten_iron"), 90, true, &recipes), 90);
        assert_eq!((again.input.clone(), again.output.clone()), (Some((id("a:ingot_cast"), 1)), None));
        let mut swapped = CastingStation::new(true);
        swapped.output = Some((id("a:x"), 1));
        swapped.swap();
        assert_eq!((swapped.input, swapped.output), (Some((id("a:x"), 1)), None));
    }
}
