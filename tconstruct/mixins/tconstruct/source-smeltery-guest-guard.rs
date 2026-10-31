fn smeltery_guest_guard(hub: &mut crate::lan::hub::Hub, conn: u32, packet: &play::ContainerClick, world: &mut World, rules: &dyn Rules) -> std::ops::ControlFlow<Vec<ItemStack>> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(gi) = hub.guests.iter().position(|guest| guest.conn == conn) else { return Continue(()); };
    let Some(menu) = hub.guests[gi].menu.as_ref() else { return Continue(()); };
    if packet.container == 0 || menu.window != packet.container { return Continue(()); }
    let Some(pos) = menu.pos else { return Continue(()); };
    let Some(tag) = crate::tconstruct_source_melter_store::read(world, pos, "tconstruct:smeltery") else { return Continue(()); };
    let Some(mut state) = crate::tconstruct_source_smeltery_state::Smeltery::from_nbt(&tag) else { return Break(Vec::new()); };
    let key = (conn, hub.guests[gi].placed_at, pos);
    let (offset, visible_count) = world.chunks.__horizon_mixins_tconstruct_smeltery_guest_pages().get(&key).copied().unwrap_or((0, state.slots.len()));
    let menu_state = menu.state;
    if visible_count != state.slots.len() {
        let offset = offset.min(crate::tconstruct_source_smeltery_layout::max_offset(state.slots.len()));
        world.chunks.__horizon_mixins_tconstruct_smeltery_guest_pages_mut().insert(key, (offset, state.slots.len()));
        hub.guests[gi].send(&C::ContainerSetData(play::ContainerSetData { container: packet.container, id: 30002, value: state.slots.len() as i16 }));
        hub.guests[gi].send(&C::ContainerSetData(play::ContainerSetData { container: packet.container, id: 30003, value: offset as i16 }));
        hub.send_menu_content(gi, crate::tconstruct_source_smeltery_page::project(&state.slots, offset));
        return Break(Vec::new());
    }
    // Reject stale pages and virtual slots before any generic transaction.
    let virtual_slot = (0..54).contains(&packet.slot) && crate::tconstruct_source_smeltery_layout::melting_index(state.slots.len(), offset, packet.slot as usize).is_none();
    if packet.state != menu_state || virtual_slot {
        hub.send_menu_content(gi, crate::tconstruct_source_smeltery_page::project(&state.slots, offset));
        return Break(Vec::new());
    }
    if packet.input == 1 {
        let mut inventory = hub.guests[gi].inventory.clone();
        let before_slots = state.slots.clone();
        let container_index = usize::try_from(packet.slot).ok().filter(|slot| *slot < 54)
            .and_then(|slot| crate::tconstruct_source_smeltery_layout::melting_index(state.slots.len(), offset, slot));
        let player_index = usize::try_from(packet.slot).ok().filter(|slot| (54..90).contains(slot)).map(|slot| {
            let index = slot - 54; if index < 27 { index + 9 } else { index - 27 }
        });
        let stack = container_index.and_then(|index| state.slots[index].clone())
            .or_else(|| player_index.and_then(|index| inventory[index].clone()));
        if let Some(stack) = stack {
            let max = rules.max_stack(&stack.item).max(1);
            let stackable = rules.items().get(&stack.item).is_some_and(|info| max > 1 && !(info.max_damage > 0 && stack.damage > 0));
            let before = stack.count;
            let remaining = if let Some(index) = container_index {
                let remaining = crate::tconstruct_tx::source_quick_move_player(&mut inventory, stack, max, stackable);
                state.slots[index] = remaining.clone();
                if remaining.is_none() { state.progress[index] = 0; }
                remaining
            } else if let Some(index) = player_index {
                let remaining = crate::tconstruct_tx::source_give_inventory(&mut state.slots, 0, stack, max, stackable);
                inventory[index] = remaining.clone();
                remaining
            } else { None };
            for (index, (old, new)) in before_slots.iter().zip(&state.slots).enumerate() {
                let same = match (old, new) {
                    (None, None) => true,
                    (Some(a), Some(b)) => a.item == b.item && a.damage == b.damage && a.extra == b.extra,
                    _ => false,
                };
                if !same { state.progress[index] = 0; }
            }
            if remaining.as_ref().map_or(0, |item| item.count) != before && crate::tconstruct_source_melter_store::write(world, pos, "tconstruct:smeltery", state.to_nbt()) {
                hub.guests[gi].inventory = inventory;
            }
        }
        let current = crate::tconstruct_source_melter_store::read(world, pos, "tconstruct:smeltery")
            .and_then(|tag| crate::tconstruct_source_smeltery_state::Smeltery::from_nbt(&tag)).unwrap_or(state);
        hub.send_menu_content(gi, crate::tconstruct_source_smeltery_page::project(&current.slots, offset));
        return Break(Vec::new());
    }
    if packet.input != 0 {
        hub.send_menu_content(gi, crate::tconstruct_source_smeltery_page::project(&state.slots, offset));
        return Break(Vec::new());
    }
    Continue(())
}
