fn tconstruct_hopper_menu(slots: crate::lan::menu::Slots, world: &crate::world::World) -> std::ops::ControlFlow<Option<Vec<Option<ItemStack>>>> {
    use std::ops::ControlFlow::{Break, Continue};
    let crate::lan::menu::Slots::Double(table, side) = slots else { return Continue(()); };
    let (Some(first), Some(second)) = (world.block_entities.get(&table), world.block_entities.get(&side)) else { return Continue(()); };
    if first.slots().len() != 27 || second.slots().len() != 5 { return Continue(()); }
    let mut result = first.slots().to_vec();
    result.extend_from_slice(second.slots());
    result.resize(36, None);
    Break(Some(result))
}
