use super::*;
pub(super) fn run(game: &mut Game<'_>,host: &mut LanHost,conn:u32,window:i32) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break,Continue};
    use crate::game::tconstruct_chest_rules::{self as chest,ChestKind};
    let Some(gi)=host.hub.guests.iter().position(|g|g.conn==conn) else {return Continue(());};
    let Some(menu)=host.hub.guests[gi].menu.as_ref().filter(|m|m.window==window) else {return Continue(());};
    let Some(table)=menu.pos else {return Continue(());};
    let key=(conn,host.hub.guests[gi].placed_at,window,table);
    let Some((source,moved))=game.world.chunks.__horizon_mixins_tconstruct_chest_guest_last_move_mut().remove(&key) else {return Continue(());};
    if host.hub.guests[gi].spectator() || !menu.slots.still_valid(&game.world,&host.hub.guests[gi],menu.pos) {return Break(());}
    let (side,builder)=match menu.slots {horizon_client::lan::menu::Slots::Block(pos)=>(pos,false),horizon_client::lan::menu::Slots::Double(_,side)=>(side,true),_=>return Break(())};
    let Some(BlockEntity::Chest(existing))=game.world.block_entities.get(&side) else {return Break(());};
    let kind=match existing.len() {256=>ChestKind::Part,64=>ChestKind::Tinkers,_=>return Break(())};
    let mut contents=existing.clone();let mut inventory=host.hub.guests[gi].inventory.clone();
    let mut inputs=if builder {match game.world.block_entities.get(&table) {Some(BlockEntity::Chest(items))=>items.clone(),_=>return Break(())}} else {Vec::new()};
    let (scroll,_)=game.world.chunks.__horizon_mixins_tconstruct_chest_guest_pages().get(&key).copied().unwrap_or((0,contents.len()));
    let max=game.res.items().get(&moved.item).map_or(64,|info|info.max_stack).max(1);
    let tagged=game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|r|r.as_ref().ok()).and_then(|t|t.get("tconstruct:chest_parts")).is_some_and(|v|v.contains(moved.item.as_ref()));
    let mut changed=false;
    if source==0 {
        for i in (9..36).chain(0..9) {
            if !inventory[i].as_ref().is_some_and(|s|s.same_item_same_components(&moved)) {continue;}
            let stack=inventory[i].take().unwrap();let old=stack.count;let rest=chest::insert_across(kind,&mut contents,stack,max,tagged);
            changed|=rest.as_ref().map_or(0,|s|s.count)!=old;inventory[i]=rest;
        }
    } else {
        let indices:Vec<_>=if source==2 && builder {(0..2).collect()} else if source==1 {
            let first=scroll*if builder {6}else{8};(first..(first+if builder {48}else{32}).min(contents.len())).collect()
        } else {Vec::new()};
        let src=if source==2 {&mut inputs}else{&mut contents};
        for i in indices {
            if !src[i].as_ref().is_some_and(|s|s.same_item_same_components(&moved)) {continue;}
            let stack=src[i].take().unwrap();let old=stack.count;let rest=crate::game::tconstruct_tx::source_quick_move_player(&mut inventory,stack,max,max>1);
            changed|=rest.as_ref().map_or(0,|s|s.count)!=old;src[i]=rest;
        }
    }
    if changed {
        if let Some(BlockEntity::Chest(items))=game.world.block_entities.get_mut(&side) {*items=contents.clone();}
        if builder {if let Some(BlockEntity::Chest(items))=game.world.block_entities.get_mut(&table) {*items=inputs.clone();}}
        game.world.modified.insert((side.0>>4,side.2>>4));game.world.modified.insert((table.0>>4,table.2>>4));host.hub.guests[gi].inventory=inventory;
    }
    let view=if builder {let mut v=inputs;v.extend(chest::project_side(&contents,scroll));v} else {
        let visual=if kind==ChestKind::Part {chest::part_visual_size(contents.iter().map(Option::is_some),contents.len())}else{contents.len()};
        let offset=scroll.min(chest::max_scroll(visual));game.world.chunks.__horizon_mixins_tconstruct_chest_guest_pages_mut().insert(key,(offset,visual));
        host.hub.guests[gi].send(&horizon_protocol::packets::play::Clientbound::ContainerSetData(horizon_protocol::packets::play::ContainerSetData{container:window,id:30101,value:visual as i16}));
        host.hub.guests[gi].send(&horizon_protocol::packets::play::Clientbound::ContainerSetData(horizon_protocol::packets::play::ContainerSetData{container:window,id:30102,value:offset as i16}));chest::project(&contents,offset)
    };
    let mut items=view;
    items.extend((9..36).chain(0..9).map(|i|host.hub.guests[gi].inventory[i].clone()));
    let net=items.iter().map(|item|host.hub.stack(item)).collect();
    let carried=host.hub.stack(&host.hub.guests[gi].carried.clone());
    let guest=&mut host.hub.guests[gi];
    let Some(menu)=guest.menu.as_mut() else {return Break(());};
    menu.state=(menu.state+1)&0x7fff;
    let content=horizon_protocol::packets::play::ContainerSetContent {container:window,state:menu.state,items:net,carried};
    menu.sent=items;
    guest.send(&horizon_protocol::packets::play::Clientbound::ContainerSetContent(content));
    Break(())
}
