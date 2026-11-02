// Host-authoritative guest crafting sessions reuse the verified crafting menu.
use super::*;
use std::ops::ControlFlow::{self, Break, Continue};

type Pos = (i32, i32, i32);
fn station(game: &Game<'_>, pos: Pos) -> bool {
    &*game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name == "tconstruct:crafting_station"
}
fn items(game: &Game<'_>, pos: Pos) -> Option<Vec<Option<ItemStack>>> {
    let BlockEntity::Chest(items) = game.world.block_entities.get(&pos)? else { return None; };
    (items.len() == 9).then(|| items.clone())
}
fn session(game: &Game<'_>, conn: u32, joined: u64, window: i32) -> Option<Pos> {
    game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_sessions().get(&(conn, joined, window)).copied()
}

pub(crate) fn open(game: &mut Game<'_>, host: &mut LanHost, conn: u32, pos: Pos) -> bool {
    game.ensure_datapacks();
    game.world.block_entities.get_or_insert_with(pos, || BlockEntity::Chest(vec![None; 9]));
    let Some(grid) = items(game, pos) else { return false; };
    game.lan_crafting_closed(host, conn, None);
    game.lan_station_closed(host, conn, None);
    game.guest_container_closed(host, conn);
    let Some(gi) = host.hub.guests.iter().position(|guest| guest.conn == conn) else { return false; };
    let joined = host.hub.guests[gi].placed_at;
    let Some(window) = host.hub.open_window(conn, "minecraft:crafting", horizon_protocol::text::literal("TConstruct Crafting Station")) else { return false; };
    let mut menu = Box::new(Menu::new(Kind::Crafting));
    game.guest_menu_op(host, gi, &mut menu, |menu, env| {
        for (i, stack) in grid.into_iter().enumerate() { menu.set_slot(env, i + 1, stack); }
    });
    host.play.entry(conn).or_default().crafting.table = Some((window, 0, menu, [pos.0, pos.1, pos.2]));
    game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_sessions_mut().insert((conn, joined, window), pos);
    let side = crate::game::tconstruct_ui::side_inventory(game, pos, true);
    if let Some(side) = side {
        let count = crate::game::tconstruct_crafting_station::side_items(game,side).map_or(0, |items|items.len());
        if matches!(count,5|9|27|54|64|256) {
            game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_side_mut().insert((conn,joined,window),(side,0));
            host.hub.guests[gi].send(&C::ContainerSetData(play::ContainerSetData {container:window,id:30110,value:count as i16}));
            host.hub.guests[gi].send(&C::ContainerSetData(play::ContainerSetData {container:window,id:30111,value:0}));
            let code=crate::game::tconstruct_source_side_layout::title_code(&game.res.facts().get(game.world.block(side.0,side.1,side.2)).name);
            host.hub.guests[gi].send(&C::ContainerSetData(play::ContainerSetData {container:window,id:30112,value:code}));
        }
    }
    game.lan_send_table_content(host, conn);
    true
}

