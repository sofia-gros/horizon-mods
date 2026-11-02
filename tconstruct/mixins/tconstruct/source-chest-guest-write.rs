fn tconstruct_chest_guest_write(slots: crate::lan::menu::Slots, world: &mut crate::world::World, guest: &crate::lan::guest::Guest, items: &[Option<ItemStack>]) -> std::ops::ControlFlow<()> {
    let _ = items;
    let crate::lan::menu::Slots::Block(pos)=slots else { return std::ops::ControlFlow::Continue(()); };
    let Some(window)=guest.menu.as_ref().filter(|m|m.pos==Some(pos)).map(|m|m.window) else { return std::ops::ControlFlow::Continue(()); };
    if world.chunks.__horizon_mixins_tconstruct_chest_guest_pages().contains_key(&(guest.conn,guest.placed_at,window,pos)) { std::ops::ControlFlow::Break(()) } else { std::ops::ControlFlow::Continue(()) }
}
