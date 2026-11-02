fn tconstruct_side_chest_click(
    game: &mut crate::game::Game<'_>, slot: Option<usize>, button: i32,
    input: horizon_client::menu::Input,
) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break,Continue};
    use horizon_client::menu::Input;
    let Some(kind)=crate::game::tconstruct_ui::side_chest_kind(game) else { return Continue(()); };
    if game.remote.is_some() { return Continue(()); }
    let Some(table)=game.container else { return Break(true); };
    let Some(side)=game.world.chunks.__horizon_mixins_tconstruct_builder_side().get(&table).copied() else { return Break(true); };
    let Some(slot)=slot else {
        if input==Input::Pickup && matches!(button,0|1) && game.eye_position().is_some() {
            let mut cursor=vec![game.carried.take()];let dropped=crate::game::tconstruct_chest_rules::take_for_drop(&mut cursor,0,button==0);game.carried=cursor.pop().flatten();
            if let (Some(stack),Some(eye))=(dropped,game.eye_position()) {game.throw_from_eyes(stack,eye,false);}
        }
        return Break(true);
    };
    if slot==322 || (324..=326).contains(&slot) { return Continue(()); }
    if !matches!(input,Input::Swap|Input::Clone) && !matches!(button,0|1) { return Break(true); }
    let offset=game.world.chunks.__horizon_mixins_tconstruct_builder_side_scroll().get(&table).map_or(0,|s|s.0);
    let actual=(327..381).contains(&slot).then_some(offset*6+slot.saturating_sub(327));
    let candidate=if input==Input::Swap {
        if button==40 {game.offhand.as_ref()} else {usize::try_from(button).ok().and_then(|i|game.inventory.get(i)).and_then(Option::as_ref)}
    } else if input==Input::QuickMove {
        if slot<36 {game.inventory.get(slot).and_then(Option::as_ref)}
        else if (300..302).contains(&slot) { game.world.block_entities.get(&table).and_then(|e|e.slots().get(slot-300)).and_then(Option::as_ref) }
        else {actual.and_then(|i|game.world.block_entities.get(&side).and_then(|e|e.slots().get(i)).and_then(Option::as_ref))}
    } else {game.carried.as_ref()};
    let max=candidate.and_then(|s|game.res.items().get(&s.item)).map_or(64,|i|i.max_stack).max(1);
    let tags=game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|r|r.as_ref().ok());
    let part=candidate.is_some_and(|s|tags.and_then(|t|t.get("tconstruct:chest_parts")).is_some_and(|v|v.contains(s.item.as_ref())));
    let pattern=candidate.is_some_and(|s|tags.and_then(|t|t.get("tconstruct:patterns")).is_some_and(|v|v.contains(s.item.as_ref())));
    let candidate_empty=candidate.is_none();
    let mut changed=false;
    let mut dropped=None;
    match input {
        Input::Pickup if slot<36 => changed=crate::game::tconstruct_chest_rules::pickup_player(&mut game.inventory,slot,&mut game.carried,button==1,max),
        Input::Pickup if (300..302).contains(&slot) => {
            if slot!=300 || game.carried.is_none() || pattern {
                if let Some(BlockEntity::Chest(items))=game.world.block_entities.get_mut(&table) {
                    changed=crate::game::tconstruct_chest_rules::pickup_player(items,slot-300,&mut game.carried,button==1,max);
                }
            }
        }
        Input::Pickup if actual.is_some() => if let Some(BlockEntity::Chest(items))=game.world.block_entities.get_mut(&side) {
            changed=crate::game::tconstruct_chest_rules::pickup_slot(kind,items,actual.unwrap(),&mut game.carried,button==1,max,part);
        },
        Input::QuickMove if slot<36 => if let Some(stack)=game.inventory.get(slot).cloned().flatten() {
            let mut table_items=match game.world.block_entities.get(&table) {Some(BlockEntity::Chest(items))=>items.clone(),_=>return Break(true)};
            let mut side_items=match game.world.block_entities.get(&side) {Some(BlockEntity::Chest(items))=>items.clone(),_=>return Break(true)};
            let old=stack.count;
            let remaining=crate::game::tconstruct_tx::source_quick_move_to_builder(&mut table_items,&mut [],stack,max,max>1,pattern);
            let remaining=remaining.and_then(|stack|crate::game::tconstruct_chest_rules::insert_across(kind,&mut side_items,stack,max,part));
            changed=remaining.as_ref().map_or(0,|s|s.count)!=old;
            if changed {
                if let Some(BlockEntity::Chest(items))=game.world.block_entities.get_mut(&table) {*items=table_items;}
                if let Some(BlockEntity::Chest(items))=game.world.block_entities.get_mut(&side) {*items=side_items;}
                game.inventory[slot]=remaining;
            }
        },
        Input::QuickMove if (300..302).contains(&slot) || actual.is_some() => {
            let (pos,index)=if (300..302).contains(&slot) {(table,slot-300)} else {(side,actual.unwrap())};
            if let Some(BlockEntity::Chest(items))=game.world.block_entities.get_mut(&pos) { if let Some(cell)=items.get_mut(index) { if let Some(stack)=cell.take() {
                let old=stack.count;let remaining=crate::game::tconstruct_tx::source_quick_move_player(&mut game.inventory,stack,max,max>1);
                changed=remaining.as_ref().map_or(0,|s|s.count)!=old;*cell=remaining;
            }}}
        }
        Input::Swap if actual.is_some() => {
            let registry=game.res.items();
            if let Some(BlockEntity::Chest(items))=game.world.block_entities.get_mut(&side) {
                if let Some(left)=crate::game::tconstruct_chest_rules::swap_slot(kind,items,actual.unwrap(),&mut game.inventory,&mut game.offhand,button,max,part,|inventory,stack| {
                    let limit=registry.get(&stack.item).map_or(64,|info|info.max_stack).max(1);crate::game::tconstruct_tx::source_quick_move_player(inventory,stack,limit,limit>1)
                }) {changed=true;dropped=left;}
            }
        }
        Input::Swap if (300..302).contains(&slot) && (slot!=300 || candidate_empty || pattern) => {
            let registry=game.res.items();
            if let Some(BlockEntity::Chest(items))=game.world.block_entities.get_mut(&table) {
                let kind=crate::game::tconstruct_chest_rules::ChestKind::Plain(items.len());
                if let Some(left)=crate::game::tconstruct_chest_rules::swap_slot(kind,items,slot-300,&mut game.inventory,&mut game.offhand,button,max,true,|inventory,stack| {
                    let limit=registry.get(&stack.item).map_or(64,|info|info.max_stack).max(1);crate::game::tconstruct_tx::source_quick_move_player(inventory,stack,limit,limit>1)
                }) {changed=true;dropped=left;}
            }
        }
        Input::Swap if slot<36 => {
            if button==40 {std::mem::swap(&mut game.inventory[slot],&mut game.offhand);changed=true;}
            else if (0..9).contains(&button) {game.inventory.swap(slot,button as usize);changed=true;}
        }
        Input::Throw if game.carried.is_none() => {
            if slot<36 {dropped=crate::game::tconstruct_chest_rules::take_for_drop(&mut game.inventory,slot,button!=0);}
            else if (300..302).contains(&slot) {if let Some(BlockEntity::Chest(items))=game.world.block_entities.get_mut(&table) {dropped=crate::game::tconstruct_chest_rules::take_for_drop(items,slot-300,button!=0);}}
            else if let Some(i)=actual {if let Some(BlockEntity::Chest(items))=game.world.block_entities.get_mut(&side) {dropped=crate::game::tconstruct_chest_rules::take_for_drop(items,i,button!=0);}}
            changed=dropped.is_some();
        }
        Input::Clone if game.creative && game.carried.is_none() => {
            let stack=if slot<36 {game.inventory.get(slot).and_then(Option::as_ref)} else if (300..302).contains(&slot) {game.world.block_entities.get(&table).and_then(|e|e.slots().get(slot-300)).and_then(Option::as_ref)} else {actual.and_then(|i|game.world.block_entities.get(&side).and_then(|e|e.slots().get(i)).and_then(Option::as_ref))};
            if let Some(stack)=stack {let mut copy=stack.clone();copy.count=game.res.items().get(&copy.item).map_or(64,|info|info.max_stack);game.carried=Some(copy);changed=true;}
        }
        Input::PickupAll if game.carried.is_some() => {
            let clicked=if slot<36 {game.inventory.get(slot).and_then(Option::as_ref)} else if (300..302).contains(&slot) {game.world.block_entities.get(&table).and_then(|e|e.slots().get(slot-300)).and_then(Option::as_ref)} else {actual.and_then(|i|game.world.block_entities.get(&side).and_then(|e|e.slots().get(i)).and_then(Option::as_ref))};
            if clicked.is_none() {
                let Some(BlockEntity::Chest(input))=game.world.block_entities.get(&table) else {return Break(true);};
                let mut combined=input.clone();combined.extend(game.inventory.clone());
                let start=offset*6;let mut order:Vec<_>=(0..2).map(|i|(false,i)).chain((start..(start+48).min(kind.slots())).map(|i|(true,i))).chain((9..36).chain(0..9).map(|i|(false,27+i))).collect();
                if button!=0 {order.reverse();}
                if let Some(BlockEntity::Chest(items))=game.world.block_entities.get_mut(&side) {changed=crate::game::tconstruct_chest_rules::pickup_all_ordered(items,&mut combined,&mut game.carried,&order,max);}
                if changed {if let Some(BlockEntity::Chest(input))=game.world.block_entities.get_mut(&table) {*input=combined[..27].to_vec();}game.inventory=combined[27..].to_vec();}
            }
        }
        _=>{},
    }
    if changed { game.world.modified.insert((table.0>>4,table.2>>4));game.world.modified.insert((side.0>>4,side.2>>4)); }
    if let (Some(stack),Some(eye))=(dropped,game.eye_position()) {game.throw_from_eyes(stack,eye,false);}
    Break(true)
}