pub(crate) fn click(game: &mut Game<'_>, host: &mut LanHost, conn: u32, packet: &play::ContainerClick) -> ControlFlow<bool> {
    let Some(gi) = host.hub.guests.iter().position(|guest| guest.conn == conn) else { return Continue(()); };
    let joined = host.hub.guests[gi].placed_at;
    let Some(pos) = session(game, conn, joined, packet.container) else { return Continue(()); };
    if packet.input==5 && game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_side().contains_key(&(conn,joined,packet.container)) {
        return Break(drag(game,host,gi,joined,pos,packet));
    }
    if packet.input==6 && game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_side().contains_key(&(conn,joined,packet.container)) {
        return Break(collect(game,host,gi,joined,pos,packet));
    }
    if (46..94).contains(&packet.slot) {
        return Break(side_click(game,host,gi,joined,pos,packet));
    }
    if packet.input==1 && (10..46).contains(&packet.slot) && quick_player(game,host,gi,joined,pos,packet) { return Break(true); }
    let Some((window, state, mut menu, stored_pos)) = host.play.get_mut(&conn).and_then(|play| play.crafting.table.take_if(|(window, ..)| *window == packet.container)) else { return Break(true); };
    if station(game, pos) && !host.hub.guests[gi].spectator() && host.hub.guests[gi].within_block_range([pos.0, pos.1, pos.2], 4.0) {
        if let Some(grid) = items(game, pos) {
            let side=game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_side().get(&(conn,joined,window)).copied().and_then(|(pos,_)| {
                let items=crate::game::tconstruct_crafting_station::side_items(game,pos)?;
                let kind=match items.len() {256=>crate::game::tconstruct_chest_rules::ChestKind::Part,64=>crate::game::tconstruct_chest_rules::ChestKind::Tinkers,n @ (5|9|27|54)=>crate::game::tconstruct_chest_rules::ChestKind::Plain(n),_=>return None};
                Some((pos,kind,items))
            });
            let tags=game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v|v.as_ref().ok()).and_then(|v|v.get("tconstruct:chest_parts")).cloned().unwrap_or_default();
            let mut updated_side=None;
            game.guest_menu_op(host, gi, &mut menu, |menu, env| {
                for (i, stack) in grid.into_iter().enumerate() { menu.set_slot(env, i + 1, stack); }
                if packet.slot==0 && packet.input==1 {
                    if let Some((pos,kind,mut items))=side {
                        let saved_grid:Vec<_>=(1..10).map(|i|menu.slot(i).cloned()).collect();
                        let saved_inventory:Vec<_>=(0..36).map(|i|menu.player_item(i).cloned()).collect();
                        let saved_carried=menu.carried.clone();
                        let lengths=(menu.drops.len(),menu.dropped.len(),menu.sounds.len(),menu.player_sounds.len(),menu.crafted.len());
                        if crate::game::tconstruct_crafting_output::quick(menu,env,&mut items,kind,&tags).is_ok() {updated_side=Some((pos,items));}
                        else {
                            for (i,stack) in saved_grid.into_iter().enumerate() {menu.set_slot(env,i+1,stack);}
                            for (i,stack) in saved_inventory.into_iter().enumerate() {menu.set_player_item(i,stack);}
                            menu.carried=saved_carried;menu.drops.truncate(lengths.0);menu.dropped.truncate(lengths.1);menu.sounds.truncate(lengths.2);menu.player_sounds.truncate(lengths.3);menu.crafted.truncate(lengths.4);
                        }
                        return;
                    }
                }
                let _ = menu.click(env, i32::from(packet.slot), i32::from(packet.button), input(packet.input));
            });
            if let Some((side,items))=updated_side {
                let _=crate::game::tconstruct_crafting_station::write_side(game,side,items);
            }
            if let Some(BlockEntity::Chest(items)) = game.world.block_entities.get_mut(&pos) {
                if items.len() == 9 { for (i, slot) in items.iter_mut().enumerate() { *slot = menu.slot(i + 1).cloned(); } }
            }
            game.world.modified.insert((pos.0 >> 4, pos.2 >> 4));
        }
    }
    host.play.entry(conn).or_default().crafting.table = Some((window, state, menu, stored_pos));
    game.lan_send_table_content(host, conn);
    Break(true)
}

fn collect(game: &mut Game<'_>,host: &mut LanHost,gi: usize,joined: u64,table: Pos,packet: &play::ContainerClick) -> bool {
    use crate::game::tconstruct_chest_rules as chest;
    if !matches!(packet.button,0|1) {return true;}
    let conn=host.hub.guests[gi].conn;let key=(conn,joined,packet.container);
    let valid=host.play.get(&conn).and_then(|p|p.crafting.table.as_ref()).is_some_and(|(window,state,_,_)|*window==packet.container && *state==packet.state);
    if !valid || !station(game,table) || host.hub.guests[gi].spectator() || !host.hub.guests[gi].within_block_range([table.0,table.1,table.2],4.0) {game.lan_send_table_content(host,conn);return true;}
    let Some(&(side,offset))=game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_side().get(&key) else {return true;};
    let Some(mut side_items)=crate::game::tconstruct_crafting_station::side_items(game,side) else {return true;};
    let Some(grid)=items(game,table) else {return true;};let mut combined=grid;combined.extend(host.hub.guests[gi].inventory.clone());let mut carried=host.hub.guests[gi].carried.clone();
    let Some(cursor)=carried.as_ref() else {return true;};let max=game.res.items().get(&cursor.item).map_or(64,|info|info.max_stack).max(1);
    let columns=crate::game::tconstruct_source_side_layout::columns(side_items.len());let visible=crate::game::tconstruct_source_side_layout::visible_rows(side_items.len())*columns;let start=offset*columns;
    let Ok(slot)=usize::try_from(packet.slot) else {return true;};
    let Some(clicked)=(match slot {1..=9=>combined.get(slot-1),10..=36=>combined.get(9+slot-10+9),37..=45=>combined.get(9+slot-37),46..=93 if slot-46<visible=>side_items.get(start+slot-46),_=>return true}) else {return true;};
    if clicked.is_some() {return true;}
    let mut order:Vec<_>=(0..9).map(|i|(false,i)).chain((start..(start+visible).min(side_items.len())).map(|i|(true,i))).chain((9..36).chain(0..9).map(|i|(false,9+i))).collect();
    if packet.button!=0 {order.reverse();}
    if chest::pickup_all_ordered(&mut side_items,&mut combined,&mut carried,&order,max) {
        if crate::game::tconstruct_crafting_station::write_side(game,side,side_items) {
            if let Some(BlockEntity::Chest(items))=game.world.block_entities.get_mut(&table) {*items=combined[..9].to_vec();}
            game.world.modified.insert((table.0>>4,table.2>>4));host.hub.guests[gi].inventory=combined[9..].to_vec();host.hub.guests[gi].carried=carried;
        }
    }
    game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_views_mut().remove(&key);update_viewers(game,host);true
}

