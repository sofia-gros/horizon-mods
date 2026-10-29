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
    if !is_station && packet.slot == 22 && packet.input == 1 {
        let mut dropped = Vec::new();
        if packet.button == 0 && crate::tconstruct_source_builder_world::player_only_quick_move(world, key) {
            let mut first_item = None;
            loop {
                let Ok(Some((plan, output, leftover))) = crate::tconstruct_source_builder_world::prepare_take(world, key, rules.items()) else { break; };
                if first_item.as_ref().is_some_and(|item| item != &output.item) { break; }
                if first_item.is_none() { first_item = Some(output.item.clone()); }
                let max = rules.max_stack(&output.item);
                let can_stack = rules.items().get(&output.item).is_some_and(|info| max > 1 && !(info.max_damage > 0 && output.damage > 0));
                let containers = world.block_entities.get(&key).map(|entity| entity.slots()).and_then(|slots| {
                    let material = crate::tconstruct_source_builder_world::crafting_container(slots.get(1).and_then(Option::as_ref), rules.items()).ok()?;
                    let pattern = crate::tconstruct_source_builder_world::crafting_container(slots.first().and_then(Option::as_ref), rules.items()).ok()?;
                    Some((material, pattern))
                });
                let (Some((material, pattern)), Some(BlockEntity::Chest(slots))) = (containers, world.block_entities.get_mut(&key)) else { break; };
                let guest = &mut hub.guests[gi];
                let Some(give) = crate::tconstruct_tx::source_quick_take_part_plan(slots, &mut guest.inventory, &plan, output, leftover, material, pattern, max, can_stack) else { break; };
                for stack in give {
                    let max = rules.max_stack(&stack.item);
                    let stackable = rules.items().get(&stack.item).is_some_and(|info| max > 1 && !(info.max_damage > 0 && stack.damage > 0));
                    if let Some(rest) = crate::tconstruct_tx::source_give_inventory(&mut guest.inventory, guest.selected, stack, max, stackable) { dropped.push(rest); }
                }
                world.modified.insert((key.0 >> 4, key.2 >> 4));
            }
        }
        if let Some(items) = hub.guests[gi].menu.as_ref().and_then(|m| m.slots.read(world, &hub.guests[gi])) { hub.send_menu_content(gi, items); }
        return Break(dropped);
    }
    if packet.input != 0 || (!is_station && (24..=26).contains(&packet.slot)) || ((2..=23).contains(&packet.slot) && (!is_station && packet.slot != 22 || is_station && packet.slot != 2)) {
        if let Some(items) = hub.guests[gi].menu.as_ref().and_then(|m| m.slots.read(world, &hub.guests[gi])) { hub.send_menu_content(gi, items); }
        return Break(Vec::new());
    }
    if !is_station && packet.slot == 22 {
        let mut dropped = Vec::new();
        if packet.button == 0 {
            if let Ok(Some((plan, output, leftover))) = crate::tconstruct_source_builder_world::prepare_take(world, key, rules.items()) {
                let containers = world.block_entities.get(&key).map(|entity| entity.slots()).and_then(|slots| {
                    let material = crate::tconstruct_source_builder_world::crafting_container(slots.get(1).and_then(Option::as_ref), rules.items()).ok()?;
                    let pattern = crate::tconstruct_source_builder_world::crafting_container(slots.first().and_then(Option::as_ref), rules.items()).ok()?;
                    Some((material, pattern))
                });
                if let (Some((material, pattern)), Some(BlockEntity::Chest(slots))) = (containers, world.block_entities.get_mut(&key)) {
                    let max = rules.max_stack(&output.item);
                    let guest = &mut hub.guests[gi];
                    if let Some(give) = crate::tconstruct_tx::source_take_part_plan(slots, &mut guest.carried, &plan, output, leftover, material, pattern, max) {
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
    let action = if is_station && (24..=25).contains(&packet.slot) { Some(packet.slot) } else { None };
    let Some(action) = action else { return Continue(()); };
    if packet.button == 0 {
        if let Some(BlockEntity::Chest(stacks)) = world.block_entities.get_mut(&key) {
            if crate::tconstruct_tx::take(stacks, &mut hub.guests[gi].carried, is_station, action) {
                world.modified.insert((key.0 >> 4, key.2 >> 4));
            }
        }
    }
    if let Some(items) = hub.guests[gi].menu.as_ref().and_then(|m| m.slots.read(world, &hub.guests[gi])) { hub.send_menu_content(gi, items); }
    Break(Vec::new())
}
