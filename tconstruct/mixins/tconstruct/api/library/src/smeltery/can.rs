//! Fluid in items: the copper can (`CopperCanItem` and `CopperCanFluidHandler`) and the scaled tank of the tank item (`ScaledFluidTank`).
//!
//! A copper can holds one ingot (90 mB) of one fluid for each can in the stack, all or nothing. Its content is kept in the stack's data as
//! the fluid id (`fluid`) and, when the fluid has data of its own, that data (`fluid_tag`). A scaled tank holds a multiple of some unit
//! and only fills and drains whole units.
use super::alloy::Fluid;
use crate::resource::ResourceId;

/// One ingot, the amount a can holds.
pub const CAN_CAPACITY: i32 = 90;

/// What a can holds: the fluid and the data of the fluid, kept as text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanContents {
    pub fluid: ResourceId,
    pub fluid_tag: Option<String>,
}

/// A fluid offered to a can, with the data it carries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanFluid {
    pub fluid: Fluid,
    pub tag: Option<String>,
}

impl CanContents {
    /// Reads the stack's `fluid` and `fluid_tag` values; a missing or unknown fluid is an empty can.
    pub fn from_values(fluid: Option<&str>, fluid_tag: Option<&str>) -> Option<Self> {
        let id = ResourceId::try_parse(fluid?)?;
        if id.to_string() == "minecraft:empty" {
            return None;
        }
        Some(Self { fluid: id, fluid_tag: fluid_tag.map(str::to_owned) })
    }

    /// The values to keep in the stack (`setFluid`); None means remove both (`removeFluid`).
    pub fn to_values(contents: Option<&Self>) -> Option<(String, Option<String>)> {
        contents.map(|c| (c.fluid.to_string(), c.fluid_tag.clone()))
    }
}

/// A stack of copper cans as a fluid handler (`CopperCanFluidHandler`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanStack {
    /// How many cans are in the stack.
    pub count: i32,
    pub contents: Option<CanContents>,
}

impl CanStack {
    /// The capacity grows with the stack: all the cans fill together.
    pub fn capacity(&self) -> i32 {
        CAN_CAPACITY * self.count
    }

    /// `getFluidInTank`: a full tank of the fluid, or nothing.
    pub fn fluid(&self) -> Option<CanFluid> {
        self.contents.as_ref().map(|c| CanFluid { fluid: Fluid { id: c.fluid.clone(), amount: self.capacity() }, tag: c.fluid_tag.clone() })
    }

    /// `fill`: only an empty can, and only a whole capacity or more; takes exactly the capacity.
    pub fn fill(&mut self, resource: &CanFluid, execute: bool) -> i32 {
        let capacity = self.capacity();
        if self.contents.is_some() || resource.fluid.amount < capacity {
            return 0;
        }
        if execute {
            self.contents = Some(CanContents { fluid: resource.fluid.id.clone(), fluid_tag: resource.tag.clone() });
        }
        capacity
    }

    /// `drain` of a named fluid: the whole content, when the fluid and its data are the same and at least the capacity is asked for.
    pub fn drain_fluid(&mut self, resource: &CanFluid, execute: bool) -> Option<CanFluid> {
        let capacity = self.capacity();
        if resource.fluid.amount < capacity {
            return None;
        }
        let out = self.fluid()?;
        if out.fluid.id != resource.fluid.id || out.tag != resource.tag {
            return None;
        }
        if execute {
            self.contents = None;
        }
        Some(out)
    }

    /// `drain` of an amount: the whole content, when at least the capacity is asked for.
    pub fn drain(&mut self, max: i32, execute: bool) -> Option<CanFluid> {
        if max < self.capacity() {
            return None;
        }
        let out = self.fluid()?;
        if execute {
            self.contents = None;
        }
        Some(out)
    }

    /// `hasCraftingRemainingItem`: a can with fluid leaves an empty can behind when used in crafting.
    pub fn leaves_remainder(&self) -> bool {
        self.contents.is_some()
    }
}

/// `ScaledFluidTank`: a tank whose amounts are multiples of `scale`. It is made with a capacity in units and holds that many times `scale`;
/// what goes in or comes out is rounded down to whole units, and it is saved in units.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScaledTank {
    pub capacity: i32,
    scale: i32,
    pub fluid: Option<Fluid>,
}

impl ScaledTank {
    pub fn new(capacity: i32, scale: i32) -> Self {
        let scale = scale.max(1);
        Self { capacity: capacity * scale, scale, fluid: None }
    }

    fn whole(&self, amount: i32) -> i32 {
        amount - amount % self.scale
    }

    pub fn fill(&mut self, fluid: &ResourceId, amount: i32, execute: bool) -> i32 {
        let amount = self.whole(amount);
        if amount <= 0 || self.fluid.as_ref().is_some_and(|f| f.id != *fluid) {
            return 0;
        }
        let room = self.capacity - self.fluid.as_ref().map_or(0, |f| f.amount);
        let taken = self.whole(room.min(amount));
        if execute && taken > 0 {
            match self.fluid.as_mut() {
                Some(f) => f.amount += taken,
                None => self.fluid = Some(Fluid { id: fluid.clone(), amount: taken }),
            }
        }
        taken
    }