fn quick_player(game: &mut Game<'_>,host: &mut LanHost,gi: usize,joined: u64,table: Pos,packet: &play::ContainerClick) -> bool {
    use crate::game::tconstruct_chest_rules::{self as chest,ChestKind};
    let conn=host.hub.guests[gi].conn;let key=(conn,joined,packet.container);
    let Some(&(side,_))=game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_side().get(&key) else {return false;};
    let valid=host.play.get(&conn).and_then(|p|p.crafting.table.as_ref()).is_some_and(|(window,state,_,_)|*window==packet.container && *state==packet.state);
    if !valid || !station(game,table) || host.hub.guests[gi].spectator() || !host.hub.guests[gi].within_block_range([table.0,table.1,table.2],4.0) {game.lan_send_table_content(host,conn);return true;}
    let index=packet.slot as usize-10;let index=if index<27 {index+9} else {index-27};
    let Some(stack)=host.hub.guests[gi].inventory[index].clone() else {return false;};
    let Some(existing)=crate::game::tconstruct_crafting_station::side_items(game,side) else {return false;};
    let kind=match existing.len() {256=>ChestKind::Part,64=>ChestKind::Tinkers,n @ (5|9|27|54)=>ChestKind::Plain(n),_=>return false};
    let max=game.res.items().get(&stack.item).map_or(64,|info|info.max_stack).max(1);
    let tags=game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v|v.as_ref().ok());
    let part=tags.and_then(|t|t.get("tconstruct:chest_parts")).is_some_and(|v|v.contains(stack.item.as_ref()));
    let mut items=existing;let old=stack.count;
    let remainder=chest::insert_across(kind,&mut items,stack,max,part);
    if remainder.as_ref().map_or(0,|s|s.count)==old {return false;}
    if !crate::game::tconstruct_crafting_station::write_side(game,side,items) {return true;}
    host.hub.guests[gi].inventory[index]=remainder;
    game.lan_send_table_content(host,conn);true
}

// Empty the temporary menu copy before vanilla close/leave returns its slots.
pub(crate) fn close(game: &mut Game<'_>, host: &mut LanHost, conn: u32, joined: u64, window: Option<i32>) {
    let Some((actual, _, menu, _)) = host.play.get_mut(&conn).and_then(|p| p.crafting.table.as_mut()) else { return; };
    if window.is_some_and(|window| window != *actual) { return; }
    let key = (conn, joined, *actual);
    let Some(pos) = game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_sessions_mut().remove(&key) else { return; };
    game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_side_mut().remove(&key);
    game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_views_mut().remove(&key);
    game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_drag_mut().remove(&key);
    // The block is updated after each transaction. It may have been removed
    // since the last click; in either case the temporary copies are not drops.
    let env = game.menu_env();
    for i in 1..10 { menu.set_slot(&env, i, None); }
    let _ = pos;
}

