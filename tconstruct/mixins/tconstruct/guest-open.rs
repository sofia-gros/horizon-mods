fn open_guest_tconstruct(game: &mut crate::game::Game<'_>, host: &mut crate::game::lan::LanHost, conn: u32, pos: (i32, i32, i32)) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
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
