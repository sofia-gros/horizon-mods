use super::*;

/// Commit an operation once and return recipe leftovers to the player.
pub(super) fn take(
    game: &mut Game<'_>,
    pos: (i32, i32, i32),
    plan: super::tconstruct_source_station_world::StationOperation,
) -> bool {
    let Some(eye) = game.eye_position() else {
        return false;
    };
    let Some(rest) = game.res.rest.get() else {
        return false;
    };
    let items = &rest.items;
    let Some(BlockEntity::Chest(slots)) = game.world.block_entities.get_mut(&pos) else {
        return false;
    };
    let Some(give) =
        super::tconstruct_source_station_world::take_operation(slots, &mut game.carried, plan)
    else {
        return false;
    };
    let mut dropped = Vec::new();
    for stack in give {
        let Some(info) = items.get(&stack.item) else {
            dropped.push(stack);
            continue;
        };
        let can_stack = info.max_stack > 1 && !(info.max_damage > 0 && stack.damage > 0);
        if let Some(rest) = super::tconstruct_tx::source_give_inventory(
            &mut game.inventory,
            game.selected,
            stack,
            info.max_stack,
            can_stack,
        ) {
            dropped.push(rest);
        }
    }
    game.world.modified.insert((pos.0 >> 4, pos.2 >> 4));
    for stack in dropped {
        game.throw_from_eyes(stack, eye, false);
    }
    true
}
