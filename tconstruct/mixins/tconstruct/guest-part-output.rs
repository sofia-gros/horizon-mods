fn guest_tconstruct_part_output(hub: &mut crate::lan::hub::Hub, conn: u32, packet: &play::ContainerClick, world: &mut World, rules: &dyn Rules) -> std::ops::ControlFlow<Vec<ItemStack>> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(gi) = hub.guests.iter().position(|g| g.conn == conn) else { return Continue(()); };
    let Some(menu) = hub.guests[gi].menu.as_ref() else { return Continue(()); };
    if packet.container == 0 || menu.window != packet.container || hub.guests[gi].spectator() || !menu.slots.still_valid(world, &hub.guests[gi], menu.pos) { return Continue(()); }
    let Some(key) = menu.pos else { return Continue(()); };
    let state = world.block(key.0, key.1, key.2).0 as usize;
    let actual = hub.encoder.block_ids.get(state).copied();
    let block = hub.sync.builtin.get("minecraft:block");
    let builder = block.and_then(|r| r.id("tconstruct:part_builder"));
    let station = block.and_then(|r| r.id("tconstruct:tinker_station"));
    let is_station = if builder.is_some() && actual == builder { false } else if station.is_some() && actual == station { true } else { return Continue(()); };
    let side_shulker = if let crate::lan::menu::Slots::Double(_, side) = menu.slots {
        let side_state = world.block(side.0, side.1, side.2).0 as usize;
        hub.encoder.block_ids.get(side_state).and_then(|id| usize::try_from(*id).ok())
            .and_then(|id| block.and_then(|registry| registry.names.get(id)))
            .is_some_and(|name| name.ends_with("shulker_box"))
    } else { false };
    if !is_station && side_shulker && (27..54).contains(&packet.slot)
        && hub.guests[gi].carried.as_ref().is_some_and(|stack| !crate::menu::fits_inside_container_items(stack)) {
        if let Some(items) = hub.guests[gi].menu.as_ref().and_then(|m| m.slots.read(world, &hub.guests[gi])) { hub.send_menu_content(gi, items); }
        return Break(Vec::new());
    }
    let hopper_side = !is_station && matches!(menu.slots, crate::lan::menu::Slots::Double(_, side) if world.block_entities.get(&side).is_some_and(|entity| entity.slots().len() == 5));
    // Padding is not a real inventory and cannot receive a click.
    if hopper_side && (32..36).contains(&packet.slot) {
        if let Some(items) = hub.guests[gi].menu.as_ref().and_then(|m| m.slots.read(world, &hub.guests[gi])) { hub.send_menu_content(gi, items); }
        return Break(Vec::new());
    }
    if !is_station && packet.input == 1 && (menu.size as i16..(menu.size + 36) as i16).contains(&packet.slot) && matches!(menu.slots, crate::lan::menu::Slots::Double(_, _)) {
        let index = (packet.slot - menu.size as i16) as usize;
        let inventory_index = if index < 27 { index + 9 } else { index - 27 };
        let side_pos = match menu.slots { crate::lan::menu::Slots::Double(_, side) => side, _ => unreachable!() };
        let Some(stack) = hub.guests[gi].inventory.get(inventory_index).cloned().flatten() else {
            if let Some(items) = hub.guests[gi].menu.as_ref().and_then(|m| m.slots.read(world, &hub.guests[gi])) { hub.send_menu_content(gi, items); }
            return Break(Vec::new());
        };
        let tags = world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref().and_then(|tags| tags.as_ref().ok());
        let Some(patterns) = tags.and_then(|tags| tags.get("tconstruct:patterns")) else {
            if let Some(items) = hub.guests[gi].menu.as_ref().and_then(|m| m.slots.read(world, &hub.guests[gi])) { hub.send_menu_content(gi, items); }
            return Break(Vec::new());
        };
        let pattern = patterns.contains(&*stack.item);
        let max = rules.max_stack(&stack.item).max(1);
        let stackable = rules.items().get(&stack.item).is_some_and(|info| max > 1 && !(info.max_damage > 0 && stack.damage > 0));
        let Some(BlockEntity::Chest(mut inputs)) = world.block_entities.get(&key).cloned() else { return Break(Vec::new()); };
        let Some(BlockEntity::Chest(mut side)) = world.block_entities.get(&side_pos).cloned() else { return Break(Vec::new()); };
        let first_side_len = side.len();
        let partner = world.chunks.__horizon_mixins_tconstruct_builder_double_partner().get(&key).copied();
        if let Some(partner) = partner {
            let Some(BlockEntity::Chest(items)) = world.block_entities.get(&partner) else { return Break(Vec::new()); };
            side.extend_from_slice(items);
        }
        let old_count = stack.count;
        let remaining = crate::tconstruct_tx::source_quick_move_to_builder(&mut inputs, &mut side, stack, max, stackable, pattern);
        if remaining.as_ref().map_or(0, |item| item.count) != old_count {
            if let Some(BlockEntity::Chest(value)) = world.block_entities.get_mut(&key) { *value = inputs; }
            if let Some(BlockEntity::Chest(value)) = world.block_entities.get_mut(&side_pos) { *value = side[..first_side_len].to_vec(); }
            if let Some(partner) = partner {
                if let Some(BlockEntity::Chest(value)) = world.block_entities.get_mut(&partner) { *value = side[first_side_len..].to_vec(); }
                world.modified.insert((partner.0 >> 4, partner.2 >> 4));
            }
            hub.guests[gi].inventory[inventory_index] = remaining;
            world.modified.insert((key.0 >> 4, key.2 >> 4));
            world.modified.insert((side_pos.0 >> 4, side_pos.2 >> 4));
        }
        if let Some(items) = hub.guests[gi].menu.as_ref().and_then(|m| m.slots.read(world, &hub.guests[gi])) { hub.send_menu_content(gi, items); }
        return Break(Vec::new());
    }
    if !is_station && packet.slot == 22 && packet.input == 1 {
        let mut dropped = Vec::new();
        let mut crafted = Vec::new();
        let mut first_item = None;
        let side_pos = match menu.slots {
            crate::lan::menu::Slots::Double(first, side) if first == key => Some(side),
            _ => None,
        };
        let partner_pos = world.chunks.__horizon_mixins_tconstruct_builder_double_partner().get(&key).copied();
        if (packet.button == 0 || packet.button == 1) && (side_pos.is_some() || crate::tconstruct_source_builder_world::player_only_quick_move(world, key)) {
            loop {
                let Ok(Some((plan, output, leftover, alternatives))) = crate::tconstruct_source_builder_world::prepare_take(world, key, rules.items()) else { break; };
                if first_item.as_ref().is_some_and(|item| item != &output.item) { break; }
                if first_item.is_none() { first_item = Some(output.item.clone()); }
                let max = rules.max_stack(&output.item);
                let can_stack = rules.items().get(&output.item).is_some_and(|info| max > 1 && !(info.max_damage > 0 && output.damage > 0));
                let containers = world.block_entities.get(&key).map(|entity| entity.slots()).and_then(|slots| {
                    let material = crate::tconstruct_source_builder_world::crafting_container(slots.get(1).and_then(Option::as_ref), rules.items()).ok()?;
                    let pattern = crate::tconstruct_source_builder_world::crafting_container(slots.first().and_then(Option::as_ref), rules.items()).ok()?;
                    Some((material, pattern))
                });
                let mut side = match side_pos {
                    Some(side_pos) => match world.block_entities.get(&side_pos) { Some(BlockEntity::Chest(items)) => items.clone(), _ => break },
                    None => Vec::new(),
                };
                let first_side_len = side.len();
                if let Some(partner) = partner_pos {
                    let Some(BlockEntity::Chest(items)) = world.block_entities.get(&partner) else { break; };
                    side.extend_from_slice(items);
                }
                let (Some((material, pattern)), Some(BlockEntity::Chest(slots))) = (containers, world.block_entities.get_mut(&key)) else { break; };
                let guest = &mut hub.guests[gi];
                let crafted_item = output.item.to_string();
                let Some((give, crafted_count)) = crate::tconstruct_tx::source_quick_take_part_plan(slots, &mut guest.inventory, &mut side, &plan, output, leftover, alternatives, material, pattern, max, can_stack, |bound| crate::tconstruct_source_builder_world::source_random_draw(rules.items(), bound)) else { break; };
                crafted.push((crafted_item, crafted_count));
                if let Some(side_pos) = side_pos {
                    if let Some(BlockEntity::Chest(items)) = world.block_entities.get_mut(&side_pos) { *items = side[..first_side_len].to_vec(); }
                    world.modified.insert((side_pos.0 >> 4, side_pos.2 >> 4));
                }
                if let Some(partner) = partner_pos {
                    if let Some(BlockEntity::Chest(items)) = world.block_entities.get_mut(&partner) { *items = side[first_side_len..].to_vec(); }
                    world.modified.insert((partner.0 >> 4, partner.2 >> 4));
                }
                for stack in give {
                    let max = rules.max_stack(&stack.item);
                    let stackable = rules.items().get(&stack.item).is_some_and(|info| max > 1 && !(info.max_damage > 0 && stack.damage > 0));
                    if let Some(rest) = crate::tconstruct_tx::source_give_inventory(&mut guest.inventory, guest.selected, stack, max, stackable) { dropped.push(rest); }
                }
                world.modified.insert((key.0 >> 4, key.2 >> 4));
            }
        }
        for (item, count) in crafted {
            world.chunks.__horizon_mixins_tconstruct_builder_crafted_mut().push((conn, item, count, hub.guests[gi].position));
        }
        if let Some(items) = hub.guests[gi].menu.as_ref().and_then(|m| m.slots.read(world, &hub.guests[gi])) { hub.send_menu_content(gi, items); }
        return Break(dropped);
    }
    // Inactive inputs remain visible only while occupied and can be removed,
    // never used as a destination. Selection is scoped to this viewer/menu.
    if is_station && (0..4).contains(&packet.slot) {
        let operation = world.block_entities.get(&key).is_some_and(|entity| crate::tconstruct_source_station_world::has_operation_tool(entity.slots()));
        let count = if operation { 4 } else { world.chunks.__horizon_mixins_tconstruct_station_guest_layout_count().get(&(conn, packet.container, key)).copied().unwrap_or(4) };
        if packet.slot as usize >= count {
            let occupied = world.block_entities.get(&key).and_then(|entity| entity.slots().get(packet.slot as usize)).and_then(Option::as_ref).is_some();
            if !occupied || hub.guests[gi].carried.is_some() || !matches!(packet.input, 0 | 1) {
                if let Some(items) = hub.guests[gi].menu.as_ref().and_then(|m| m.slots.read(world, &hub.guests[gi])) { hub.send_menu_content(gi, items); }
                return Break(Vec::new());
            }
            if packet.input == 1 { return Continue(()); }
        }
    }
    // The slot belongs to the underlying Station inventory even before the
    // tool is inserted. Layout selection only controls whether it is visible.
    let repair_slot = is_station && packet.slot == 3;
    if packet.input != 0 || (!is_station && (24..=26).contains(&packet.slot)) || ((2..=23).contains(&packet.slot) && (!is_station && packet.slot != 22 || is_station && packet.slot != 2 && !repair_slot)) {
        if let Some(items) = hub.guests[gi].menu.as_ref().and_then(|m| m.slots.read(world, &hub.guests[gi])) { hub.send_menu_content(gi, items); }
        return Break(Vec::new());
    }
    if !is_station && packet.slot == 22 {
        let mut dropped = Vec::new();
        if packet.button == 0 || packet.button == 1 {
            if let Ok(Some((plan, output, leftover, alternatives))) = crate::tconstruct_source_builder_world::prepare_take(world, key, rules.items()) {
                let containers = world.block_entities.get(&key).map(|entity| entity.slots()).and_then(|slots| {
                    let material = crate::tconstruct_source_builder_world::crafting_container(slots.get(1).and_then(Option::as_ref), rules.items()).ok()?;
                    let pattern = crate::tconstruct_source_builder_world::crafting_container(slots.first().and_then(Option::as_ref), rules.items()).ok()?;
                    Some((material, pattern))
                });
                if let (Some((material, pattern)), Some(BlockEntity::Chest(slots))) = (containers, world.block_entities.get_mut(&key)) {
                    let max = rules.max_stack(&output.item);
                    let guest = &mut hub.guests[gi];
                    let crafted_item = output.item.clone();
                    let crafted_count = output.count;
                    if let Some(give) = crate::tconstruct_tx::source_take_part_plan(slots, &mut guest.carried, &plan, output, leftover, alternatives, material, pattern, max, |bound| crate::tconstruct_source_builder_world::source_random_draw(rules.items(), bound)) {
                        world.chunks.__horizon_mixins_tconstruct_builder_crafted_mut().push((conn, crafted_item.to_string(), crafted_count, guest.position));
                        for stack in give {
                            let max = rules.max_stack(&stack.item);
                            let can_stack = rules.items().get(&stack.item).is_some_and(|info| max > 1 && !(info.max_damage > 0 && stack.damage > 0));
                            if let Some(rest) = crate::tconstruct_tx::source_give_inventory(&mut guest.inventory, guest.selected, stack, max, can_stack) { dropped.push(rest); }
                        }
                        world.modified.insert((key.0 >> 4, key.2 >> 4));
                    }
                }
            }
        }
        if let Some(items) = hub.guests[gi].menu.as_ref().and_then(|m| m.slots.read(world, &hub.guests[gi])) { hub.send_menu_content(gi, items); }
        return Break(dropped);
    }
    let mut dropped = Vec::new();
    let action = if is_station && (24..=25).contains(&packet.slot) { Some(packet.slot) } else { None };
    let Some(action) = action else { return Continue(()); };
    if packet.button == 0 {
        let mut building = if action == 24 {
            world.block_entities.get(&key).and_then(|entity| crate::tconstruct_source_station_world::building_plan(world, entity.slots()))
        } else { None };
        if let Some((_, output)) = building.as_mut() {
            let name = world.chunks.__horizon_mixins_tconstruct_station_guest_names().get(&(conn, packet.container, key)).map(String::as_str);
            crate::tconstruct_source_station_name::name_result(output, name);
        }
        let repair = if action == 25 {
            world.block_entities.get(&key).and_then(|entity| crate::tconstruct_source_station_world::operation_plan(world, entity.slots()))
        } else { None };
        if let Some(BlockEntity::Chest(stacks)) = world.block_entities.get_mut(&key) {
            let give = if action == 24 {
                building.and_then(|(matched, output)| {
                    let max_stack = rules.max_stack(&output.item);
                    crate::tconstruct_source_station_recipes::take_built_tool(stacks, &mut hub.guests[gi].carried, &matched, output, max_stack).then(Vec::new)
                })
            } else { repair.and_then(|plan| crate::tconstruct_source_station_world::take_operation(stacks, &mut hub.guests[gi].carried, plan)) };
            if let Some(give) = give {
                if action == 24 { world.chunks.__horizon_mixins_tconstruct_station_guest_names_mut().remove(&(conn, packet.container, key)); }
                for stack in give {
                    let max = rules.max_stack(&stack.item);
                    let can_stack = rules.items().get(&stack.item).is_some_and(|info| max > 1 && !(info.max_damage > 0 && stack.damage > 0));
                    let guest = &mut hub.guests[gi];
                    if let Some(rest) = crate::tconstruct_tx::source_give_inventory(&mut guest.inventory, guest.selected, stack, max, can_stack) { dropped.push(rest); }
                }
                world.modified.insert((key.0 >> 4, key.2 >> 4));
            }
        }
    }
    if let Some(items) = hub.guests[gi].menu.as_ref().and_then(|m| m.slots.read(world, &hub.guests[gi])) { hub.send_menu_content(gi, items); }
    Break(dropped)
}
