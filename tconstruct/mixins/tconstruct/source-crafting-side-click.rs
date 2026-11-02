fn tconstruct_crafting_side_click(game: &mut crate::game::Game<'_>, slot: Option<usize>, button: i32, input: horizon_client::menu::Input) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    use horizon_client::menu::Input;
    if game.remote.is_some() { return Continue(()); }
    let Some((side,count,offset,kind)) = crate::game::tconstruct_crafting_station::side(game) else { return Continue(()); };
    let Some(slot) = slot else { return Continue(()); };
    let side_slot = (600..648).contains(&slot).then_some(offset * crate::game::tconstruct_source_side_layout::columns(count) + slot.saturating_sub(600)).filter(|i| *i < count);
    if side_slot.is_none() && !(slot < 36 && input == Input::QuickMove) && !(input==Input::PickupAll && (slot<36 || (100..109).contains(&slot))) { return Continue(()); }
    if !matches!(input,Input::Swap|Input::Clone) && !matches!(button, 0 | 1) { return Break(true); }
    let Some(mut items)=crate::game::tconstruct_crafting_station::side_items(game,side) else {return Break(true);};
    let candidate = if input == Input::Swap {
        if button==40 {game.offhand.as_ref()} else {usize::try_from(button).ok().and_then(|i|game.inventory.get(i)).and_then(Option::as_ref)}
    } else if input == Input::QuickMove {
        if slot < 36 { game.inventory.get(slot).and_then(Option::as_ref) }
        else { side_slot.and_then(|i|items.get(i)).and_then(Option::as_ref) }
    } else { game.carried.as_ref() };
    let max = candidate.and_then(|s| game.res.items().get(&s.item)).map_or(64, |info| info.max_stack).max(1);
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|result| result.as_ref().ok());
    let part = candidate.is_some_and(|s| tags.and_then(|t| t.get("tconstruct:chest_parts")).is_some_and(|v| v.contains(s.item.as_ref())));
    let mut changed = false;
    let mut dropped=None;
    match (input, side_slot) {
        (Input::Pickup, Some(i)) => changed = crate::game::tconstruct_chest_rules::pickup_slot(kind, &mut items, i, &mut game.carried, button == 1, max, part),
        (Input::QuickMove, Some(i)) => if let Some(stack) = items[i].take() {
            let old = stack.count;
            let remainder = crate::game::tconstruct_tx::source_quick_move_player(&mut game.inventory, stack, max, max > 1);
            changed = remainder.as_ref().map_or(0, |s| s.count) != old;
            items[i] = remainder;
        },
        (Input::QuickMove, None) => if let Some(stack) = game.inventory[slot].take() {
            let old = stack.count;
            let remainder = crate::game::tconstruct_chest_rules::insert_across(kind, &mut items, stack, max, part);
            changed = remainder.as_ref().map_or(0, |s| s.count) != old;
            game.inventory[slot] = remainder;
            if !changed { return Continue(()); }
        },
        (Input::Swap,Some(i)) => {
            let registry=game.res.items();
            if let Some(left)=crate::game::tconstruct_chest_rules::swap_slot(kind,&mut items,i,&mut game.inventory,&mut game.offhand,button,max,part,|inventory,stack| {
                let limit=registry.get(&stack.item).map_or(64,|info|info.max_stack).max(1);crate::game::tconstruct_tx::source_quick_move_player(inventory,stack,limit,limit>1)
            }) {changed=true;dropped=left;}
        },
        (Input::Throw,Some(i)) if game.carried.is_none() => {dropped=crate::game::tconstruct_chest_rules::take_for_drop(&mut items,i,button!=0);changed=dropped.is_some();},
        (Input::Clone,Some(i)) if game.creative && game.carried.is_none() => {
            if let Some(stack)=items[i].as_ref() {let mut copy=stack.clone();copy.count=game.res.items().get(&copy.item).map_or(64,|info|info.max_stack);game.carried=Some(copy);changed=true;}
        },
        (Input::PickupAll,_) if game.carried.is_some() => {
            let clicked=if slot<36 {game.inventory.get(slot).and_then(Option::as_ref)} else if (100..109).contains(&slot) {game.craft_grid.get(slot-100).and_then(Option::as_ref)} else {side_slot.and_then(|i|items.get(i)).and_then(Option::as_ref)};
            if clicked.is_none() {
                let mut combined=game.craft_grid.clone();combined.extend(game.inventory.clone());
                let start=offset*crate::game::tconstruct_source_side_layout::columns(count);
                let visible=crate::game::tconstruct_source_side_layout::visible_rows(count)*crate::game::tconstruct_source_side_layout::columns(count);
                let mut order:Vec<_>=(0..9).map(|i|(false,i)).chain((start..(start+visible).min(count)).map(|i|(true,i))).chain((9..36).chain(0..9).map(|i|(false,9+i))).collect();
                if button!=0 {order.reverse();}
                changed=crate::game::tconstruct_chest_rules::pickup_all_ordered(&mut items,&mut combined,&mut game.carried,&order,max);
                if changed {game.craft_grid=combined[..9].to_vec();game.inventory=combined[9..].to_vec();crate::game::tconstruct_crafting_station::sync(game);}
            }
        },
        _ => {},
    }
    if changed { let _=crate::game::tconstruct_crafting_station::write_side(game,side,items); }
    if let (Some(stack),Some(eye))=(dropped,game.eye_position()) {game.throw_from_eyes(stack,eye,false);}
    Break(true)
}
