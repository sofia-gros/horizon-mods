fn tconstruct_double_menu_get(slots: crate::lan::menu::Slots, world: &crate::world::World, index: usize) -> std::ops::ControlFlow<Option<ItemStack>> {
    use std::ops::ControlFlow::{Break, Continue};
    let crate::lan::menu::Slots::Double(table, side) = slots else { return Continue(()); };
    let Some(partner) = world.chunks.__horizon_mixins_tconstruct_builder_double_partner().get(&table) else { return Continue(()); };
    let (Some(first), Some(second)) = (world.block_entities.get(&table), world.block_entities.get(&side)) else { return Break(None); };
    let first_two = first.slots().len() + second.slots().len();
    if index < first_two { return Continue(()); }
    Break(world.block_entities.get(partner).and_then(|entity| entity.slots().get(index - first_two)).cloned().flatten())
}
