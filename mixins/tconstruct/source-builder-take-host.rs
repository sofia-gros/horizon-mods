use super::*;

/// Revalidate the selected source recipe against the current world and apply
/// one cursor pickup. Any item that does not fit is dropped by the game.
pub(super) fn take(game: &mut Game<'_>, pos: (i32, i32, i32)) -> bool {
    let Some(eye) = game.eye_position() else { return false; };
    let Some(rest) = game.res.rest.get() else { return false; };
    let items = &rest.items;
    let Ok(Some((plan, output, leftover, alternatives))) = super::tconstruct_source_builder_world::prepare_take(&game.world, pos, items) else { return false; };
    let Some(max) = items.get(&output.item).map(|info| info.max_stack) else { return false; };
    let Some(slots) = game.world.block_entities.get(&pos).map(|entity| entity.slots()) else { return false; };
    let Ok(material_container) = super::tconstruct_source_builder_world::crafting_container(slots.get(1).and_then(Option::as_ref), items) else { return false; };
    let Ok(pattern_container) = super::tconstruct_source_builder_world::crafting_container(slots.first().and_then(Option::as_ref), items) else { return false; };
    let Some(BlockEntity::Chest(slots)) = game.world.block_entities.get_mut(&pos) else { return false; };
    let Some(give) = super::tconstruct_tx::source_take_part_plan(slots, &mut game.carried, &plan, output, leftover, alternatives, material_container, pattern_container, max, |bound| super::tconstruct_source_builder_world::source_random_draw(items, bound)) else { return false; };
    let mut dropped = Vec::new();
    for stack in give {
        let Some(info) = items.get(&stack.item) else { dropped.push(stack); continue; };
        let can_stack = info.max_stack > 1 && !(info.max_damage > 0 && stack.damage > 0);
        if let Some(left) = super::tconstruct_tx::source_give_inventory(&mut game.inventory, game.selected, stack, info.max_stack, can_stack) { dropped.push(left); }
    }
    game.world.modified.insert((pos.0 >> 4, pos.2 >> 4));
    for stack in dropped { game.throw_from_eyes(stack, eye, false); }
    true
}

/// Quick-move the source result only where no adjacent inventory can take
/// precedence. Recheck the recipe for every iteration, as the source menu does.
pub(super) fn quick_take(game: &mut Game<'_>, pos: (i32, i32, i32)) -> bool {
    if !super::tconstruct_source_builder_world::player_only_quick_move(&game.world, pos) { return false; }
    let Some(eye) = game.eye_position() else { return false; };
    let Some(rest) = game.res.rest.get() else { return false; };
    let items = &rest.items;
    let mut first_item = None;
    let mut dropped = Vec::new();
    let mut changed = false;
    loop {
        let Ok(Some((plan, output, leftover, alternatives))) = super::tconstruct_source_builder_world::prepare_take(&game.world, pos, items) else { break; };
        if first_item.as_ref().is_some_and(|item| item != &output.item) { break; }
        if first_item.is_none() { first_item = Some(output.item.clone()); }
        let Some(info) = items.get(&output.item) else { break; };
        let max = info.max_stack;
        let can_stack = max > 1 && !(info.max_damage > 0 && output.damage > 0);
        let Some(slots) = game.world.block_entities.get(&pos).map(|entity| entity.slots()) else { break; };
        let Ok(material) = super::tconstruct_source_builder_world::crafting_container(slots.get(1).and_then(Option::as_ref), items) else { break; };
        let Ok(pattern) = super::tconstruct_source_builder_world::crafting_container(slots.first().and_then(Option::as_ref), items) else { break; };
        let Some(BlockEntity::Chest(slots)) = game.world.block_entities.get_mut(&pos) else { break; };
        let Some(give) = super::tconstruct_tx::source_quick_take_part_plan(slots, &mut game.inventory, &plan, output, leftover, alternatives, material, pattern, max, can_stack, |bound| super::tconstruct_source_builder_world::source_random_draw(items, bound)) else { break; };
        changed = true;
        for stack in give {
            let Some(info) = items.get(&stack.item) else { dropped.push(stack); continue; };
            let can_stack = info.max_stack > 1 && !(info.max_damage > 0 && stack.damage > 0);
            if let Some(left) = super::tconstruct_tx::source_give_inventory(&mut game.inventory, game.selected, stack, info.max_stack, can_stack) { dropped.push(left); }
        }
    }
    if changed { game.world.modified.insert((pos.0 >> 4, pos.2 >> 4)); }
    for stack in dropped { game.throw_from_eyes(stack, eye, false); }
    changed
}
