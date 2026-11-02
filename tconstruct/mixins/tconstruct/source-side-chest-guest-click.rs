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
        let mut view=table_items;view.extend(chest::project_side(&side_items,scroll));hub.send_menu_content(gi,view);return Break(Vec::new());
    }
    let raw=usize::try_from(packet.slot).ok();
    let side_slot=raw.filter(|i|(27..75).contains(i)).map(|i|scroll*6+i-27).filter(|i|*i<count);
    let table_slot=raw.filter(|i|*i<2);
    let player=raw.filter(|i|(81..117).contains(i)).map(|i| {let i=i-81;if i<27 {i+9} else {i-27}});
    let mut inventory=hub.guests[gi].inventory.clone();let mut carried=hub.guests[gi].carried.clone();
    let candidate=if packet.input==1 { side_slot.and_then(|i|side_items[i].as_ref()).or_else(||table_slot.and_then(|i|table_items[i].as_ref())).or_else(||player.and_then(|i|inventory[i].as_ref())) } else {carried.as_ref()};
    let max=candidate.map_or(64,|s|rules.max_stack(&s.item).max(1));
    let tags=world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|r|r.as_ref().ok());
    let part=candidate.is_some_and(|s|tags.and_then(|t|t.get("tconstruct:chest_parts")).is_some_and(|v|v.contains(s.item.as_ref())));
    let pattern=candidate.is_some_and(|s|tags.and_then(|t|t.get("tconstruct:patterns")).is_some_and(|v|v.contains(s.item.as_ref())));
    let mut changed=false;
    if matches!(packet.button,0|1) {
        match (packet.input,side_slot,table_slot,player) {
            (0,Some(i),_,_) => changed=chest::pickup_slot(kind,&mut side_items,i,&mut carried,packet.button==1,max,part),
            (0,_,Some(i),_) if i!=0 || carried.is_none() || pattern => changed=chest::pickup_player(&mut table_items,i,&mut carried,packet.button==1,max),
            (0,_,_,Some(i)) => changed=chest::pickup_player(&mut inventory,i,&mut carried,packet.button==1,max),
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
            _=>{},
        }
    }
    if changed {
        if let Some(crate::containers::BlockEntity::Chest(items))=world.block_entities.get_mut(&table) {*items=table_items.clone();} else {return Break(Vec::new());}
        if let Some(crate::containers::BlockEntity::Chest(items))=world.block_entities.get_mut(&side) {*items=side_items.clone();} else {return Break(Vec::new());}
        world.modified.insert((table.0>>4,table.2>>4));world.modified.insert((side.0>>4,side.2>>4));
        hub.guests[gi].inventory=inventory;hub.guests[gi].carried=carried;
    }
    let mut view=table_items;view.extend(chest::project_side(&side_items,scroll));hub.send_menu_content(gi,view);
    Break(Vec::new())
}
