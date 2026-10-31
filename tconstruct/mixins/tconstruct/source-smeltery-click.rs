fn smeltery_click(game: &mut crate::game::Game<'_>, slot: Option<usize>, button: i32, input: horizon_client::menu::Input) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    if !crate::game::tconstruct_source_smeltery_ui::is_smeltery(game) { return Continue(()); }
    let (count, offset) = crate::game::tconstruct_source_smeltery_ui::page(game);
    // Reject outside clicks and every synthetic container slot before the
    // generic menu can alter the carried stack or player inventory.
    let Some(slot) = slot else { return Break(true); };
    if (36..300).contains(&slot) { return Break(true); }
    if (300..302).contains(&slot) {
        if game.remote.is_some() { return Continue(()); }
        let Some(pos) = game.container else { return Break(true); };
        let Some(mut state) = crate::game::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:smeltery")
            .and_then(|tag| crate::game::tconstruct_source_smeltery_state::Smeltery::from_nbt(&tag)) else { return Break(true); };
        let (mut containers, direction) = game.world.chunks.__horizon_mixins_tconstruct_smeltery_local_containers()
            .get(&pos).cloned().unwrap_or(([None, None], 0));
        if input == horizon_client::menu::Input::QuickMove {
            if let Some(stack) = containers[slot - 300].clone() {
                let max = game.res.items().get(&stack.item).map_or(64, |item| item.max_stack).max(1);
                let mut inventory = game.inventory.clone();
                let before = stack.count;
                let remaining = crate::game::tconstruct_tx::source_quick_move_player(&mut inventory, stack, max, max > 1);
                if remaining.as_ref().map_or(0, |stack| stack.count) != before {
                    containers[slot - 300] = remaining;
                    game.inventory = inventory;
                    game.world.chunks.__horizon_mixins_tconstruct_smeltery_local_containers_mut().insert(pos, (containers.clone(), direction));
                    if let Some(horizon_client::containers::BlockEntity::Chest(live)) = game.world.block_entities.get_mut(&pos) {
                        *live = crate::game::tconstruct_source_smeltery_page::project_with_containers(&state.slots, offset, &containers);
                    }
                }
            }
            return Break(true);
        }
        let mut carried = game.carried.clone();
        let known_fluids = game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref()
            .and_then(|value| value.as_ref().ok()).map(|(fluids, _)| fluids.clone()).unwrap_or_default();
        if input == horizon_client::menu::Input::Pickup && matches!(button, 0 | 1)
            && crate::game::tconstruct_source_fluid_container::click(&mut containers, &mut carried, slot - 300, button == 1, &|id| known_fluids.contains(id)) {
            crate::game::tconstruct_source_fluid_container::transfer(&mut state.tank, &mut containers,
                crate::game::tconstruct_source_fluid_container::Direction::from_index(direction),
                &|id| game.res.items().get(id).is_some(), &|id| known_fluids.contains(id));
            if crate::game::tconstruct_source_melter_store::write(&mut game.world, pos, "tconstruct:smeltery", state.to_nbt()) {
                game.carried = carried;
                game.world.chunks.__horizon_mixins_tconstruct_smeltery_local_containers_mut().insert(pos, (containers.clone(), direction));
                if let Some(horizon_client::containers::BlockEntity::Chest(live)) = game.world.block_entities.get_mut(&pos) {
                    *live = crate::game::tconstruct_source_smeltery_page::project_with_containers(&state.slots, offset, &containers);
                }
            }
        }
        return Break(true);
    }
    if slot >= 300 && (slot >= 354 || crate::game::tconstruct_source_smeltery_layout::melting_index(count, offset, slot - 300).is_none()) { return Break(true); }
    if !matches!(input, horizon_client::menu::Input::Pickup | horizon_client::menu::Input::QuickMove) { return Break(true); }
    if game.remote.is_some() { return Continue(()); }
    let Some(pos) = game.container else { return Break(true); };
    let Some(mut state) = crate::game::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:smeltery")
        .and_then(|tag| crate::game::tconstruct_source_smeltery_state::Smeltery::from_nbt(&tag)) else { return Break(true); };
    if input == horizon_client::menu::Input::QuickMove && slot < 36 {
        let fluids = game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref()
            .and_then(|value| value.as_ref().ok()).map(|(fluids, _)| fluids.clone()).unwrap_or_default();
        let mut stack = game.inventory.get(slot).cloned().flatten();
        if stack.as_ref().is_some_and(|stack| crate::game::tconstruct_source_fluid_container::accepts(stack, &|fluid| fluids.contains(fluid))) {
            let (mut containers, mode) = game.world.chunks.__horizon_mixins_tconstruct_smeltery_local_containers()
                .get(&pos).cloned().unwrap_or(([None, None], 0));
            if crate::game::tconstruct_source_fluid_container::quick_input(&mut containers, &mut stack, &|fluid| fluids.contains(fluid)) {
                crate::game::tconstruct_source_fluid_container::transfer(&mut state.tank, &mut containers,
                    crate::game::tconstruct_source_fluid_container::Direction::from_index(mode),
                    &|item| game.res.items().get(item).is_some(), &|fluid| fluids.contains(fluid));
                if crate::game::tconstruct_source_melter_store::write(&mut game.world, pos, "tconstruct:smeltery", state.to_nbt()) {
                    game.inventory[slot] = stack;
                    game.world.chunks.__horizon_mixins_tconstruct_smeltery_local_containers_mut().insert(pos, (containers.clone(), mode));
                    if let Some(horizon_client::containers::BlockEntity::Chest(live)) = game.world.block_entities.get_mut(&pos) {
                        *live = crate::game::tconstruct_source_smeltery_page::project_with_containers(&state.slots, offset, &containers);
                    }
                }
            }
            return Break(true);
        }
    }
    if input == horizon_client::menu::Input::QuickMove {
        let mut inventory = game.inventory.clone();
        let before_slots = state.slots.clone();
        let source = if slot < 36 { inventory.get(slot).cloned().flatten() }
            else { crate::game::tconstruct_source_smeltery_layout::melting_index(count, offset, slot - 300).and_then(|index| state.slots[index].clone()) };
        if let Some(stack) = source {
            let info = game.res.items().get(&stack.item);
            let max = info.map_or(64, |item| item.max_stack).max(1);
            let stackable = info.is_some_and(|item| max > 1 && !(item.max_damage > 0 && stack.damage > 0));
            let before = stack.count;
            let remaining = if slot < 36 {
                let remaining = crate::game::tconstruct_tx::source_give_inventory(&mut state.slots, 0, stack, max, stackable);
                inventory[slot] = remaining.clone(); remaining
            } else {
                let Some(index) = crate::game::tconstruct_source_smeltery_layout::melting_index(count, offset, slot - 300) else { return Break(true); };
                let remaining = crate::game::tconstruct_tx::source_quick_move_player(&mut inventory, stack, max, stackable);
                state.slots[index] = remaining.clone(); remaining
            };
            for (index, (before, after)) in before_slots.iter().zip(&state.slots).enumerate() {
                let same = match (before, after) {
                    (None, None) => true,
                    (Some(a), Some(b)) => a.item == b.item && a.damage == b.damage && a.extra == b.extra,
                    _ => false,
                };
                if !same { state.progress[index] = 0; }
            }
            if remaining.as_ref().map_or(0, |item| item.count) != before && crate::game::tconstruct_source_melter_store::write(&mut game.world, pos, "tconstruct:smeltery", state.to_nbt()) {
                game.inventory = inventory;
                if let Some(horizon_client::containers::BlockEntity::Chest(live)) = game.world.block_entities.get_mut(&pos) {
                    let containers = game.world.chunks.__horizon_mixins_tconstruct_smeltery_local_containers()
                        .get(&pos).map(|(slots, _)| slots.clone()).unwrap_or([None, None]);
                    *live = crate::game::tconstruct_source_smeltery_page::project_with_containers(&state.slots, offset, &containers);
                }
            }
        }
        return Break(true);
    }
    let containers = game.world.chunks.__horizon_mixins_tconstruct_smeltery_local_containers().get(&pos)
        .map(|(slots, _)| slots.clone()).unwrap_or([None, None]);
    let expected = crate::game::tconstruct_source_smeltery_page::project_with_containers(&state.slots, offset, &containers);
    let Some(horizon_client::containers::BlockEntity::Chest(live)) = game.world.block_entities.get_mut(&pos) else { return Break(true); };
    let previous_live = live.clone();
    *live = expected.clone();
    let previous_inventory = game.inventory.clone();
    let previous_carried = game.carried.clone();
    let handled = game.menu_ops(&[(Some(slot), button, input)]);
    let changed = game.world.block_entities.get(&pos).map(|entity| entity.slots().to_vec());
    let before_slots = state.slots.clone();
    let committed = changed.as_ref().is_some_and(|view| crate::game::tconstruct_source_smeltery_page::commit_with_containers(&mut state.slots, offset, &expected, view, &containers));
    if committed {
        for (index, (before, after)) in before_slots.iter().zip(&state.slots).enumerate() {
            let same = match (before, after) {
                (None, None) => true,
                (Some(a), Some(b)) => a.item == b.item && a.damage == b.damage && a.extra == b.extra,
                _ => false,
            };
            if !same { state.progress[index] = 0; }
        }
        if crate::game::tconstruct_source_melter_store::write(&mut game.world, pos, "tconstruct:smeltery", state.to_nbt()) { return Break(handled); }
    }
    game.inventory = previous_inventory;
    game.carried = previous_carried;
    if let Some(horizon_client::containers::BlockEntity::Chest(live)) = game.world.block_entities.get_mut(&pos) { *live = previous_live; }
    Break(true)
}
