//! Source-compatible single-fluid storage embedded in an item component.
//! The caller owns capacity, capability eligibility and the transaction.
use horizon_items::items::ItemStack;
use horizon_world::nbt::{Compound, Tag};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StackFluid {
    pub id: String,
    pub amount: u32,
    pub tag: Option<Tag>,
}

/// Return unknown for malformed/opaque data, Some(None) for an empty tank.
pub fn read(stack: &ItemStack, compound_key: &str, fluid_key: &str) -> Option<Option<StackFluid>> {
    if stack.count != 1 || stack.wire_patch().is_some() { return None; }
    let Some(custom) = stack.data().and_then(|data| data.other("minecraft:custom_data")) else { return Some(None); };
    let Tag::Compound(custom) = custom else { return None; };
    let Some(compound) = custom.get(&compound_key.into()) else { return Some(None); };
    let Tag::Compound(compound) = compound else { return None; };
    let Some(fluid) = compound.get(&fluid_key.into()) else { return Some(None); };
    let Tag::Compound(fluid) = fluid else { return None; };
    if fluid.keys().any(|key| !matches!(key.to_string_lossy().as_str(), "FluidName" | "Amount" | "Tag")) { return None; }
    let Tag::String(id) = fluid.get(&"FluidName".into())? else { return None; };
    let id = id.to_string_lossy();
    let Tag::Int(amount) = fluid.get(&"Amount".into())? else { return None; };
    let amount = u32::try_from(*amount).ok()?;
    let tag = match fluid.get(&"Tag".into()) { None => None, Some(value @ Tag::Compound(_)) => Some(value.clone()), _ => return None };
    if id.is_empty() || id == "minecraft:empty" || amount == 0 { return None; }
    Some(Some(StackFluid { id, amount, tag }))
}

/// Stage the updated item on a copy; no caller state is modified on failure.
pub fn write(stack: &ItemStack, compound_key: &str, fluid_key: &str, fluid: Option<&StackFluid>, capacity: u32) -> Option<ItemStack> {
    read(stack, compound_key, fluid_key)?;
    if let Some(value) = fluid {
        if value.id.is_empty() || value.id == "minecraft:empty" || value.amount == 0 || value.amount > capacity
            || value.amount > i32::MAX as u32 || value.tag.as_ref().is_some_and(|tag| !matches!(tag, Tag::Compound(_))) { return None; }
    }
    let mut custom = match stack.data().and_then(|data| data.other("minecraft:custom_data")) {
        None => Compound::new(), Some(Tag::Compound(value)) => value.clone(), _ => return None,
    };
    let mut compound = match custom.remove(&compound_key.into()) {
        None => Compound::new(), Some(Tag::Compound(value)) => value, _ => return None,
    };
    match fluid {
        None => { compound.remove(&fluid_key.into()); },
        Some(fluid) => {
            let mut value = Compound::new();
            value.insert("FluidName".into(), Tag::String(fluid.id.as_str().into()));
            value.insert("Amount".into(), Tag::Int(fluid.amount as i32));
            if let Some(tag) = &fluid.tag { value.insert("Tag".into(), tag.clone()); }
            compound.insert(fluid_key.into(), Tag::Compound(value));
        }
    }
    if !compound.is_empty() { custom.insert(compound_key.into(), Tag::Compound(compound)); }
    let mut output = stack.clone();
    output.data_mut().set_other("minecraft:custom_data", (!custom.is_empty()).then_some(Tag::Compound(custom)));
    Some(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn item_tank_round_trips_metadata_without_touching_other_fields() {
        let mut item = ItemStack::new("tconstruct:pickaxe", 1);
        item.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
            ("other", Tag::Int(9)),
            ("tic_persistent", Tag::compound([("unrelated", Tag::Int(4))])),
        ])));
        let fluid = StackFluid { id: "minecraft:water".into(), amount: 250, tag: Some(Tag::compound([("example", Tag::Int(1))])) };
        let filled = write(&item, "tic_persistent", "tconstruct:tank_fluid", Some(&fluid), 1000).unwrap();
        assert_eq!(read(&filled, "tic_persistent", "tconstruct:tank_fluid"), Some(Some(fluid)));
        let empty = write(&filled, "tic_persistent", "tconstruct:tank_fluid", None, 1000).unwrap();
        assert_eq!(empty.data().unwrap().other("minecraft:custom_data").unwrap().get("other"), Some(&Tag::Int(9)));
        assert_eq!(empty.data().unwrap().other("minecraft:custom_data").unwrap().get("tic_persistent").unwrap().get("unrelated"), Some(&Tag::Int(4)));
        assert_eq!(read(&empty, "tic_persistent", "tconstruct:tank_fluid"), Some(None));
    }
}
