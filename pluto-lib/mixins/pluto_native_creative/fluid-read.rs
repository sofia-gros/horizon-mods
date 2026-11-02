//! Read-only access to the host's registered block-entity fluid tanks.
use crate::{containers::BlockEntity, world::World};
use horizon_core::geometry::Direction;
use horizon_world::nbt::Tag;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FluidTank {
    pub fluid: Option<String>,
    pub amount: i32,
    pub capacity: i32,
}

/// The first tank exposed on `face`, using the host's native tank registry
/// and persisted `fluids` representation. No fluid is inserted or removed.
pub fn first(world: &World, pos: (i32,i32,i32), face: Direction) -> Option<FluidTank> {
    if !world.loaded(pos.0, pos.2) { return None; }
    let state = world.block(pos.0,pos.1,pos.2);
    let tanks = world.block_entities.mod_fluids(state.0)?;
    let index = *tanks.faces[face as usize].first()?;
    let capacity = *tanks.tanks.get(index)?;
    let BlockEntity::Mod(entity) = world.block_entities.get(&pos)? else { return None; };
    if entity.id.is_empty() { return None; }
    let saved = match entity.nbt.get(&"fluids".into()) { Some(Tag::List(values)) => values.get(index), _ => None };
    let fluid = match saved.and_then(|value| value.get("Fluid")) {
        Some(Tag::String(id)) if id.to_string_lossy() != "minecraft:empty" => Some(id.to_string_lossy()),
        _ => None,
    };
    let amount = match saved.and_then(|value| value.get("Amount")) { Some(Tag::Int(value)) => *value, _ => 0 };
    Some(FluidTank { fluid, amount, capacity })
}
