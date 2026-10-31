fn open_guest_tconstruct(game: &mut crate::game::Game<'_>, host: &mut crate::game::lan::LanHost, conn: u32, pos: (i32, i32, i32)) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    let machine = game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name.clone();
    if matches!(&*machine, "tconstruct:smeltery_controller" | "tconstruct:foundry_controller") {
        game.ensure_datapacks();
        if !crate::game::tconstruct_source_machine_runtime::ensure(game, pos, &machine) { return Break(false); }
        crate::game::tconstruct_source_machine_runtime::refresh_smeltery(game, pos);
        if crate::game::tconstruct_source_smeltery_world::detect(game, pos).is_none() { return Break(false); }
        let Some(state) = crate::game::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:smeltery")
            .and_then(|tag| crate::game::tconstruct_source_smeltery_state::Smeltery::from_nbt(&tag)) else { return Break(false); };
        if state.slots.is_empty() { return Break(false); }
        let slots = horizon_client::lan::menu::Slots::Block(pos);
        let Some(guest) = host.hub.guest(conn) else { return Break(false); };
        let key = (conn, guest.placed_at, pos);
        game.world.chunks.__horizon_mixins_tconstruct_smeltery_guest_pages_mut().insert(key, (0, state.slots.len()));
        game.guest_container_closed(host, conn);
        game.lan_station_closed(host, conn, None);
        let Some(guest) = host.hub.guest_mut(conn) else { return Break(false); };
        *guest.__horizon_mixins_tconstruct_smeltery_guest_containers_mut() = Some((pos, [None, None], 0));
        let Some(items) = host.hub.guest(conn).and_then(|guest| slots.read(&game.world, guest)) else { return Break(false); };
        let title = if &*machine == "tconstruct:foundry_controller" { "TConstruct Foundry" } else { "TConstruct Smeltery" };
        host.hub.open_container(conn, "minecraft:generic_9x6", horizon_protocol::text::literal(title), pos, slots, items);
        if let Some(guest) = host.hub.guest(conn) {
            if let Some(window) = guest.menu.as_ref().map(|menu| menu.window) {
                guest.send(&horizon_protocol::packets::play::Clientbound::ContainerSetData(horizon_protocol::packets::play::ContainerSetData { container: window, id: 30002, value: state.slots.len() as i16 }));
                guest.send(&horizon_protocol::packets::play::Clientbound::ContainerSetData(horizon_protocol::packets::play::ContainerSetData { container: window, id: 30003, value: 0 }));
            }
        }
        return Break(true);
    }
    if matches!(&*machine, "tconstruct:seared_melter" | "tconstruct:seared_heater") {
        game.ensure_datapacks();
        if !crate::game::tconstruct_source_machine_runtime::ensure(game, pos, &machine) { return Break(false); }
        let heater = (pos.0, pos.1 - 1, pos.2);
        let has_heater = &*machine == "tconstruct:seared_melter"
            && &*game.res.facts().get(game.world.block(heater.0, heater.1, heater.2)).name == "tconstruct:seared_heater"
            && crate::game::tconstruct_source_machine_runtime::ensure(game, heater, "tconstruct:seared_heater");
        let slots = if has_heater { horizon_client::lan::menu::Slots::Double(pos, heater) } else { horizon_client::lan::menu::Slots::Block(pos) };
        let Some(items) = host.hub.guest(conn).and_then(|guest| slots.read(&game.world, guest)) else { return Break(false); };
        game.guest_container_closed(host, conn);
        game.lan_station_closed(host, conn, None);
        let title = if &*machine == "tconstruct:seared_melter" { "TConstruct Melter" } else { "TConstruct Heater" };
        let menu = if has_heater { "minecraft:generic_9x6" } else { "minecraft:generic_9x3" };
        host.hub.open_container(conn, menu, horizon_protocol::text::literal(title), pos, slots, items);
        return Break(true);
    }
    let name = &game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name;
    let mut title = match &**name {
        "tconstruct:part_builder" => "TConstruct Part Builder",
        "tconstruct:tinker_station" => "TConstruct Tinker Station",
        _ => return Continue(()),
    };
    game.ensure_datapacks();
    let side = if title == "TConstruct Part Builder" { crate::game::tconstruct_ui::side_inventory(game, pos, true) } else { None };
    let partner = side.and_then(|side| game.chest_partner(side).map(|(partner, _)| partner));
    if let Some(partner) = partner {
        game.world.chunks.__horizon_mixins_tconstruct_builder_double_partner_mut().insert(pos, partner);
        title = "TConstruct Part Builder Double";
    } else { game.world.chunks.__horizon_mixins_tconstruct_builder_double_partner_mut().remove(&pos); }
    let slots = side.map_or(horizon_client::lan::menu::Slots::Block(pos), |side| horizon_client::lan::menu::Slots::Double(pos, side));
    let Some(items) = host.hub.guest(conn).and_then(|g| slots.read(&game.world, g)) else { return Break(false); };
    game.guest_container_closed(host, conn);
    game.lan_station_closed(host, conn, None);
    let menu = match items.len() { 36 => "minecraft:generic_9x4", 54 | 81 => "minecraft:generic_9x6", _ => "minecraft:generic_9x3" };
    host.hub.open_container(conn, menu, horizon_protocol::text::literal(title), pos, slots, items);
    if title.starts_with("TConstruct Part Builder") {
        let count = side.and_then(|side| game.world.block_entities.get(&side)).map_or(0, |entity| entity.slots().len()) + partner.and_then(|pos| game.world.block_entities.get(&pos)).map_or(0, |entity| entity.slots().len());
        if let Some(guest) = host.hub.guest(conn) {
            if let Some(window) = guest.menu.as_ref().map(|menu| menu.window) {
                guest.send(&horizon_protocol::packets::play::Clientbound::ContainerSetData(horizon_protocol::packets::play::ContainerSetData { container: window, id: 30000, value: count as i16 }));
                let title = side.map_or(0, |side| crate::game::tconstruct_source_side_layout::title_code(&game.res.facts().get(game.world.block(side.0, side.1, side.2)).name));
                guest.send(&horizon_protocol::packets::play::Clientbound::ContainerSetData(horizon_protocol::packets::play::ContainerSetData { container: window, id: 30001, value: title }));
            }
        }
    }
    Break(true)
}
