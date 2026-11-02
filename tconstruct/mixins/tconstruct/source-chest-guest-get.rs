fn tconstruct_chest_guest_get(slots: crate::lan::menu::Slots, world: &crate::world::World, guest: &crate::lan::guest::Guest, index: usize) -> std::ops::ControlFlow<Option<ItemStack>> {
    let crate::lan::menu::Slots::Block(pos)=slots else { return std::ops::ControlFlow::Continue(()); };
    let Some(window)=guest.menu.as_ref().filter(|m|m.pos==Some(pos)).map(|m|m.window) else { return std::ops::ControlFlow::Continue(()); };
    let Some(&(offset,_))=world.chunks.__horizon_mixins_tconstruct_chest_guest_pages().get(&(guest.conn,guest.placed_at,window,pos)) else { return std::ops::ControlFlow::Continue(()); };
    let item=if index<32 { world.block_entities.get(&pos).and_then(|entity|entity.slots().get(offset*8+index)).cloned().flatten() } else { None };
    std::ops::ControlFlow::Break(item)
}
