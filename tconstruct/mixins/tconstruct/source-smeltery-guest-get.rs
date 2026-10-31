fn smeltery_guest_get(slots: crate::lan::menu::Slots, world: &crate::world::World, guest: &crate::lan::guest::Guest, index: usize) -> std::ops::ControlFlow<Option<ItemStack>> {
    use std::ops::ControlFlow::{Break, Continue};
    let crate::lan::menu::Slots::Block(pos) = slots else { return Continue(()); };
    let Some(tag) = crate::tconstruct_source_melter_store::read(world, pos, "tconstruct:smeltery") else { return Continue(()); };
    let Some(state) = crate::tconstruct_source_smeltery_state::Smeltery::from_nbt(&tag) else { return Break(None); };
    let offset = world.chunks.__horizon_mixins_tconstruct_smeltery_guest_pages().get(&(guest.conn, guest.placed_at, pos)).map_or(0, |page| page.0);
    Break(crate::tconstruct_source_smeltery_layout::melting_index(state.slots.len(), offset, index).and_then(|i| state.slots[i].clone()))
}
