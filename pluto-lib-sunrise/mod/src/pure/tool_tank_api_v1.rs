//! A fluid tank carried in an item stack's own data: the tool tank of `ToolTankHelper` and
//! `TankModule` (SlimeKnights Tinkers' Construct, MIT; see the fluid item API for the item
//! containers whose contents are whole stacks instead).
//!
//! One fluid stack is kept under a caller-chosen key of a data compound, in the same
//! `{FluidName, Amount, Tag?}` shape the fluid item API's tanks use (Forge's
//! `FluidStack.writeToNBT`). The capacity is not part of the stored value: a tool's capacity is
//! its own stat and changes as the tool is rebuilt, so every call takes the current capacity.
//! Setting, filling and draining follow the original's rules exactly:
//!
//! - Setting an empty stack removes the key; setting a stack clamps its amount to the capacity
//!   (`ToolTankHelper.setFluid`). A stack clamped to nothing is empty and removes the key too.
//! - Filling is first come, first served (`TankModule.fill`): an empty tank takes the offered
//!   fluid up to the capacity; a tank holding the same fluid (same id and same tag, Forge's
//!   `isFluidEqual`) tops up to the capacity; any other fluid is refused whole.
//! - Draining takes the stored fluid up to the amount asked for, either of a named fluid
//!   (`drain(FluidStack)`, which must match the stored one) or of whatever is stored
//!   (`drain(int)`).
//!
//! Asking and doing are the same calls with an `execute` flag, as with the fluid storage API:
//! a call with `execute` false reports what it would move and changes nothing.

use super::base::{Identifier, Nbt, NbtCompound, NbtCompoundExt};
use super::block_entity_api_v1::FluidStack;

/// The fluid stored under `key` in `data`, if it holds any. A stored amount of 0 or less, a
/// missing name or an unparsable fluid id all read as an empty tank.
pub fn read(data: &NbtCompound, key: &str) -> Option<FluidStack> {
    let tag = data.get_compound(key)?;
    let name = tag.get_string("FluidName")?;
    let amount = tag.get_int("Amount", 0);
    if amount <= 0 {
        return None;
    }
    let fluid = Identifier::parse(name).ok()?;
    let tag = tag.get_compound("Tag").cloned();
    Some(FluidStack { fluid, amount: amount as u32, tag })
}

/// `ToolTankHelper.setFluid`: stores `fluid` under `key`, clamped to `capacity`; an empty
/// stack (or one clamped to nothing) removes the key. Returns what is stored afterwards.
pub fn write(data: &mut NbtCompound, key: &str, fluid: Option<&FluidStack>, capacity: u32) -> Option<FluidStack> {
    let Some(fluid) = fluid.filter(|fluid| fluid.amount > 0) else {
        data.remove(key);
        return None;
    };
    let amount = fluid.amount.min(capacity);
    if amount == 0 {
        data.remove(key);
        return None;
    }
    let mut tag = NbtCompound::new();
    tag.insert("FluidName".to_owned(), Nbt::String(fluid.fluid.to_string()));
    tag.insert("Amount".to_owned(), Nbt::Int(amount.min(i32::MAX as u32) as i32));
    if let Some(fluid_tag) = &fluid.tag {
        tag.insert("Tag".to_owned(), Nbt::Compound(fluid_tag.clone()));
    }
    data.insert(key.to_owned(), Nbt::Compound(tag));
    Some(FluidStack { fluid: fluid.fluid.clone(), amount, tag: fluid.tag.clone() })
}

