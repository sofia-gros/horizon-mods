fn tconstruct_chest_guest_click(hub: &mut crate::lan::hub::Hub, conn: u32, packet: &play::ContainerClick, world: &mut World, rules: &dyn Rules) -> std::ops::ControlFlow<Vec<ItemStack>> {
    use std::ops::ControlFlow::{Break,Continue};
    use crate::tconstruct_chest_rules::{self as chest,ChestKind};
    let Some(gi)=hub.guests.iter().position(|g|g.conn==conn) else { return Continue(()); };
    let Some(menu)=hub.guests[gi].menu.as_ref() else { return Continue(()); };
    if !matches!(menu.slots,crate::lan::menu::Slots::Block(_)) { return Continue(()); }
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
        world.chunks.__horizon_mixins_tconstruct_chest_guest_drag_mut().remove(&key);
        world.chunks.__horizon_mixins_tconstruct_chest_guest_pages_mut().insert(key,(scroll,visual));
        hub.guests[gi].send(&C::ContainerSetData(play::ContainerSetData { container:packet.container,id:30101,value:visual as i16 }));
        hub.guests[gi].send(&C::ContainerSetData(play::ContainerSetData { container:packet.container,id:30102,value:scroll as i16 }));
        hub.send_menu_content(gi,chest::project(&contents,scroll));
        return Break(Vec::new());
    }
    let raw=usize::try_from(packet.slot).ok();
    let storage=raw.filter(|i|*i<32).map(|i|scroll*8+i).filter(|i|*i<visual);
    let player=raw.filter(|i|(54..90).contains(i)).map(|i| { let i=i-54; if i<27 {i+9} else {i-27} });
    if packet.input==5 {
        let mode=((packet.button>>2)&3) as u8;let phase=packet.button&3;
        if phase==0 {
            world.chunks.__horizon_mixins_tconstruct_chest_guest_drag_mut().remove(&key);
            if hub.guests[gi].carried.is_some() && (mode<2 || mode==2 && hub.guests[gi].creative()) {
                world.chunks.__horizon_mixins_tconstruct_chest_guest_drag_mut().insert(key,(mode,Vec::new()));
            }
            return Break(Vec::new());
        }
        if phase==1 {
            if let Some((stored,slots))=world.chunks.__horizon_mixins_tconstruct_chest_guest_drag_mut().get_mut(&key) {
                if *stored==mode {if let Some(index)=storage {slots.push((true,index));} else if let Some(index)=player {slots.push((false,index));}}
            }
            return Break(Vec::new());
        }
        let pending=world.chunks.__horizon_mixins_tconstruct_chest_guest_drag_mut().remove(&key);
        if phase!=2 {return Break(Vec::new());}
        let Some((stored,selected))=pending.filter(|(stored,_)|*stored==mode) else {return Break(Vec::new());};
        let mut inventory=hub.guests[gi].inventory.clone();let mut carried=hub.guests[gi].carried.clone();
        let max=carried.as_ref().map_or(64,|stack|rules.max_stack(&stack.item).max(1));
        let tagged=carried.as_ref().is_some_and(|stack|world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|r|r.as_ref().ok()).and_then(|t|t.get("tconstruct:chest_parts")).is_some_and(|v|v.contains(stack.item.as_ref())));
        if chest::quick_craft(kind,&mut contents,&mut inventory,&mut carried,&selected,stored,max,tagged,hub.guests[gi].creative()) {
            if let Some(crate::containers::BlockEntity::Chest(live))=world.block_entities.get_mut(&pos) {*live=contents.clone();}
            world.modified.insert((pos.0>>4,pos.2>>4));hub.guests[gi].inventory=inventory;hub.guests[gi].carried=carried;
        }
        let visual=if kind==ChestKind::Part {chest::part_visual_size(contents.iter().map(Option::is_some),contents.len())} else {contents.len()};
        let scroll=scroll.min(chest::max_scroll(visual));world.chunks.__horizon_mixins_tconstruct_chest_guest_pages_mut().insert(key,(scroll,visual));
        hub.guests[gi].send(&C::ContainerSetData(play::ContainerSetData {container:packet.container,id:30101,value:visual as i16}));
        hub.guests[gi].send(&C::ContainerSetData(play::ContainerSetData {container:packet.container,id:30102,value:scroll as i16}));
        hub.send_menu_content(gi,chest::project(&contents,scroll));
        return Break(Vec::new());
    }
    let mut inventory=hub.guests[gi].inventory.clone();
    let mut carried=hub.guests[gi].carried.clone();
    let mut offhand=hub.guests[gi].offhand.clone();
    let candidate=if packet.input==2 {if packet.button==40 {offhand.as_ref()} else {usize::try_from(packet.button).ok().and_then(|i|inventory.get(i)).and_then(Option::as_ref)}} else if packet.input==1 { storage.and_then(|i|contents[i].as_ref()).or_else(||player.and_then(|i|inventory[i].as_ref())) } else { carried.as_ref() };
    let max=candidate.map_or(64,|s|rules.max_stack(&s.item).max(1));
    let tagged=candidate.is_some_and(|s| world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|r|r.as_ref().ok()).and_then(|tags|tags.get("tconstruct:chest_parts")).is_some_and(|items|items.contains(s.item.as_ref())));
    let mut changed=false;
    let mut dropped=None;
    if matches!(packet.button,0|1) || matches!(packet.input,2|3) {
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
            (2,Some(i),_) => if let Some(left)=chest::swap_slot(kind,&mut contents,i,&mut inventory,&mut offhand,i32::from(packet.button),max,tagged,|inventory,stack| {
                let limit=rules.max_stack(&stack.item).max(1);crate::tconstruct_tx::source_quick_move_player(inventory,stack,limit,limit>1)
            }) {changed=true;dropped=left;},
            (2,_,Some(i)) => {
                if packet.button==40 {std::mem::swap(&mut inventory[i],&mut offhand);changed=true;}
                else if (0..9).contains(&packet.button) {inventory.swap(i,packet.button as usize);changed=true;}
            },
            (0,None,None) if packet.slot==-999 => {
                let mut cursor=vec![carried.take()];dropped=chest::take_for_drop(&mut cursor,0,packet.button==0);carried=cursor.pop().flatten();changed=dropped.is_some();
            },
            (4,Some(i),_) if carried.is_none() => {dropped=chest::take_for_drop(&mut contents,i,packet.button!=0);changed=dropped.is_some();},
            (4,_,Some(i)) if carried.is_none() => {dropped=chest::take_for_drop(&mut inventory,i,packet.button!=0);changed=dropped.is_some();},
            (3,storage,player) if hub.guests[gi].creative() && carried.is_none() => {
                let stack=storage.and_then(|i|contents[i].as_ref()).or_else(||player.and_then(|i|inventory[i].as_ref()));
                if let Some(stack)=stack {let mut copy=stack.clone();copy.count=rules.max_stack(&copy.item).max(1);carried=Some(copy);changed=true;}
            },
            (6,storage,player) if carried.is_some() => {
                let clicked=storage.and_then(|i|contents[i].as_ref()).or_else(||player.and_then(|i|inventory[i].as_ref()));
                if clicked.is_none() && (storage.is_some() || player.is_some()) {
                    let start=scroll*chest::CHEST_COLUMNS;let visible:Vec<_>=(start..(start+32).min(visual)).collect();
                    changed=chest::pickup_all(&mut contents,&visible,&mut inventory,&mut carried,packet.button!=0,max);
                }
            },
            _=>{},
        }
    }
    if changed {
        if let Some(crate::containers::BlockEntity::Chest(live))=world.block_entities.get_mut(&pos) { *live=contents.clone(); }
        else { return Break(Vec::new()); }
        world.modified.insert((pos.0>>4,pos.2>>4));
        hub.guests[gi].inventory=inventory;hub.guests[gi].carried=carried;hub.guests[gi].offhand=offhand;
        if packet.input==2 && packet.button==40 {
            let item=hub.stack(&hub.guests[gi].offhand.clone());
            let guest=&mut hub.guests[gi];guest.menu_state=(guest.menu_state+1)&0x7fff;
            guest.send(&C::ContainerSetSlot(play::ContainerSetSlot {container:0,state:guest.menu_state,slot:45,item}));
        }
    }
    let new_visual=if kind==ChestKind::Part { chest::part_visual_size(contents.iter().map(Option::is_some),contents.len()) } else { contents.len() };
    let new_scroll=scroll.min(chest::max_scroll(new_visual));
    if new_visual!=known_count || new_scroll!=scroll {
        world.chunks.__horizon_mixins_tconstruct_chest_guest_pages_mut().insert(key,(new_scroll,new_visual));
        hub.guests[gi].send(&C::ContainerSetData(play::ContainerSetData { container:packet.container,id:30101,value:new_visual as i16 }));
        hub.guests[gi].send(&C::ContainerSetData(play::ContainerSetData { container:packet.container,id:30102,value:new_scroll as i16 }));
    }
    hub.send_menu_content(gi,chest::project(&contents,new_scroll));
    Break(dropped.into_iter().collect())
}
