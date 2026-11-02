fn tconstruct_side_chest_guest_click(hub: &mut crate::lan::hub::Hub, conn: u32, packet: &play::ContainerClick, world: &mut World, rules: &dyn Rules) -> std::ops::ControlFlow<Vec<ItemStack>> {
    use std::ops::ControlFlow::{Break,Continue};
    use crate::tconstruct_chest_rules::{self as chest,ChestKind};
    let Some(gi)=hub.guests.iter().position(|g|g.conn==conn) else { return Continue(()); };
    let Some(menu)=hub.guests[gi].menu.as_ref() else { return Continue(()); };
    let crate::lan::menu::Slots::Double(table,side)=menu.slots else { return Continue(()); };
    let key=(conn,hub.guests[gi].placed_at,menu.window,table);
    let Some(&(scroll,count))=world.chunks.__horizon_mixins_tconstruct_chest_guest_pages().get(&key) else { return Continue(()); };
    if packet.container==0 || packet.container!=menu.window || hub.guests[gi].spectator() || !menu.slots.still_valid(world,&hub.guests[gi],menu.pos) { return Break(Vec::new()); }
    let state=menu.state;
    let Some(crate::containers::BlockEntity::Chest(existing))=world.block_entities.get(&side) else { return Break(Vec::new()); };
    let kind=match existing.len() {256=>ChestKind::Part,64=>ChestKind::Tinkers,_=>return Break(Vec::new())};
    if count!=existing.len() { return Break(Vec::new()); }
    let mut side_items=existing.clone();
    let Some(crate::containers::BlockEntity::Chest(first))=world.block_entities.get(&table) else { return Break(Vec::new()); };
    let mut table_items=first.clone();
    if table_items.len()!=27 { return Break(Vec::new()); }
    if packet.state!=state {
        world.chunks.__horizon_mixins_tconstruct_chest_guest_drag_mut().remove(&key);
        let mut view=table_items;view.extend(chest::project_side(&side_items,scroll));hub.send_menu_content(gi,view);return Break(Vec::new());
    }
    let raw=usize::try_from(packet.slot).ok();
    let side_slot=raw.filter(|i|(27..75).contains(i)).map(|i|scroll*6+i-27).filter(|i|*i<count);
    let table_slot=raw.filter(|i|*i<2);
    let player=raw.filter(|i|(81..117).contains(i)).map(|i| {let i=i-81;if i<27 {i+9} else {i-27}});
    if packet.input==5 {
        let mode=((packet.button>>2)&3) as u8;let phase=packet.button&3;
        if phase==0 {
            world.chunks.__horizon_mixins_tconstruct_chest_guest_drag_mut().remove(&key);
            if hub.guests[gi].carried.is_some() && (mode<2 || mode==2 && hub.guests[gi].creative()) {world.chunks.__horizon_mixins_tconstruct_chest_guest_drag_mut().insert(key,(mode,Vec::new()));}
            return Break(Vec::new());
        }
        if phase==1 {
            let selected=side_slot.map(|i|(true,i)).or_else(||table_slot.map(|i|(false,i))).or_else(||player.map(|i|(false,27+i)));
            if let Some((stored,slots))=world.chunks.__horizon_mixins_tconstruct_chest_guest_drag_mut().get_mut(&key) {
                if *stored==mode {if let Some(selected)=selected {if slots.len()<86 && !slots.contains(&selected) {slots.push(selected);}}}
            }
            return Break(Vec::new());
        }
        let pending=world.chunks.__horizon_mixins_tconstruct_chest_guest_drag_mut().remove(&key);
        if phase!=2 {return Break(Vec::new());}
        let Some((_,mut selected))=pending.filter(|(stored,_)|*stored==mode) else {return Break(Vec::new());};
        let mut combined=table_items.clone();combined.extend(hub.guests[gi].inventory.clone());let mut carried=hub.guests[gi].carried.clone();
        let max=carried.as_ref().map_or(64,|stack|rules.max_stack(&stack.item).max(1));
        let tags=world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|r|r.as_ref().ok());
        let part=carried.as_ref().is_some_and(|s|tags.and_then(|t|t.get("tconstruct:chest_parts")).is_some_and(|v|v.contains(s.item.as_ref())));
        let pattern=carried.as_ref().is_some_and(|s|tags.and_then(|t|t.get("tconstruct:patterns")).is_some_and(|v|v.contains(s.item.as_ref())));
        if !pattern {selected.retain(|entry|*entry!=(false,0));}
        if chest::quick_craft(kind,&mut side_items,&mut combined,&mut carried,&selected,mode,max,part,hub.guests[gi].creative()) {
            if let Some(crate::containers::BlockEntity::Chest(items))=world.block_entities.get_mut(&table) {*items=combined[..27].to_vec();}
            if let Some(crate::containers::BlockEntity::Chest(items))=world.block_entities.get_mut(&side) {*items=side_items.clone();}
            world.modified.insert((table.0>>4,table.2>>4));world.modified.insert((side.0>>4,side.2>>4));hub.guests[gi].inventory=combined[27..].to_vec();hub.guests[gi].carried=carried;
        }
        let mut view=combined[..27].to_vec();view.extend(chest::project_side(&side_items,scroll));hub.send_menu_content(gi,view);
        return Break(Vec::new());
    }
    let mut inventory=hub.guests[gi].inventory.clone();let mut carried=hub.guests[gi].carried.clone();let mut offhand=hub.guests[gi].offhand.clone();
    let candidate=if packet.input==2 {if packet.button==40 {offhand.as_ref()} else {usize::try_from(packet.button).ok().and_then(|i|inventory.get(i)).and_then(Option::as_ref)}} else if packet.input==1 { side_slot.and_then(|i|side_items[i].as_ref()).or_else(||table_slot.and_then(|i|table_items[i].as_ref())).or_else(||player.and_then(|i|inventory[i].as_ref())) } else {carried.as_ref()};
    let max=candidate.map_or(64,|s|rules.max_stack(&s.item).max(1));
    let tags=world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|r|r.as_ref().ok());
    let part=candidate.is_some_and(|s|tags.and_then(|t|t.get("tconstruct:chest_parts")).is_some_and(|v|v.contains(s.item.as_ref())));
    let pattern=candidate.is_some_and(|s|tags.and_then(|t|t.get("tconstruct:patterns")).is_some_and(|v|v.contains(s.item.as_ref())));
    let candidate_empty=candidate.is_none();
    let mut changed=false;
    let mut dropped=None;
    if matches!(packet.button,0|1) || matches!(packet.input,2|3) {
        match (packet.input,side_slot,table_slot,player) {
            (0,Some(i),_,_) => changed=chest::pickup_slot(kind,&mut side_items,i,&mut carried,packet.button==1,max,part),
            (0,_,Some(i),_) if i!=0 || carried.is_none() || pattern => changed=chest::pickup_player(&mut table_items,i,&mut carried,packet.button==1,max),
            (0,_,_,Some(i)) => changed=chest::pickup_player(&mut inventory,i,&mut carried,packet.button==1,max),
            (0,None,None,None) if packet.slot==-999 => {
                let mut cursor=vec![carried.take()];dropped=chest::take_for_drop(&mut cursor,0,packet.button==0);carried=cursor.pop().flatten();changed=dropped.is_some();
            },
            (1,_,_,Some(i)) => if let Some(stack)=inventory[i].take() {
                let old=stack.count;let rem=crate::tconstruct_tx::source_quick_move_to_builder(&mut table_items,&mut [],stack,max,max>1,pattern);
                let rem=rem.and_then(|stack|chest::insert_across(kind,&mut side_items,stack,max,part));
                changed=rem.as_ref().map_or(0,|s|s.count)!=old;inventory[i]=rem;
            },
            (1,Some(i),_,_) => if let Some(stack)=side_items[i].take() {
                let old=stack.count;let rem=crate::tconstruct_tx::source_quick_move_player(&mut inventory,stack,max,max>1);
                changed=rem.as_ref().map_or(0,|s|s.count)!=old;side_items[i]=rem;
            },
            (1,_,Some(i),_) => if let Some(stack)=table_items[i].take() {
                let old=stack.count;let rem=crate::tconstruct_tx::source_quick_move_player(&mut inventory,stack,max,max>1);
                changed=rem.as_ref().map_or(0,|s|s.count)!=old;table_items[i]=rem;
            },
            (2,Some(i),_,_) => if let Some(left)=chest::swap_slot(kind,&mut side_items,i,&mut inventory,&mut offhand,i32::from(packet.button),max,part,|inventory,stack| {
                let limit=rules.max_stack(&stack.item).max(1);crate::tconstruct_tx::source_quick_move_player(inventory,stack,limit,limit>1)
            }) {changed=true;dropped=left;},
            (2,_,Some(i),_) if i!=0 || candidate_empty || pattern => {
                let kind=ChestKind::Plain(table_items.len());
                if let Some(left)=chest::swap_slot(kind,&mut table_items,i,&mut inventory,&mut offhand,i32::from(packet.button),max,true,|inventory,stack| {
                    let limit=rules.max_stack(&stack.item).max(1);crate::tconstruct_tx::source_quick_move_player(inventory,stack,limit,limit>1)
                }) {changed=true;dropped=left;}
            },
            (2,_,_,Some(i)) => {
                if packet.button==40 {std::mem::swap(&mut inventory[i],&mut offhand);changed=true;}
                else if (0..9).contains(&packet.button) {inventory.swap(i,packet.button as usize);changed=true;}
            },
            (4,Some(i),_,_) if carried.is_none() => {dropped=chest::take_for_drop(&mut side_items,i,packet.button!=0);changed=dropped.is_some();},
            (4,_,Some(i),_) if carried.is_none() => {dropped=chest::take_for_drop(&mut table_items,i,packet.button!=0);changed=dropped.is_some();},
            (4,_,_,Some(i)) if carried.is_none() => {dropped=chest::take_for_drop(&mut inventory,i,packet.button!=0);changed=dropped.is_some();},
            (3,side,table,player) if hub.guests[gi].creative() && carried.is_none() => {
                let stack=side.and_then(|i|side_items[i].as_ref()).or_else(||table.and_then(|i|table_items[i].as_ref())).or_else(||player.and_then(|i|inventory[i].as_ref()));
                if let Some(stack)=stack {let mut copy=stack.clone();copy.count=rules.max_stack(&copy.item).max(1);carried=Some(copy);changed=true;}
            },
            (6,side,table_slot,player) if carried.is_some() => {
                let clicked=side.and_then(|i|side_items[i].as_ref()).or_else(||table_slot.and_then(|i|table_items[i].as_ref())).or_else(||player.and_then(|i|inventory[i].as_ref()));
                if clicked.is_none() && (side.is_some() || table_slot.is_some() || player.is_some()) {
                    let mut combined=table_items.clone();combined.extend(inventory.clone());let start=scroll*6;
                    let mut order:Vec<_>=(0..2).map(|i|(false,i)).chain((start..(start+48).min(count)).map(|i|(true,i))).chain((9..36).chain(0..9).map(|i|(false,27+i))).collect();
                    if packet.button!=0 {order.reverse();}
                    changed=chest::pickup_all_ordered(&mut side_items,&mut combined,&mut carried,&order,max);
                    if changed {table_items=combined[..27].to_vec();inventory=combined[27..].to_vec();}
                }
            },
            _=>{},
        }
    }
    if changed {
        if let Some(crate::containers::BlockEntity::Chest(items))=world.block_entities.get_mut(&table) {*items=table_items.clone();} else {return Break(Vec::new());}
        if let Some(crate::containers::BlockEntity::Chest(items))=world.block_entities.get_mut(&side) {*items=side_items.clone();} else {return Break(Vec::new());}
        world.modified.insert((table.0>>4,table.2>>4));world.modified.insert((side.0>>4,side.2>>4));
        hub.guests[gi].inventory=inventory;hub.guests[gi].carried=carried;hub.guests[gi].offhand=offhand;
        if packet.input==2 && packet.button==40 {
            let item=hub.stack(&hub.guests[gi].offhand.clone());let guest=&mut hub.guests[gi];guest.menu_state=(guest.menu_state+1)&0x7fff;
            guest.send(&C::ContainerSetSlot(play::ContainerSetSlot {container:0,state:guest.menu_state,slot:45,item}));
        }
    }
    let mut view=table_items;view.extend(chest::project_side(&side_items,scroll));hub.send_menu_content(gi,view);
    Break(dropped.into_iter().collect())
}