fn drag(game: &mut Game<'_>,host: &mut LanHost,gi: usize,joined: u64,table: Pos,packet: &play::ContainerClick) -> bool {
    use crate::game::tconstruct_chest_rules::{self as chest,ChestKind};
    let conn=host.hub.guests[gi].conn;let key=(conn,joined,packet.container);
    let valid=host.play.get(&conn).and_then(|p|p.crafting.table.as_ref()).is_some_and(|(window,state,_,_)|*window==packet.container && *state==packet.state);
    if !valid || !station(game,table) || host.hub.guests[gi].spectator() || !host.hub.guests[gi].within_block_range([table.0,table.1,table.2],4.0) {
        game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_drag_mut().remove(&key);game.lan_send_table_content(host,conn);return true;
    }
    let Some(&(side,offset))=game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_side().get(&key) else {return true;};
    let Some(mut side_items)=crate::game::tconstruct_crafting_station::side_items(game,side) else {return true;};
    let kind=match side_items.len() {256=>ChestKind::Part,64=>ChestKind::Tinkers,n @ (5|9|27|54)=>ChestKind::Plain(n),_=>return true};
    let mode=((packet.button>>2)&3) as u8;let phase=packet.button&3;
    if phase==0 {
        game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_drag_mut().remove(&key);
        if host.hub.guests[gi].carried.is_some() && (mode<2 || mode==2 && host.hub.guests[gi].creative()) {game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_drag_mut().insert(key,(mode,Vec::new()));}
        return true;
    }
    if phase==1 {
        let slot=usize::try_from(packet.slot).ok();let columns=crate::game::tconstruct_source_side_layout::columns(side_items.len());
        let visible=crate::game::tconstruct_source_side_layout::visible_rows(side_items.len())*columns;
        let selected=slot.and_then(|slot|match slot {
            1..=9=>Some((false,slot-1)),10..=36=>Some((false,9+slot-10+9)),37..=45=>Some((false,9+slot-37)),
            46..=93 if slot-46<visible=>Some((true,offset*columns+slot-46)),_=>None,
        });
        if let Some((stored,slots))=game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_drag_mut().get_mut(&key) {
            if *stored==mode {if let Some(selected)=selected {if slots.len()<93 && !slots.contains(&selected) {slots.push(selected);}}}
        }
        return true;
    }
    let pending=game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_drag_mut().remove(&key);
    if phase!=2 {return true;}
    let Some((_,selected))=pending.filter(|(stored,_)|*stored==mode) else {return true;};
    let Some(grid)=items(game,table) else {return true;};let mut combined=grid;combined.extend(host.hub.guests[gi].inventory.clone());let mut carried=host.hub.guests[gi].carried.clone();
    let max=carried.as_ref().and_then(|s|game.res.items().get(&s.item)).map_or(64,|info|info.max_stack).max(1);
    let tagged=carried.as_ref().is_some_and(|s|game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v|v.as_ref().ok()).and_then(|v|v.get("tconstruct:chest_parts")).is_some_and(|v|v.contains(s.item.as_ref())));
    if chest::quick_craft(kind,&mut side_items,&mut combined,&mut carried,&selected,mode,max,tagged,host.hub.guests[gi].creative()) {
        if crate::game::tconstruct_crafting_station::write_side(game,side,side_items) {
            if let Some(BlockEntity::Chest(items))=game.world.block_entities.get_mut(&table) {*items=combined[..9].to_vec();}
            game.world.modified.insert((table.0>>4,table.2>>4));host.hub.guests[gi].inventory=combined[9..].to_vec();host.hub.guests[gi].carried=carried;
        }
    }
    // The viewer refresh replaces the temporary crafting menu grid and sends
    // the entire corrected result, inventory and visible side page.
    game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_views_mut().remove(&key);
    update_viewers(game,host);
    true
}

