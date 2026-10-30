fn send_part_selection_data(hub: &mut Hub, ctx: &mut TickCtx<'_>) {
    let world = &mut *ctx.world;
    for i in 0..hub.guests.len() {
        if !hub.here(&hub.guests[i]) { continue; }
        let Some(menu) = hub.guests[i].menu.as_ref() else { continue; };
        let Some(pos) = menu.pos else { continue; };
        if !menu.slots.valid(world) { continue; }
        let state = world.block(pos.0, pos.1, pos.2).0 as usize;
        let builder = hub.sync.builtin.get("minecraft:block").and_then(|r| r.id("tconstruct:part_builder"));
        let station = hub.sync.builtin.get("minecraft:block").and_then(|r| r.id("tconstruct:tinker_station"));
        if station.is_some() && hub.encoder.block_ids.get(state).copied() == station {
            let Some(entity) = world.block_entities.get(&pos) else { continue; };
            let slots = entity.slots();
            let build = crate::tconstruct_source_station_world::building_plan(world, slots).map(|(_, output)| output);
            let operation = crate::tconstruct_source_station_world::operation_plan(world, slots).map(|(output, _)| output);
            let key = (hub.guests[i].conn, hub.guests[i].placed_at, menu.window, pos);
            let changed = world.chunks.__horizon_mixins_tconstruct_station_preview_sent().get(&key)
                .is_none_or(|(old_state, old_build, old_operation)| *old_state != menu.state || old_build != &build || old_operation != &operation);
            if changed {
                let build_net = hub.stack(&build);
                let operation_net = hub.stack(&operation);
                let guest = &mut hub.guests[i];
                let Some(menu) = guest.menu.as_mut() else { continue; };
                menu.state = (menu.state + 1) & 0x7FFF;
                let (window, state) = (menu.window, menu.state);
                guest.send(&C::ContainerSetSlot(play::ContainerSetSlot { container: window, state, slot: 24, item: build_net }));
                guest.send(&C::ContainerSetSlot(play::ContainerSetSlot { container: window, state, slot: 25, item: operation_net }));
                world.chunks.__horizon_mixins_tconstruct_station_preview_sent_mut().insert(key, (state, build, operation));
            }
            continue;
        }
        if builder.is_none() || hub.encoder.block_ids.get(state).copied() != builder { continue; }
        let patterns = crate::tconstruct_source_builder_world::refresh(world, pos);
        let encoded = patterns.as_ref().and_then(|patterns| crate::tconstruct_source_builder_wire::encode(patterns));
        let value = world.chunks.__horizon_mixins_tconstruct_part_selections().get(&pos)
            .and_then(|selected| patterns.as_ref()?.iter().position(|pattern| pattern == selected))
            .map(|index| index as i16).unwrap_or(-1);
        let preview = hub.sync.builtin.get("minecraft:item").and_then(|registry| {
            crate::tconstruct_source_builder_world::refresh_preview(world, pos,
                || registry.names.iter().filter(|id| !id.is_empty()).cloned().collect())
        });
        let preview_key = (hub.guests[i].conn, hub.guests[i].placed_at, menu.window, pos);
        let preview_changed = world.chunks.__horizon_mixins_tconstruct_builder_preview_sent().get(&preview_key)
            .is_none_or(|(old_state, old)| *old_state != menu.state || old != &preview);
        // The original cast cost is world recipe data. Send a complete frame
        // alongside the menu data, never an inferred or partially updated cost.
        let cast_frame = world.chunks.__horizon_mixins_tconstruct_source_cast_costs().as_ref()
            .and_then(|costs| crate::tconstruct_source_builder_wire::encode(&costs.iter().map(ToString::to_string).collect::<Vec<_>>()));
        let sent_frame = cast_frame.clone().unwrap_or_else(|| vec![i16::MIN]);
        let cast_changed = world.chunks.__horizon_mixins_tconstruct_builder_cast_wire_sent().get(&preview_key) != Some(&sent_frame);
        if cast_changed {
            world.chunks.__horizon_mixins_tconstruct_builder_cast_wire_sent_mut().insert(preview_key, sent_frame);
        }
        let guest = &mut hub.guests[i];
        let Some(menu) = guest.menu.as_mut() else { continue; };
        if menu.sent_data.is_empty() { menu.sent_data.push(i16::MIN); }
        let wire = encoded.as_deref().unwrap_or(&[i16::MIN]);
        let wire_changed = menu.sent_data.get(1..) != Some(wire);
        if wire_changed {
            menu.sent_data.truncate(1);
            menu.sent_data.extend_from_slice(wire);
        }
        let selection_changed = menu.sent_data[0] != value;
        menu.sent_data[0] = value;
        let window = menu.window;
        if wire_changed {
            let send = |guest: &mut crate::lan::guest::Guest, id, value| guest.send(&C::ContainerSetData(play::ContainerSetData { container: window, id, value }));
            if let Some(encoded) = encoded {
                send(guest, 1, -1);
                for (i, &word) in encoded[1..].iter().enumerate() { send(guest, (i + 2) as i16, word); }
                send(guest, 1, encoded[0]);
            } else { send(guest, 1, -2); }
        }
        if cast_changed {
            let send = |guest: &mut crate::lan::guest::Guest, id, value| guest.send(&C::ContainerSetData(play::ContainerSetData { container: window, id, value }));
            if let Some(frame) = cast_frame {
                send(guest, 20000, -1);
                for (i, &word) in frame[1..].iter().enumerate() { send(guest, 20001 + i as i16, word); }
                send(guest, 20000, frame[0]);
            } else { send(guest, 20000, -2); }
        }
        if selection_changed || wire_changed { guest.send(&C::ContainerSetData(play::ContainerSetData { container: window, id: 0, value })); }
        if preview_changed {
            let net = hub.stack(&preview);
            let guest = &mut hub.guests[i];
            let Some(menu) = guest.menu.as_mut() else { continue; };
            menu.state = (menu.state + 1) & 0x7FFF;
            let state = menu.state;
            guest.send(&C::ContainerSetSlot(play::ContainerSetSlot { container: window, state, slot: 22, item: net }));
            world.chunks.__horizon_mixins_tconstruct_builder_preview_sent_mut().insert(preview_key, (state, preview));
        }
    }
}
