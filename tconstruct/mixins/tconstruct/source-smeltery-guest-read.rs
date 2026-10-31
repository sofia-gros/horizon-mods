fn smeltery_guest_read(slots: crate::lan::menu::Slots, world: &crate::world::World, guest: &crate::lan::guest::Guest) -> std::ops::ControlFlow<Option<Vec<Option<ItemStack>>>> {
    use std::ops::ControlFlow::{Break, Continue};
    let crate::lan::menu::Slots::Block(pos) = slots else { return Continue(()); };
    let Some(tag) = crate::tconstruct_source_melter_store::read(world, pos, "tconstruct:smeltery") else { return Continue(()); };
    let Some(state) = crate::tconstruct_source_smeltery_state::Smeltery::from_nbt(&tag) else { return Break(None); };
    let offset = world.chunks.__horizon_mixins_tconstruct_smeltery_guest_pages().get(&(guest.conn, guest.placed_at, pos)).copied().unwrap_or(0);
    Break(Some(crate::tconstruct_source_smeltery_page::project(&state.slots, offset)))
}