fn side_click(game: &mut Game<'_>, host: &mut LanHost, gi: usize, joined: u64, table: Pos, packet: &play::ContainerClick) -> bool {
    use crate::game::tconstruct_chest_rules::{self as chest, ChestKind};
    let conn = host.hub.guests[gi].conn;
    let Some(&(side,offset)) = game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_side().get(&(conn,joined,packet.container)) else { return true; };
    let valid_state=host.play.get(&conn).and_then(|p|p.crafting.table.as_ref()).is_some_and(|(window,state,_,_)|*window==packet.container && *state==packet.state);
    if !valid_state { game.lan_send_table_content(host,conn); return true; }
    if !station(game,table) || host.hub.guests[gi].spectator() || !host.hub.guests[gi].within_block_range([table.0,table.1,table.2],4.0) { return true; }
    let Some(existing) = crate::game::tconstruct_crafting_station::side_items(game,side) else { return true; };
    let kind = match existing.len() {256=>ChestKind::Part,64=>ChestKind::Tinkers,n @ (5|9|27|54)=>ChestKind::Plain(n),_=>return true};
    let visible=crate::game::tconstruct_source_side_layout::visible_rows(existing.len())*crate::game::tconstruct_source_side_layout::columns(existing.len());
    if packet.slot as usize-46>=visible {return true;}
    let index = offset*crate::game::tconstruct_source_side_layout::columns(existing.len())+(packet.slot as usize-46);
    if index>=existing.len() || (!matches!(packet.input,2|3) && !matches!(packet.button,0|1)) { return true; }
    let mut inventory=host.hub.guests[gi].inventory.clone();
    let mut carried=host.hub.guests[gi].carried.clone();
    let mut offhand=host.hub.guests[gi].offhand.clone();
    let candidate=if packet.input==2 {if packet.button==40 {offhand.as_ref()} else {usize::try_from(packet.button).ok().and_then(|i|inventory.get(i)).and_then(Option::as_ref)}} else if packet.input==1 {existing[index].as_ref()} else {carried.as_ref()};
    let max=candidate.and_then(|s|game.res.items().get(&s.item)).map_or(64,|info|info.max_stack).max(1);
    let tags=game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|v|v.as_ref().ok());
    let part=candidate.is_some_and(|s|tags.and_then(|t|t.get("tconstruct:chest_parts")).is_some_and(|v|v.contains(s.item.as_ref())));
    let mut side_items=existing;
    let mut changed=false;
    let mut dropped=None;
    match packet.input {
        0 => changed=chest::pickup_slot(kind,&mut side_items,index,&mut carried,packet.button==1,max,part),
        1 => if let Some(stack)=side_items[index].take() {
            let old=stack.count;let remainder=crate::game::tconstruct_tx::source_quick_move_player(&mut inventory,stack,max,max>1);
            changed=remainder.as_ref().map_or(0,|s|s.count)!=old;side_items[index]=remainder;
        },
        2 => if let Some(left)=chest::swap_slot(kind,&mut side_items,index,&mut inventory,&mut offhand,i32::from(packet.button),max,part,|inventory,stack| {
            let limit=game.res.items().get(&stack.item).map_or(64,|info|info.max_stack).max(1);crate::game::tconstruct_tx::source_quick_move_player(inventory,stack,limit,limit>1)
        }) {changed=true;dropped=left;},
        4 if carried.is_none() => {dropped=chest::take_for_drop(&mut side_items,index,packet.button!=0);changed=dropped.is_some();},
        3 if host.hub.guests[gi].creative() && carried.is_none() => {
            if let Some(stack)=side_items[index].as_ref() {let mut copy=stack.clone();copy.count=game.res.items().get(&copy.item).map_or(64,|info|info.max_stack);carried=Some(copy);changed=true;}
        },
        _=>{},
    }
    if changed {
        if !crate::game::tconstruct_crafting_station::write_side(game,side,side_items) {return true;}
        host.hub.guests[gi].inventory=inventory;host.hub.guests[gi].carried=carried;host.hub.guests[gi].offhand=offhand;
        if packet.input==2 && packet.button==40 {
            let item=host.hub.stack(&host.hub.guests[gi].offhand.clone());let guest=&mut host.hub.guests[gi];guest.menu_state=(guest.menu_state+1)&0x7fff;
            guest.send(&C::ContainerSetSlot(play::ContainerSetSlot {container:0,state:guest.menu_state,slot:45,item}));
        }
    }
    if let Some(stack)=dropped {game.drop_guest_items(host,gi,vec![stack]);}
    game.lan_send_table_content(host,conn);
    true
}

