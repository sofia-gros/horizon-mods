//! Shared pieces of the pipes mod: direction bookkeeping, saved-data codecs for the stacks in
//! transit, and the block lists the connection shapes are decided from.
//!
//! Clean-room design: the transport model follows the classic extraction-pipe idea — a
//! redstone-enabled extraction pipe pulls one item (or a measure of fluid) out of the block it
//! faces every eight ticks and sends it into the pipe run — but no code or asset of any existing
//! mod is copied; everything here is written for this mod against PlutoLib's public neighbour
//! reach (`Neighbours`), the same reach the faucets and channels of Tinkers' Construct use.
use pluto_lib::{Direction, FluidStack, Identifier, ItemSlot, Nbt, NbtCompound};

pub const NAMESPACE: &str = "pipes";
pub const ITEM_PIPE: &str = "item_pipe";
pub const EXTRACTION_PIPE: &str = "extraction_pipe";
pub const FLUID_PIPE: &str = "fluid_pipe";
pub const FLUID_EXTRACTION_PIPE: &str = "fluid_extraction_pipe";

/// The index of a face in `Direction::ALL` order (down, up, north, south, west, east). An item
/// pipe's inbox slot for a face is the face's index here; the type's sided access opens exactly
/// that slot for inserts through that face, so a pipe always knows which side a stack arrived
/// from.
pub fn face_index(face: Direction) -> usize {
    Direction::ALL.iter().position(|own| *own == face).unwrap_or(0)
}

/// One of this mod's item pipes (the transport pipe or the extraction pipe).
pub fn is_item_pipe(id: &Identifier) -> bool {
    id.namespace() == NAMESPACE && (id.path() == ITEM_PIPE || id.path() == EXTRACTION_PIPE)
}

/// One of this mod's fluid pipes.
pub fn is_fluid_pipe(id: &Identifier) -> bool {
    id.namespace() == NAMESPACE && (id.path() == FLUID_PIPE || id.path() == FLUID_EXTRACTION_PIPE)
}

/// A stack as saved data: its item, count, damage and its own `minecraft:custom_data`. The other
/// per-stack components a stack can carry (a tool component, attribute modifiers, enchantments,
/// custom model data) have no saved form inside a pipe, so a stack that rides a pipe keeps the
/// four fields above; plain materials — ores, ingots, cast products — are unaffected.
pub fn stack_to_nbt(stack: &ItemSlot) -> NbtCompound {
    let mut data = NbtCompound::new();
    data.insert("Item".to_owned(), Nbt::String(stack.item.to_string()));
    data.insert("Count".to_owned(), Nbt::Int(stack.count as i32));
    data.insert("Damage".to_owned(), Nbt::Int(stack.damage as i32));
    if let Some(max) = stack.max_damage {
        data.insert("MaxDamage".to_owned(), Nbt::Int(max));
    }
    if !stack.custom_data.is_empty() {
        data.insert("Data".to_owned(), Nbt::Compound(stack.custom_data.clone()));
    }
    data
}

/// The stack [`stack_to_nbt`] saved, or `None` when the item id does not parse or the count is
/// not positive.
pub fn stack_from_nbt(data: &NbtCompound) -> Option<ItemSlot> {
    let item = Identifier::parse(data.get("Item")?.as_str()?).ok()?;
    let count = data.get("Count")?.as_i64()?;
    if count <= 0 {
        return None;
    }
    let mut stack = ItemSlot::new(item, count as u32);
    stack.damage = data.get("Damage").and_then(Nbt::as_i64).unwrap_or(0).max(0) as u32;
    stack.max_damage = data.get("MaxDamage").and_then(Nbt::as_i64).map(|value| value as i32);
    if let Some(custom) = data.get("Data").and_then(Nbt::as_compound) {
        stack.custom_data = custom.clone();
    }
    Some(stack)
}

/// A fluid stack as saved data (the faucet's encoding: `Fluid`, `Amount`, optional `Tag`).
pub fn fluid_to_nbt(stack: &FluidStack) -> NbtCompound {
    let mut data = NbtCompound::new();
    data.insert("Fluid".to_owned(), Nbt::String(stack.fluid.to_string()));
    data.insert("Amount".to_owned(), Nbt::Int(stack.amount as i32));
    if let Some(tag) = &stack.tag {
        data.insert("Tag".to_owned(), Nbt::Compound(tag.clone()));
    }
    data
}

/// The fluid stack [`fluid_to_nbt`] saved, or `None` when it is malformed or empty.
pub fn fluid_from_nbt(data: &NbtCompound) -> Option<FluidStack> {
    let fluid = Identifier::parse(data.get("Fluid")?.as_str()?).ok()?;
    let amount = data.get("Amount")?.as_i64()?;
    if amount <= 0 {
        return None;
    }
    Some(FluidStack {
        fluid,
        amount: amount as u32,
        tag: data.get("Tag").and_then(Nbt::as_compound).cloned(),
    })
}
