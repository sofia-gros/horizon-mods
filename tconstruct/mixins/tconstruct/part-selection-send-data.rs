fn send_part_selection_data(hub: &mut Hub, ctx: &mut TickCtx<'_>) {
    let world = &mut *ctx.world;
    for i in 0..hub.guests.len() {
        if !hub.here(&hub.guests[i]) { continue; }
        let Some(menu) = hub.guests[i].menu.as_ref() else { continue; };
        let Some(pos) = menu.pos else { continue; };
        if !menu.slots.valid(world) { continue; }
        let state = world.block(pos.0, pos.1, pos.2).0 as usize;
        let builder = hub.sync.builtin.get("minecraft:block").and_then(|r| r.id("tconstruct:part_builder"));
        let actual = hub.encoder.block_ids.get(state).copied();
        let worktable = hub.sync.builtin.get("minecraft:block").and_then(|r| r.id("tconstruct:modifier_worktable")).is_some_and(|id| Some(id) == actual);
        let capacity = ["tconstruct:tinker_station", "tconstruct:tinkers_anvil", "tconstruct:scorched_anvil", "tconstruct:modifier_worktable"].into_iter()
            .find_map(|name| hub.sync.builtin.get("minecraft:block").and_then(|r| r.id(name))
                .filter(|id| Some(*id) == actual).and_then(|_| crate::tconstruct_source_anvil_capacity::input_capacity(name)));
        if let Some(capacity) = capacity {
            let (window, old_menu_state) = (menu.window, menu.state);
            if worktable {
                let selected = world.chunks.__horizon_mixins_tconstruct_part_selections().get(&pos).map(String::as_str);
                let index = selected.and_then(crate::tconstruct_source_upgrade_rules::selection_index).map(|index| index as i16).unwrap_or(-1);
                hub.guests[i].send(&C::ContainerSetData(play::ContainerSetData { container: window, id: 29999, value: index }));
            }
            let layouts = world.chunks.__horizon_mixins_tconstruct_material_snapshot().as_ref().map(|snapshot| {
                snapshot.iter().filter(|(id, _)| id.contains(":tinkering/station_layouts/"))
                    .filter_map(|(id, layers)| Some(vec![id.clone(), std::str::from_utf8(layers.last()?).ok()?.to_owned()]))
                    .flatten().collect::<Vec<_>>()
            });
            let frame = layouts.as_ref().and_then(|layouts| crate::tconstruct_source_builder_wire::encode(layouts));
            let wire_key = (hub.guests[i].conn, hub.guests[i].placed_at, window, pos);
            let wire = frame.as_deref().unwrap_or(&[i16::MIN]);
            let (_, wire_changed) = crate::tconstruct_source_menu_wire_cache::changed(
                world.chunks.__horizon_mixins_tconstruct_menu_wire_sent().get(&wire_key), -1, wire);
            if wire_changed {
                world.chunks.__horizon_mixins_tconstruct_menu_wire_sent_mut().insert(wire_key, (-1, wire.to_vec()));
            }
            let guest = &mut hub.guests[i];
            if let Some(menu) = guest.menu.as_mut() {
                if wire_changed {
                    let window = menu.window;
                    let send = |guest: &mut crate::lan::guest::Guest, id, value| guest.send(&C::ContainerSetData(play::ContainerSetData { container: window, id, value }));
                    if let Some(frame) = frame {
                        send(guest, 10000, -1);
                        for (index, &word) in frame[1..].iter().enumerate() { send(guest, 10001 + index as i16, word); }
                        send(guest, 10000, frame[0]);
                    } else { send(guest, 10000, -2); }
                }
            }
            let Some(entity) = world.block_entities.get(&pos) else { continue; };
            let slots = entity.slots();
            let mut build_reason = None;
            let mut operation_reason = None;
            let mut build = if worktable { None } else { crate::tconstruct_source_station_world::building_plan_diagnostic(world, slots, capacity, &mut build_reason).map(|(_, output)| output) };
            if let Some(output) = build.as_mut() {
                let name = world.chunks.__horizon_mixins_tconstruct_station_guest_names().get(&(hub.guests[i].conn, window, pos)).map(String::as_str);
                crate::tconstruct_source_station_name::name_result(output, name);
            }
            let mut operation = if worktable {
                world.chunks.__horizon_mixins_tconstruct_part_selections().get(&pos)
                    .and_then(|selected| crate::tconstruct_source_station_world::extraction_plan(world, slots, selected)).map(|plan| plan.output)
            } else { crate::tconstruct_source_station_world::operation_plan_diagnostic(world, slots, capacity, &mut operation_reason).map(|plan| plan.output) };
            if capacity == 5 {
                if let Some(output) = operation.as_mut() {
                    let name = world.chunks.__horizon_mixins_tconstruct_station_guest_names().get(&(hub.guests[i].conn, window, pos)).map(String::as_str);
                    crate::tconstruct_source_station_name::name_result(output, name);
                }
            }
            if build.is_none() && operation.is_none() {
                world.chunks.__horizon_mixins_tconstruct_station_guest_names_mut().remove(&(hub.guests[i].conn, window, pos));
            }
            let key = (hub.guests[i].conn, hub.guests[i].placed_at, window, pos);
            let tool = build.as_ref().or(operation.as_ref()).or_else(|| slots.first().and_then(Option::as_ref));
            let traits = tool.and_then(|tool| crate::tconstruct_source_material_traits::levels(world, tool))
                .and_then(|traits| serde_json::to_string(&traits.levels).ok()).unwrap_or_else(|| "{}".to_owned());
            let defense = tool.and_then(|tool| crate::tconstruct_source_defense_slots::display(world, tool))
                .and_then(|slots| serde_json::to_string(&slots).ok()).unwrap_or_else(|| " ".to_owned());
            // Keep existing field positions for traits and both error messages.
            let trait_frame = crate::tconstruct_source_builder_wire::encode(&[traits,
                build_reason.clone().unwrap_or_else(|| " ".into()),
                operation_reason.clone().unwrap_or_else(|| " ".into()), defense]).filter(|frame| frame.len() < 1767);
            let info_sent = trait_frame.clone().unwrap_or_else(|| vec![i16::MIN]);
            let info_changed = world.chunks.__horizon_mixins_tconstruct_builder_info_sent().get(&key) != Some(&info_sent);
            let changed = info_changed || world.chunks.__horizon_mixins_tconstruct_station_preview_sent().get(&key)
                .is_none_or(|(old_state, old_build, old_operation, old_build_reason, old_operation_reason)| *old_state != old_menu_state || old_build != &build || old_operation != &operation || old_build_reason != &build_reason || old_operation_reason != &operation_reason);
            if changed {
                let build_net = hub.stack(&build);
                let operation_net = hub.stack(&operation);
                let guest = &mut hub.guests[i];
                let Some(menu) = guest.menu.as_mut() else { continue; };
                menu.state = (menu.state + 1) & 0x7FFF;
                let (window, state) = (menu.window, menu.state);
                guest.send(&C::ContainerSetSlot(play::ContainerSetSlot { container: window, state, slot: 24, item: build_net }));
                guest.send(&C::ContainerSetSlot(play::ContainerSetSlot { container: window, state, slot: 25, item: operation_net }));
                if let Some(frame) = trait_frame {
                    guest.send(&C::ContainerSetData(play::ContainerSetData { container: window, id: 31000, value: -1 }));
                    for (index, &word) in frame[1..].iter().enumerate() {
                        guest.send(&C::ContainerSetData(play::ContainerSetData { container: window, id: 31001 + index as i16, value: word }));
                    }
                    guest.send(&C::ContainerSetData(play::ContainerSetData { container: window, id: 31000, value: frame[0] }));
                } else {
                    guest.send(&C::ContainerSetData(play::ContainerSetData { container: window, id: 31000, value: -2 }));
                }
                world.chunks.__horizon_mixins_tconstruct_builder_info_sent_mut().insert(key, info_sent);
                world.chunks.__horizon_mixins_tconstruct_station_preview_sent_mut().insert(key, (state, build, operation, build_reason, operation_reason));
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
        let mut info = crate::tconstruct_source_builder_world::display_info(world, pos);
        let side = match menu.slots { crate::lan::menu::Slots::Double(_, side) => Some(side), _ => None };
        let custom_name = side.and_then(|side| world.block_names.get(&side)).or_else(||
            world.chunks.__horizon_mixins_tconstruct_builder_double_partner().get(&pos).and_then(|partner| world.block_names.get(partner)))
            .cloned().unwrap_or_default();
        info.push(custom_name);
        let info_frame = crate::tconstruct_source_builder_wire::encode(&info);
        let info_frame = info_frame.filter(|frame| frame.len() < 1767);
        let info_sent = info_frame.clone().unwrap_or_else(|| vec![i16::MIN]);
        let info_changed = world.chunks.__horizon_mixins_tconstruct_builder_info_sent().get(&preview_key) != Some(&info_sent);
        if info_changed { world.chunks.__horizon_mixins_tconstruct_builder_info_sent_mut().insert(preview_key, info_sent); }
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
        let wire = encoded.as_deref().unwrap_or(&[i16::MIN]);
        let (selection_changed, wire_changed) = crate::tconstruct_source_menu_wire_cache::changed(
            world.chunks.__horizon_mixins_tconstruct_menu_wire_sent().get(&preview_key), value, wire);
        if selection_changed || wire_changed {
            world.chunks.__horizon_mixins_tconstruct_menu_wire_sent_mut().insert(preview_key, (value, wire.to_vec()));
        }
        let window = menu.window;
        if info_changed {
            let send = |guest: &mut crate::lan::guest::Guest, id, value| guest.send(&C::ContainerSetData(play::ContainerSetData { container: window, id, value }));
            if let Some(frame) = info_frame {
                send(guest, 31000, -1);
                for (index, &word) in frame[1..].iter().enumerate() { send(guest, 31001 + index as i16, word); }
                send(guest, 31000, frame[0]);
            } else { send(guest, 31000, -2); }
        }
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
