fn smeltery_guest_button(game: &mut crate::game::Game<'_>, host: &mut crate::game::lan::LanHost, conn: u32, window: i32, id: i32) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    if (1..3).contains(&id) || id < 0 { return Continue(()); }
    let Some(gi) = host.hub.guests.iter().position(|guest| guest.conn == conn) else { return Continue(()); };
    let Some(menu) = host.hub.guests[gi].menu.as_ref() else { return Continue(()); };
    if menu.window != window || !menu.slots.still_valid(&game.world, &host.hub.guests[gi], menu.pos) { return Continue(()); }
    let Some(pos) = menu.pos else { return Continue(()); };
    let Some(mut state) = crate::game::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:smeltery")
        .and_then(|tag| crate::game::tconstruct_source_smeltery_state::Smeltery::from_nbt(&tag)) else { return Continue(()); };
    if id == 0 {
        let (mut slots, mode) = host.hub.guests[gi].__horizon_mixins_tconstruct_smeltery_guest_containers().as_ref()
            .filter(|(at, _, _)| *at == pos).map(|(_, slots, mode)| (slots.clone(), *mode)).unwrap_or(([None, None], 0));
        let direction = crate::game::tconstruct_source_fluid_container::Direction::from_index(mode).next();
        let known_fluids = game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref()
            .and_then(|value| value.as_ref().ok()).map(|(fluids, _)| fluids.clone()).unwrap_or_default();
        crate::game::tconstruct_source_fluid_container::transfer(&mut state.tank, &mut slots, direction,
            &|item| game.res.items().get(item).is_some(), &|fluid| known_fluids.contains(fluid));
        if crate::game::tconstruct_source_melter_store::write(&mut game.world, pos, "tconstruct:smeltery", state.to_nbt()) {
            *host.hub.guests[gi].__horizon_mixins_tconstruct_smeltery_guest_containers_mut() = Some((pos, slots, direction.index()));
            host.hub.guests[gi].send(&horizon_protocol::packets::play::Clientbound::ContainerSetData(horizon_protocol::packets::play::ContainerSetData { container: window, id: 30004, value: i16::from(direction.index()) }));
        }
        return Break(());
    }
    if id == 3 {
        if host.hub.guests[gi].carried.is_some() { crate::game::tconstruct_source_fluid_direct::guest(game, host, gi, pos, None); }
        return Break(());
    }
    if !matches!(id, 32000 | 32001) && !(33000..=45348).contains(&id) {
        let Some(index) = usize::try_from(id - 4).ok() else { return Break(()); };
        if host.hub.guests[gi].carried.is_some() {
            crate::game::tconstruct_source_fluid_direct::guest(game, host, gi, pos, Some(index));
        } else if state.tank.move_to_bottom(index) {
            let (mut slots, mode) = host.hub.guests[gi].__horizon_mixins_tconstruct_smeltery_guest_containers().as_ref()
                .filter(|(at, _, _)| *at == pos).map(|(_, slots, mode)| (slots.clone(), *mode)).unwrap_or(([None, None], 0));
            let fluids = game.world.chunks.__horizon_mixins_tconstruct_tank_fluid_context().as_ref()
                .and_then(|value| value.as_ref().ok()).map(|(fluids, _)| fluids.clone()).unwrap_or_default();
            crate::game::tconstruct_source_fluid_container::transfer(&mut state.tank, &mut slots,
                crate::game::tconstruct_source_fluid_container::Direction::from_index(mode),
                &|item| game.res.items().get(item).is_some(), &|fluid| fluids.contains(fluid));
            if crate::game::tconstruct_source_melter_store::write(&mut game.world, pos, "tconstruct:smeltery", state.to_nbt()) {
                *host.hub.guests[gi].__horizon_mixins_tconstruct_smeltery_guest_containers_mut() = Some((pos, slots, mode));
            }
        }
        return Break(());
    }
    let key = (conn, host.hub.guests[gi].placed_at, pos);
    let max = crate::game::tconstruct_source_smeltery_layout::max_offset(state.slots.len());
    let current = game.world.chunks.__horizon_mixins_tconstruct_smeltery_guest_pages().get(&key).map_or(0, |page| page.0);
    let next = if id == 32000 { current.saturating_sub(1) } else if id == 32001 { current.saturating_add(1).min(max) } else { ((id - 33000) as usize).min(max) };
    game.world.chunks.__horizon_mixins_tconstruct_smeltery_guest_pages_mut().insert(key, (next, state.slots.len()));
    let containers = host.hub.guests[gi].__horizon_mixins_tconstruct_smeltery_guest_containers().as_ref()
        .filter(|(at, _, _)| *at == pos).map(|(_, slots, _)| slots.clone()).unwrap_or([None, None]);
    let mut items = crate::game::tconstruct_source_smeltery_page::project_with_containers(&state.slots, next, &containers);
    items.extend((9..36).chain(0..9).map(|i| host.hub.guests[gi].inventory[i].clone()));
    let net = items.iter().map(|item| host.hub.stack(item)).collect();
    let carried = host.hub.guests[gi].carried.clone();
    let carried = host.hub.stack(&carried);
    let guest = &mut host.hub.guests[gi];
    let Some(menu) = guest.menu.as_mut() else { return Break(()); };
    menu.state = (menu.state + 1) & 0x7FFF;
    let packet = horizon_protocol::packets::play::ContainerSetContent { container: window, state: menu.state, items: net, carried };
    menu.sent = items;
    guest.send(&horizon_protocol::packets::play::Clientbound::ContainerSetData(horizon_protocol::packets::play::ContainerSetData { container: window, id: 30003, value: next as i16 }));
    guest.send(&horizon_protocol::packets::play::Clientbound::ContainerSetContent(packet));
    Break(())
}
