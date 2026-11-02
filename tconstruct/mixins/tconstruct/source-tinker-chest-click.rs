fn tconstruct_chest_click(
    game: &mut crate::game::Game<'_>, slot: Option<usize>, button: i32,
    input: horizon_client::menu::Input,
) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    use horizon_client::menu::Input;
    let Some(kind) = crate::game::tconstruct_chest_ui::kind(game) else { return Continue(()); };
    if game.remote.is_some() { return Continue(()); }
    let Some(key) = game.container else { return Break(true); };
    let Some(slot) = slot else {
        if input==Input::Pickup && matches!(button,0|1) && game.eye_position().is_some() {
            let mut cursor=vec![game.carried.take()];
            let dropped=crate::game::tconstruct_chest_rules::take_for_drop(&mut cursor,0,button==0);
            game.carried=cursor.pop().flatten();
            if let (Some(stack),Some(eye))=(dropped,game.eye_position()) {game.throw_from_eyes(stack,eye,false);}
        }
        return Break(true);
    };
    if !matches!(input,Input::Swap|Input::Clone) && button != 0 && button != 1 { return Break(true); }
    let candidate = if input == Input::Swap {
        if button==40 {game.offhand.as_ref()} else {usize::try_from(button).ok().and_then(|i|game.inventory.get(i)).and_then(Option::as_ref)}
    } else if input == Input::QuickMove {
        if slot < 36 { game.inventory.get(slot).and_then(Option::as_ref) }
        else { game.world.block_entities.get(&key).and_then(|e| e.slots().get(slot.saturating_sub(300))).and_then(Option::as_ref) }
    } else { game.carried.as_ref() };
    let max = candidate.and_then(|s| game.res.items().get(&s.item)).map_or(64, |info| info.max_stack);
    let tagged = candidate.is_some_and(|s| game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()
        .and_then(|result| result.as_ref().ok()).and_then(|tags| tags.get("tconstruct:chest_parts"))
        .is_some_and(|items| items.contains(s.item.as_ref())));
    let mut changed = false;
    let mut dropped=None;
    match input {
        Input::Pickup if slot < 36 => {
            changed = crate::game::tconstruct_chest_rules::pickup_player(&mut game.inventory, slot, &mut game.carried, button == 1, max);
        }
        Input::Pickup if (300..556).contains(&slot) => {
            if let Some(BlockEntity::Chest(items)) = game.world.block_entities.get_mut(&key) {
                changed = crate::game::tconstruct_chest_rules::pickup_slot(kind, items, slot - 300, &mut game.carried, button == 1, max, tagged);
            }
        }
        Input::QuickMove if slot < 36 => {
            if let Some(stack) = game.inventory.get(slot).cloned().flatten() {
                if let Some(BlockEntity::Chest(items)) = game.world.block_entities.get_mut(&key) {
                    let old = stack.count;
                    let remainder = crate::game::tconstruct_chest_rules::insert_across(kind, items, stack, max, tagged);
                    changed = remainder.as_ref().map_or(0, |s| s.count) != old;
                    game.inventory[slot] = remainder;
                }
            }
        }
        Input::QuickMove if (300..556).contains(&slot) => {
            if let Some(BlockEntity::Chest(items)) = game.world.block_entities.get_mut(&key) {
                if let Some(cell) = items.get_mut(slot - 300) {
                    if let Some(stack) = cell.take() {
                        let old = stack.count;
                        let remainder = crate::game::tconstruct_tx::source_quick_move_player(&mut game.inventory, stack, max, max > 1);
                        changed = remainder.as_ref().map_or(0, |s| s.count) != old;
                        *cell = remainder;
                    }
                }
            }
        }
        Input::Swap if (300..556).contains(&slot) => {
            let registry=game.res.items();
            if let Some(BlockEntity::Chest(items))=game.world.block_entities.get_mut(&key) {
                if let Some(left)=crate::game::tconstruct_chest_rules::swap_slot(kind,items,slot-300,&mut game.inventory,&mut game.offhand,button,max,tagged,|inventory,stack| {
                    let limit=registry.get(&stack.item).map_or(64,|info|info.max_stack).max(1);
                    crate::game::tconstruct_tx::source_quick_move_player(inventory,stack,limit,limit>1)
                }) {changed=true;dropped=left;}
            }
        }
        Input::Swap if slot<36 => {
            if button==40 {std::mem::swap(&mut game.inventory[slot],&mut game.offhand);changed=true;}
            else if (0..9).contains(&button) {game.inventory.swap(slot,button as usize);changed=true;}
        }
        Input::Throw if game.carried.is_none() => {
            if slot<36 {dropped=crate::game::tconstruct_chest_rules::take_for_drop(&mut game.inventory,slot,button!=0);}
            else if (300..556).contains(&slot) {
                if let Some(BlockEntity::Chest(items))=game.world.block_entities.get_mut(&key) {dropped=crate::game::tconstruct_chest_rules::take_for_drop(items,slot-300,button!=0);}
            }
            changed=dropped.is_some();
        }
        Input::Clone if game.creative && game.carried.is_none() => {
            let stack=if slot<36 {game.inventory.get(slot).and_then(Option::as_ref)} else {game.world.block_entities.get(&key).and_then(|e|e.slots().get(slot.saturating_sub(300))).and_then(Option::as_ref)};
            if let Some(stack)=stack {let mut copy=stack.clone();copy.count=game.res.items().get(&copy.item).map_or(64,|info|info.max_stack);game.carried=Some(copy);changed=true;}
        }
        _ => {}
    }
    if changed { game.world.modified.insert((key.0 >> 4, key.2 >> 4)); }
    if let (Some(stack),Some(eye))=(dropped,game.eye_position()) {game.throw_from_eyes(stack,eye,false);}
    // Unsupported gestures are consumed until implemented; never let the
    // vanilla 54-slot menu misinterpret this chest's large slot IDs.
    Break(true)
}