    pub fn drain(&mut self, max: i32, execute: bool) -> Option<Fluid> {
        let held = self.fluid.as_ref()?;
        let drained = self.whole(max).min(held.amount);
        if drained <= 0 {
            return None;
        }
        let out = Fluid { id: held.id.clone(), amount: drained };
        if execute {
            let left = held.amount - drained;
            self.fluid = (left > 0).then(|| Fluid { id: out.id.clone(), amount: left });
        }
        Some(out)
    }

    /// `writeToNBT`: the fluid and its amount in units.
    pub fn to_saved(&self) -> Option<(String, i32)> {
        self.fluid.as_ref().map(|f| (f.id.to_string(), f.amount / self.scale))
    }

    /// `readFromNBT`: the amount in units back to millibuckets, rounded to whole units.
    pub fn read_saved(&mut self, saved: Option<(&str, i32)>) {
        self.fluid = saved.and_then(|(id, units)| Some(Fluid { id: ResourceId::try_parse(id)?, amount: units * self.scale })).filter(|f| f.amount > 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(text: &str) -> ResourceId {
        ResourceId::try_parse(text).unwrap()
    }

    fn lava(amount: i32) -> CanFluid {
        CanFluid { fluid: Fluid { id: id("minecraft:lava"), amount }, tag: None }
    }

    #[test]
    fn a_can_takes_exactly_its_capacity_and_only_when_empty() {
        let mut can = CanStack { count: 1, contents: None };
        assert_eq!(can.fill(&lava(50), true), 0, "less than a can's worth");
        assert_eq!(can.fill(&lava(1000), true), 90, "takes exactly 90");
        assert_eq!(can.fluid().map(|f| f.fluid.amount), Some(90));
        assert_eq!(can.fill(&lava(1000), true), 0, "already full");
        assert!(can.leaves_remainder());
    }

    #[test]
    fn a_stack_of_cans_fills_and_drains_together() {
        let mut stack = CanStack { count: 4, contents: None };
        assert_eq!(stack.capacity(), 360);
        assert_eq!(stack.fill(&lava(300), true), 0);
        assert_eq!(stack.fill(&lava(360), true), 360);
        assert_eq!(stack.drain(359, true), None, "less than the capacity");
        assert_eq!(stack.drain(360, true).map(|f| f.fluid.amount), Some(360));
        assert!(stack.contents.is_none() && !stack.leaves_remainder());
    }

    #[test]
    fn a_named_drain_needs_the_same_fluid_and_data() {
        let mut can = CanStack { count: 1, contents: Some(CanContents { fluid: id("a:potion"), fluid_tag: Some("{Potion:speed}".to_owned()) }) };
        let wrong_tag = CanFluid { fluid: Fluid { id: id("a:potion"), amount: 90 }, tag: Some("{Potion:slow}".to_owned()) };
        assert_eq!(can.drain_fluid(&wrong_tag, true), None);
        let wrong_fluid = CanFluid { fluid: Fluid { id: id("a:other"), amount: 90 }, tag: Some("{Potion:speed}".to_owned()) };
        assert_eq!(can.drain_fluid(&wrong_fluid, true), None);
        let right = CanFluid { fluid: Fluid { id: id("a:potion"), amount: 90 }, tag: Some("{Potion:speed}".to_owned()) };
        assert_eq!(can.drain_fluid(&right, false).map(|f| f.fluid.amount), Some(90));
        assert!(can.contents.is_some(), "a simulated drain leaves it");
        can.drain_fluid(&right, true);
        assert!(can.contents.is_none());
    }

    #[test]
    fn the_stack_values_round_trip_and_an_empty_fluid_is_an_empty_can() {
        let contents = CanContents::from_values(Some("minecraft:lava"), None).unwrap();
        assert_eq!(CanContents::to_values(Some(&contents)), Some(("minecraft:lava".to_owned(), None)));
        assert_eq!(CanContents::to_values(None), None);
        assert_eq!(CanContents::from_values(Some("minecraft:empty"), None), None);
        assert_eq!(CanContents::from_values(None, Some("{}")), None);
    }

    #[test]
    fn a_scaled_tank_fills_and_drains_whole_units_and_saves_in_units() {
        let mut tank = ScaledTank::new(10, 25);
        assert_eq!(tank.capacity, 250);
        assert_eq!(tank.fill(&id("a:iron"), 60, true), 50, "60 rounds down to two units");
        assert_eq!(tank.drain(30, true).map(|f| f.amount), Some(25));
        assert_eq!(tank.to_saved(), Some(("a:iron".to_owned(), 1)));
        let mut loaded = ScaledTank::new(10, 25);
        loaded.read_saved(Some(("a:iron", 3)));
        assert_eq!(loaded.fluid.as_ref().map(|f| f.amount), Some(75));
        assert_eq!(tank.drain(10, true), None, "less than a unit");
    }
}