/// `TankModule.fill`: how much of `resource` the tank under `key` takes, out of a capacity of
/// `capacity`. With `execute` the taken fluid is stored; a simulation changes nothing.
pub fn fill(data: &mut NbtCompound, key: &str, capacity: u32, resource: &FluidStack, execute: bool) -> u32 {
    if resource.amount == 0 {
        return 0;
    }
    match read(data, key) {
        None => {
            let taken = resource.amount.min(capacity);
            if taken > 0 && execute {
                write(data, key, Some(resource), capacity);
            }
            taken
        }
        Some(current) => {
            if current.amount >= capacity || !current.same_fluid(resource) {
                return 0;
            }
            let filled = resource.amount.min(capacity - current.amount);
            if filled > 0 && execute {
                let grown = FluidStack { fluid: current.fluid.clone(), amount: current.amount + filled, tag: current.tag.clone() };
                write(data, key, Some(&grown), capacity);
            }
            filled
        }
    }
}

/// `TankModule.drain(FluidStack)`: takes up to `wanted.amount` of the stored fluid, which must
/// be the same fluid as `wanted` (id and tag). None when the tank is empty or holds another
/// fluid. With `execute` the drained fluid leaves the tank.
pub fn drain(data: &mut NbtCompound, key: &str, wanted: &FluidStack, execute: bool) -> Option<FluidStack> {
    if wanted.amount == 0 {
        return None;
    }
    let current = read(data, key)?;
    if !current.same_fluid(wanted) {
        return None;
    }
    take(data, key, current, wanted.amount, execute)
}

/// `TankModule.drain(int)`: takes up to `max` of whatever the tank under `key` holds.
pub fn drain_amount(data: &mut NbtCompound, key: &str, max: u32, execute: bool) -> Option<FluidStack> {
    if max == 0 {
        return None;
    }
    let current = read(data, key)?;
    take(data, key, current, max, execute)
}

/// Takes up to `max` out of `current` (the tank's content), storing the rest on `execute`.
fn take(data: &mut NbtCompound, key: &str, current: FluidStack, max: u32, execute: bool) -> Option<FluidStack> {
    let amount = current.amount.min(max);
    if amount == 0 {
        return None;
    }
    if execute {
        if amount == current.amount {
            write(data, key, None, 0);
        } else {
            let left = FluidStack { fluid: current.fluid.clone(), amount: current.amount - amount, tag: current.tag.clone() };
            write(data, key, Some(&left), u32::MAX);
        }
    }
    Some(FluidStack { fluid: current.fluid, amount, tag: current.tag })
}

