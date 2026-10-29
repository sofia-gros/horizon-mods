use super::*;

/// Revalidate the selected source recipe against the current world and apply
/// one cursor pickup. Any item that does not fit is dropped by the game.
pub(super) fn take(game: &mut Game<'_>, pos: (i32, i32, i32)) -> bool {
    let Some(eye) = game.eye_position() else { return false; };
    let Some(rest) = game.res.rest.get() else { return false; };
    let items = &rest.items;
    let Ok(Some((plan, output, leftover))) = super::tconstruct_source_builder_world::prepare_take(&game.world, pos, items) else { return false; };
    let Some(max) = items.get(&output.item).map(|info| info.max_stack) else { return false; };
    let Some(slots) = game.world.block_entities.get(&pos).map(|entity| entity.slots()) else { return false; };
    let Ok(material_container) = super::tconstruct_source_builder_world::crafting_container(slots.get(1).and_then(Option::as_ref), items) else { return false; };
    let Ok(pattern_container) = super::tconstruct_source_builder_world::crafting_container(slots.first().and_then(Option::as_ref), items) else { return false; };
    let Some(BlockEntity::Chest(slots)) = game.world.block_entities.get_mut(&pos) else { return false; };
    let Some(give) = super::tconstruct_tx::source_take_part_plan(slots, &mut game.carried, &plan, output, leftover, material_container, pattern_container, max) else { return false; };
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
