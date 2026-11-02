fn tconstruct_chest_guest_read(slots: crate::lan::menu::Slots, world: &crate::world::World, guest: &crate::lan::guest::Guest) -> std::ops::ControlFlow<Option<Vec<Option<ItemStack>>>> {
    use std::ops::ControlFlow::{Break,Continue};
    let (table,side)=match slots {crate::lan::menu::Slots::Block(pos)=>(pos,None),crate::lan::menu::Slots::Double(table,side)=>(table,Some(side)),_=>return Continue(())};
    let Some(window)=guest.menu.as_ref().filter(|m|m.pos==Some(table)).map(|m|m.window) else { return Continue(()); };
    let Some(&(offset,_))=world.chunks.__horizon_mixins_tconstruct_chest_guest_pages().get(&(guest.conn,guest.placed_at,window,table)) else { return Continue(()); };
    let Some(entity)=world.block_entities.get(&side.unwrap_or(table)) else { return Break(None); };
    if let Some(_)=side {
        let Some(first)=world.block_entities.get(&table) else { return Break(None); };
        let mut result=first.slots().to_vec();
        result.extend(crate::tconstruct_chest_rules::project_side(entity.slots(),offset));
        Break(Some(result))
    } else { Break(Some(crate::tconstruct_chest_rules::project(entity.slots(),offset))) }
}