/// `TankModule.validate`: clamps the stored amount to `capacity`, keeping the fluid itself.
/// Returns whether the stored stack changed.
pub fn validate(data: &mut NbtCompound, key: &str, capacity: u32) -> bool {
    let Some(current) = read(data, key) else { return false };
    if current.amount <= capacity {
        return false;
    }
    write(data, key, Some(&current), capacity);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn stack(fluid: &str, amount: u32) -> FluidStack { FluidStack::new(id(fluid), amount) }
    fn tagged(fluid: &str, amount: u32) -> FluidStack {
        let mut tag = NbtCompound::new();
        tag.insert("potion".to_owned(), Nbt::String("minecraft:water".to_owned()));
        FluidStack { fluid: id(fluid), amount, tag: Some(tag) }
    }
    fn data_with(fluid: &FluidStack, capacity: u32) -> NbtCompound {
        let mut data = NbtCompound::new();
        write(&mut data, "tank", Some(fluid), capacity);
        data
    }

    #[test]
    fn a_written_fluid_round_trips_with_its_tag_and_an_empty_write_removes_the_key() {
        let mut data = NbtCompound::new();
        assert_eq!(read(&data, "tank"), None);
        let stored = write(&mut data, "tank", Some(&tagged("tconstruct:molten_iron", 500)), 1000);
        assert_eq!(stored, Some(tagged("tconstruct:molten_iron", 500)));
        assert_eq!(read(&data, "tank"), Some(tagged("tconstruct:molten_iron", 500)));
        // The stored shape is the fluid item tanks' shape.
        let compound = data.get_compound("tank").unwrap();
        assert_eq!(compound.get_string("FluidName"), Some("tconstruct:molten_iron"));
        assert_eq!(compound.get_int("Amount", 0), 500);
        assert!(compound.get_compound("Tag").is_some());
        assert_eq!(write(&mut data, "tank", None, 1000), None);
        assert_eq!(read(&data, "tank"), None);
        assert!(!data.contains_key("tank"));
    }

    #[test]
    fn setting_clamps_to_the_capacity_and_a_zero_capacity_stores_nothing() {
        let mut data = NbtCompound::new();
        let stored = write(&mut data, "tank", Some(&stack("minecraft:lava", 1500)), 1000);
        assert_eq!(stored, Some(stack("minecraft:lava", 1000)), "setFluid clamps to the capacity");
        assert_eq!(write(&mut data, "tank", Some(&stack("minecraft:lava", 10)), 0), None);
        assert_eq!(read(&data, "tank"), None);
    }

    #[test]
    fn fill_takes_an_empty_tank_first_and_tops_up_the_same_fluid_only() {
        let mut data = NbtCompound::new();
        assert_eq!(fill(&mut data, "tank", 1000, &stack("minecraft:water", 1400), false), 1000, "a simulation reports the take");
        assert_eq!(read(&data, "tank"), None, "a simulation stores nothing");
        assert_eq!(fill(&mut data, "tank", 1000, &stack("minecraft:water", 1400), true), 1000);
        assert_eq!(read(&data, "tank"), Some(stack("minecraft:water", 1000)));
        assert_eq!(fill(&mut data, "tank", 1000, &stack("minecraft:water", 100), true), 0, "a full tank takes nothing");

        let mut data = data_with(&stack("minecraft:water", 400), 1000);
        assert_eq!(fill(&mut data, "tank", 1000, &stack("minecraft:lava", 100), true), 0, "another fluid is refused whole");
        assert_eq!(fill(&mut data, "tank", 1000, &tagged("minecraft:water", 100), true), 0, "a tagged stack is another fluid to an untagged tank");
        assert_eq!(fill(&mut data, "tank", 1000, &stack("minecraft:water", 900), true), 600, "the same fluid tops up to the capacity");
        assert_eq!(read(&data, "tank"), Some(stack("minecraft:water", 1000)));
    }

    #[test]
    fn drain_takes_up_to_the_ask_and_the_named_form_must_match() {
        let mut data = data_with(&stack("tconstruct:molten_gold", 700), 1000);
        assert_eq!(drain_amount(&mut data, "tank", 200, false), Some(stack("tconstruct:molten_gold", 200)));
        assert_eq!(read(&data, "tank"), Some(stack("tconstruct:molten_gold", 700)), "a simulation drains nothing");
        assert_eq!(drain(&mut data, "tank", &stack("minecraft:lava", 100), true), None, "a named drain of another fluid fails");
        assert_eq!(drain(&mut data, "tank", &tagged("tconstruct:molten_gold", 100), true), None, "the tag must match too");
        assert_eq!(drain(&mut data, "tank", &stack("tconstruct:molten_gold", 250), true), Some(stack("tconstruct:molten_gold", 250)));
        assert_eq!(read(&data, "tank"), Some(stack("tconstruct:molten_gold", 450)));
        assert_eq!(drain_amount(&mut data, "tank", 1000, true), Some(stack("tconstruct:molten_gold", 450)), "draining past the content takes it all");
        assert_eq!(read(&data, "tank"), None);
        assert_eq!(drain_amount(&mut data, "tank", 10, true), None, "an empty tank drains nothing");
    }

    #[test]
    fn validate_clamps_an_overfull_tank_and_leaves_a_fitting_one() {
        let mut data = data_with(&stack("minecraft:lava", 900), 1000);
        assert!(!validate(&mut data, "tank", 1000));
        assert!(validate(&mut data, "tank", 500), "a capacity that shrank under the content clamps it");
        assert_eq!(read(&data, "tank"), Some(stack("minecraft:lava", 500)));
        assert!(!validate(&mut data, "tank", 500));
    }
}
