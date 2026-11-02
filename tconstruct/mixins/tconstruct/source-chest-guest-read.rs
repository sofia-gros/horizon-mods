fn tconstruct_chest_guest_read(slots: crate::lan::menu::Slots, world: &crate::world::World, guest: &crate::lan::guest::Guest) -> std::ops::ControlFlow<Option<Vec<Option<ItemStack>>>> {
    let crate::lan::menu::Slots::Block(pos)=slots else { return std::ops::ControlFlow::Continue(()); };
    let Some(window)=guest.menu.as_ref().filter(|m|m.pos==Some(pos)).map(|m|m.window) else { return std::ops::ControlFlow::Continue(()); };
    let Some(&(offset,_))=world.chunks.__horizon_mixins_tconstruct_chest_guest_pages().get(&(guest.conn,guest.placed_at,window,pos)) else { return std::ops::ControlFlow::Continue(()); };
    let result=world.block_entities.get(&pos).map(|entity| crate::tconstruct_chest_rules::project(entity.slots(),offset));
    std::ops::ControlFlow::Break(result)
}
