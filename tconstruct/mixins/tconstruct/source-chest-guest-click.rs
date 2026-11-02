fn tconstruct_chest_guest_click(hub: &mut crate::lan::hub::Hub, conn: u32, packet: &play::ContainerClick, world: &mut World, rules: &dyn Rules) -> std::ops::ControlFlow<Vec<ItemStack>> {
    use std::ops::ControlFlow::{Break,Continue};
    use crate::tconstruct_chest_rules::{self as chest,ChestKind};
    let Some(gi)=hub.guests.iter().position(|g|g.conn==conn) else { return Continue(()); };
    let Some(menu)=hub.guests[gi].menu.as_ref() else { return Continue(()); };
    let Some(pos)=menu.pos else { return Continue(()); };
    let key=(conn,hub.guests[gi].placed_at,menu.window,pos);
    let Some(&(scroll,known_count))=world.chunks.__horizon_mixins_tconstruct_chest_guest_pages().get(&key) else { return Continue(()); };
    if packet.container==0 || packet.container!=menu.window || hub.guests[gi].spectator() || !menu.slots.still_valid(world,&hub.guests[gi],menu.pos) { return Break(Vec::new()); }
    let state=menu.state;
    let Some(crate::containers::BlockEntity::Chest(existing))=world.block_entities.get(&pos) else { return Break(Vec::new()); };
    let kind=match existing.len() { 256=>ChestKind::Part,64=>ChestKind::Tinkers,_=>return Break(Vec::new()) };
    let mut contents=existing.clone();
    let visual=if kind==ChestKind::Part { chest::part_visual_size(contents.iter().map(Option::is_some),contents.len()) } else { contents.len() };
    let scroll=scroll.min(chest::max_scroll(visual));
    if known_count!=visual || packet.state!=state {
        world.chunks.__horizon_mixins_tconstruct_chest_guest_pages_mut().insert(key,(scroll,visual));
        hub.guests[gi].send(&C::ContainerSetData(play::ContainerSetData { container:packet.container,id:30101,value:visual as i16 }));
        hub.guests[gi].send(&C::ContainerSetData(play::ContainerSetData { container:packet.container,id:30102,value:scroll as i16 }));
        hub.send_menu_content(gi,chest::project(&contents,scroll));
        return Break(Vec::new());
    }
    let raw=usize::try_from(packet.slot).ok();
    let storage=raw.filter(|i|*i<32).map(|i|scroll*8+i).filter(|i|*i<visual);
    let player=raw.filter(|i|(54..90).contains(i)).map(|i| { let i=i-54; if i<27 {i+9} else {i-27} });
    let mut inventory=hub.guests[gi].inventory.clone();
    let mut carried=hub.guests[gi].carried.clone();
    let candidate=if packet.input==1 { storage.and_then(|i|contents[i].as_ref()).or_else(||player.and_then(|i|inventory[i].as_ref())) } else { carried.as_ref() };
    let max=candidate.map_or(64,|s|rules.max_stack(&s.item).max(1));
    let tagged=candidate.is_some_and(|s| world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|r|r.as_ref().ok()).and_then(|tags|tags.get("tconstruct:chest_parts")).is_some_and(|items|items.contains(s.item.as_ref())));
    let mut changed=false;
    if matches!(packet.button,0|1) {
        match (packet.input,storage,player) {
            (0,Some(i),_) => changed=chest::pickup_slot(kind,&mut contents,i,&mut carried,packet.button==1,max,tagged),
            (0,_,Some(i)) => changed=chest::pickup_player(&mut inventory,i,&mut carried,packet.button==1,max),
            (1,Some(i),_) => if let Some(stack)=contents[i].take() {
                let old=stack.count;let remaining=crate::tconstruct_tx::source_quick_move_player(&mut inventory,stack,max,max>1);
                changed=remaining.as_ref().map_or(0,|s|s.count)!=old;contents[i]=remaining;
            },
            (1,_,Some(i)) => if let Some(stack)=inventory[i].take() {
                let old=stack.count;let remaining=chest::insert_across(kind,&mut contents,stack,max,tagged);
                changed=remaining.as_ref().map_or(0,|s|s.count)!=old;inventory[i]=remaining;
            },
            _=>{},
        }
    }
    if changed {
        if let Some(crate::containers::BlockEntity::Chest(live))=world.block_entities.get_mut(&pos) { *live=contents.clone(); }
        else { return Break(Vec::new()); }
        world.modified.insert((pos.0>>4,pos.2>>4));
        hub.guests[gi].inventory=inventory;hub.guests[gi].carried=carried;
    }
    let new_visual=if kind==ChestKind::Part { chest::part_visual_size(contents.iter().map(Option::is_some),contents.len()) } else { contents.len() };
    let new_scroll=scroll.min(chest::max_scroll(new_visual));
    if new_visual!=known_count || new_scroll!=scroll {
        world.chunks.__horizon_mixins_tconstruct_chest_guest_pages_mut().insert(key,(new_scroll,new_visual));
        hub.guests[gi].send(&C::ContainerSetData(play::ContainerSetData { container:packet.container,id:30101,value:new_visual as i16 }));
        hub.guests[gi].send(&C::ContainerSetData(play::ContainerSetData { container:packet.container,id:30102,value:new_scroll as i16 }));
    }
    hub.send_menu_content(gi,chest::project(&contents,new_scroll));
    Break(Vec::new())
}
