fn tconstruct_chest_guest_get(slots: crate::lan::menu::Slots, world: &crate::world::World, guest: &crate::lan::guest::Guest, index: usize) -> std::ops::ControlFlow<Option<ItemStack>> {
    use std::ops::ControlFlow::{Break,Continue};
    let (table,side)=match slots {crate::lan::menu::Slots::Block(pos)=>(pos,None),crate::lan::menu::Slots::Double(table,side)=>(table,Some(side)),_=>return Continue(())};
    let Some(window)=guest.menu.as_ref().filter(|m|m.pos==Some(table)).map(|m|m.window) else { return Continue(()); };
    let Some(&(offset,_))=world.chunks.__horizon_mixins_tconstruct_chest_guest_pages().get(&(guest.conn,guest.placed_at,window,table)) else { return Continue(()); };
    if let Some(side)=side {
        if index<27 { return Break(world.block_entities.get(&table).and_then(|e|e.slots().get(index)).cloned().flatten()); }
        let i=index-27;
        return Break(if i<48 {world.block_entities.get(&side).and_then(|e|e.slots().get(offset*6+i)).cloned().flatten()} else {None});
    }
    Break(if index<32 {world.block_entities.get(&table).and_then(|e|e.slots().get(offset*8+index)).cloned().flatten()} else {None})
}
