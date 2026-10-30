fn tconstruct_double_menu_write(slots: crate::lan::menu::Slots, world: &mut crate::world::World, items: &[Option<ItemStack>]) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let crate::lan::menu::Slots::Double(table, side) = slots else { return Continue(()); };
    let Some(&partner) = world.chunks.__horizon_mixins_tconstruct_builder_double_partner().get(&table) else { return Continue(()); };
    if [table, side, partner].iter().any(|pos| !world.block_entities.contains_key(pos)) { return Break(()); }
    let mut items = items.iter().cloned();
    for pos in [table, side, partner] {
        if let Some(entity) = world.block_entities.get_mut(&pos) {
            for slot in entity.slots_mut() { *slot = items.next().flatten(); }
        }
    }
    Break(())
}
