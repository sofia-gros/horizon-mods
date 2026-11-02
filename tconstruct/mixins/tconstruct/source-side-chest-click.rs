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
    let Some(slot)=slot else { return Break(true); };
    if slot==322 || (324..=326).contains(&slot) { return Continue(()); }
    if !matches!(button,0|1) { return Break(true); }
    let offset=game.world.chunks.__horizon_mixins_tconstruct_builder_side_scroll().get(&table).map_or(0,|s|s.0);
    let actual=(327..381).contains(&slot).then_some(offset*6+slot.saturating_sub(327));
    let candidate=if input==Input::QuickMove {
        if slot<36 {game.inventory.get(slot).and_then(Option::as_ref)}
        else if (300..302).contains(&slot) { game.world.block_entities.get(&table).and_then(|e|e.slots().get(slot-300)).and_then(Option::as_ref) }
        else {actual.and_then(|i|game.world.block_entities.get(&side).and_then(|e|e.slots().get(i)).and_then(Option::as_ref))}
    } else {game.carried.as_ref()};
    let max=candidate.and_then(|s|game.res.items().get(&s.item)).map_or(64,|i|i.max_stack).max(1);
    let tags=game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|r|r.as_ref().ok());
    let part=candidate.is_some_and(|s|tags.and_then(|t|t.get("tconstruct:chest_parts")).is_some_and(|v|v.contains(s.item.as_ref())));
    let pattern=candidate.is_some_and(|s|tags.and_then(|t|t.get("tconstruct:patterns")).is_some_and(|v|v.contains(s.item.as_ref())));
    let mut changed=false;
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
        _=>{},
    }
    if changed { game.world.modified.insert((table.0>>4,table.2>>4));game.world.modified.insert((side.0>>4,side.2>>4)); }
    Break(true)
}
