fn tconstruct_chest_guest_button(game: &mut crate::game::Game<'_>, host: &mut crate::game::lan::LanHost, conn: u32, window: i32, id: i32) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break,Continue};
    if (42000..=42043).contains(&id) {
        let Some(gi)=host.hub.guests.iter().position(|g|g.conn==conn) else { return Continue(()); };
        let Some(menu)=host.hub.guests[gi].menu.as_ref() else { return Continue(()); };
        let horizon_client::lan::menu::Slots::Double(table,side)=menu.slots else { return Continue(()); };
        let key=(conn,host.hub.guests[gi].placed_at,menu.window,table);
        if !game.world.chunks.__horizon_mixins_tconstruct_chest_guest_pages().contains_key(&key) { return Continue(()); }
        if menu.window!=window || host.hub.guests[gi].spectator() || !menu.slots.still_valid(&game.world,&host.hub.guests[gi],menu.pos) { return Break(()); }
        let (Some(first),Some(second))=(game.world.block_entities.get(&table),game.world.block_entities.get(&side)) else { return Break(()); };
        let count=second.slots().len();if !matches!(count,64|256) || first.slots().len()!=27 {return Break(());}
        let next=((id-42000) as usize).min(crate::game::tconstruct_source_side_layout::max_offset(count));
        let mut items=first.slots().to_vec();items.extend(crate::game::tconstruct_chest_rules::project_side(second.slots(),next));
        game.world.chunks.__horizon_mixins_tconstruct_chest_guest_pages_mut().insert(key,(next,count));
        items.extend((9..36).chain(0..9).map(|i|host.hub.guests[gi].inventory[i].clone()));
        let net=items.iter().map(|item|host.hub.stack(item)).collect();
        let carried=host.hub.stack(&host.hub.guests[gi].carried.clone());let guest=&mut host.hub.guests[gi];
        let Some(menu)=guest.menu.as_mut() else { return Break(()); };menu.state=(menu.state+1)&0x7fff;
        let content=horizon_protocol::packets::play::ContainerSetContent {container:window,state:menu.state,items:net,carried};menu.sent=items;
        guest.send(&horizon_protocol::packets::play::Clientbound::ContainerSetData(horizon_protocol::packets::play::ContainerSetData {container:window,id:30103,value:next as i16}));
        guest.send(&horizon_protocol::packets::play::Clientbound::ContainerSetContent(content));
        return Break(());
    }
    if !(41000..=41028).contains(&id) { return Continue(()); }
    let Some(gi)=host.hub.guests.iter().position(|g|g.conn==conn) else { return Continue(()); };
    let Some(menu)=host.hub.guests[gi].menu.as_ref() else { return Continue(()); };
    let Some(pos)=menu.pos else { return Continue(()); };
    let key=(conn,host.hub.guests[gi].placed_at,menu.window,pos);
    if !game.world.chunks.__horizon_mixins_tconstruct_chest_guest_pages().contains_key(&key) { return Continue(()); }
    if menu.window!=window || host.hub.guests[gi].spectator() || !menu.slots.still_valid(&game.world,&host.hub.guests[gi],menu.pos) { return Break(()); }
    let Some(horizon_client::containers::BlockEntity::Chest(slots))=game.world.block_entities.get(&pos) else { return Break(()); };
    let visual=match slots.len() { 256 => crate::game::tconstruct_chest_rules::part_visual_size(slots.iter().map(Option::is_some),slots.len()),64 => 64,_ => return Break(()) };
    let next=((id-41000) as usize).min(crate::game::tconstruct_chest_rules::max_scroll(visual));
    let mut items=crate::game::tconstruct_chest_rules::project(slots,next);
    game.world.chunks.__horizon_mixins_tconstruct_chest_guest_pages_mut().insert(key,(next,visual));
    items.extend((9..36).chain(0..9).map(|i|host.hub.guests[gi].inventory[i].clone()));
    let net=items.iter().map(|item|host.hub.stack(item)).collect();
    let carried=host.hub.stack(&host.hub.guests[gi].carried.clone());
    let guest=&mut host.hub.guests[gi];
    let Some(menu)=guest.menu.as_mut() else { return Break(()); };
    menu.state=(menu.state+1)&0x7fff;
    let content=horizon_protocol::packets::play::ContainerSetContent { container:window,state:menu.state,items:net,carried };
    menu.sent=items;
    guest.send(&horizon_protocol::packets::play::Clientbound::ContainerSetData(horizon_protocol::packets::play::ContainerSetData { container:window,id:30101,value:visual as i16 }));
    guest.send(&horizon_protocol::packets::play::Clientbound::ContainerSetData(horizon_protocol::packets::play::ContainerSetData { container:window,id:30102,value:next as i16 }));
    guest.send(&horizon_protocol::packets::play::Clientbound::ContainerSetContent(content));
    Break(())
}
