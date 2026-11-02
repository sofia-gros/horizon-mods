fn tconstruct_crafting_side_click(game: &mut crate::game::Game<'_>, slot: Option<usize>, button: i32, input: horizon_client::menu::Input) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    use horizon_client::menu::Input;
    let Some((side,count,offset,kind)) = crate::game::tconstruct_crafting_station::side(game) else { return Continue(()); };
    let Some(slot) = slot else { return Continue(()); };
    let side_slot = (600..648).contains(&slot).then_some(offset * crate::game::tconstruct_source_side_layout::columns(count) + slot.saturating_sub(600)).filter(|i| *i < count);
    if side_slot.is_none() && !(slot < 36 && input == Input::QuickMove) { return Continue(()); }
    if !matches!(button, 0 | 1) { return Break(true); }
    let candidate = if input == Input::QuickMove {
        if slot < 36 { game.inventory.get(slot).and_then(Option::as_ref) }
        else { side_slot.and_then(|i| game.world.block_entities.get(&side).and_then(|entity| entity.slots().get(i)).and_then(Option::as_ref)) }
    } else { game.carried.as_ref() };
    let max = candidate.and_then(|s| game.res.items().get(&s.item)).map_or(64, |info| info.max_stack).max(1);
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|result| result.as_ref().ok());
    let part = candidate.is_some_and(|s| tags.and_then(|t| t.get("tconstruct:chest_parts")).is_some_and(|v| v.contains(s.item.as_ref())));
    let Some(BlockEntity::Chest(items)) = game.world.block_entities.get_mut(&side) else { return Break(true); };
    let mut changed = false;
    match (input, side_slot) {
        (Input::Pickup, Some(i)) => changed = crate::game::tconstruct_chest_rules::pickup_slot(kind, items, i, &mut game.carried, button == 1, max, part),
        (Input::QuickMove, Some(i)) => if let Some(stack) = items[i].take() {
            let old = stack.count;
            let remainder = crate::game::tconstruct_tx::source_quick_move_player(&mut game.inventory, stack, max, max > 1);
            changed = remainder.as_ref().map_or(0, |s| s.count) != old;
            items[i] = remainder;
        },
        (Input::QuickMove, None) => if let Some(stack) = game.inventory[slot].take() {
            let old = stack.count;
            let remainder = crate::game::tconstruct_chest_rules::insert_across(kind, items, stack, max, part);
            changed = remainder.as_ref().map_or(0, |s| s.count) != old;
            game.inventory[slot] = remainder;
            if !changed { return Continue(()); }
        },
        _ => {},
    }
    if changed { game.world.modified.insert((side.0 >> 4, side.2 >> 4)); }
    Break(true)
}