pub(crate) fn content(game: &mut Game<'_>,host: &mut LanHost,conn: u32) -> ControlFlow<()> {
    let Some(gi)=host.hub.guests.iter().position(|g|g.conn==conn) else {return Continue(());};
    let joined=host.hub.guests[gi].placed_at;
    let Some((window,state,menu,_))=host.play.get_mut(&conn).and_then(|p|p.crafting.table.as_mut()) else {return Continue(());};
    let key=(conn,joined,*window);
    if !game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_sessions().contains_key(&key) {return Continue(());}
    let extra=game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_side().get(&key).and_then(|(side,offset)|crate::game::tconstruct_crafting_station::side_items(game,*side).map(|items|(*offset,items)));
    let mut slots: Vec<Option<ItemStack>>=(0..10).map(|i|menu.slot(i).cloned()).collect();
    let grid=slots[1..10].to_vec();
    slots.extend((9..36).chain(0..9).map(|i|host.hub.guests[gi].inventory[i].clone()));
    let extra=extra.map(|(offset,items)| {let start=offset*crate::game::tconstruct_source_side_layout::columns(items.len());(0..48).map(|i|items.get(start+i).cloned().flatten()).collect::<Vec<_>>()}).unwrap_or_default();
    slots.extend(extra.iter().cloned());
    game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_views_mut().insert(key,(grid,extra));
    *state=(*state+1)&0x7fff;let (window,state)=(*window,*state);
    let net=slots.iter().map(|slot|host.hub.stack(slot)).collect();
    let carried=host.hub.stack(&host.hub.guests[gi].carried.clone());
    host.hub.send_to(conn,&C::ContainerSetContent(play::ContainerSetContent {container:window,state,items:net,carried}));
    Break(())
}

pub(crate) fn update_viewers(game: &mut Game<'_>,host: &mut LanHost) {
    let sessions:Vec<_>=game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_sessions().iter().map(|(k,v)|(*k,*v)).collect();
    for ((conn,joined,window),pos) in sessions {
        let Some(gi)=host.hub.guests.iter().position(|g|g.conn==conn && g.placed_at==joined) else {continue;};
        let Some(grid)=items(game,pos) else {continue;};
        let extra=game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_side().get(&(conn,joined,window)).and_then(|(side,offset)|crate::game::tconstruct_crafting_station::side_items(game,*side).map(|items|(*offset,items)))
            .map(|(offset,items)| (0..48).map(|i|items.get(offset*crate::game::tconstruct_source_side_layout::columns(items.len())+i).cloned().flatten()).collect::<Vec<_>>()).unwrap_or_default();
        if game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_views().get(&(conn,joined,window)).is_some_and(|view|view.0==grid && view.1==extra) {continue;}
        let Some((active,state,mut menu,stored_pos))=host.play.get_mut(&conn).and_then(|p|p.crafting.table.take_if(|(active,..)|*active==window)) else {continue;};
        game.guest_menu_op(host,gi,&mut menu,|menu,env| {for (i,stack) in grid.into_iter().enumerate() {menu.set_slot(env,i+1,stack);}});
        host.play.entry(conn).or_default().crafting.table=Some((active,state,menu,stored_pos));
        game.lan_send_table_content(host,conn);
    }
}

pub(crate) fn scroll(game: &mut Game<'_>,host: &mut LanHost,conn: u32,window: i32,id: i32) -> ControlFlow<()> {
    if !(43000..=43043).contains(&id) {return Continue(());}
    let Some(gi)=host.hub.guests.iter().position(|g|g.conn==conn) else {return Continue(());};
    let joined=host.hub.guests[gi].placed_at;let key=(conn,joined,window);
    let Some(table)=session(game,conn,joined,window) else {return Continue(());};
    if !host.play.get(&conn).and_then(|p|p.crafting.table.as_ref()).is_some_and(|(active,_,_,_)|*active==window) {return Break(());}
    if !station(game,table) || host.hub.guests[gi].spectator() || !host.hub.guests[gi].within_block_range([table.0,table.1,table.2],4.0) {return Break(());}
    let Some(&(side,_))=game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_side().get(&key) else {return Break(());};
    let Some(items)=crate::game::tconstruct_crafting_station::side_items(game,side) else {return Break(());};
    let offset=((id-43000) as usize).min(crate::game::tconstruct_source_side_layout::max_offset(items.len()));
    game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_side_mut().insert(key,(side,offset));
    host.hub.guests[gi].send(&C::ContainerSetData(play::ContainerSetData {container:window,id:30111,value:offset as i16}));
    game.lan_send_table_content(host,conn);
    Break(())
}
