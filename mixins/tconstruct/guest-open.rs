fn open_guest_tconstruct(game: &mut crate::game::Game<'_>, host: &mut crate::game::lan::LanHost, conn: u32, pos: (i32, i32, i32)) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    let name = &game.res.facts().get(game.world.block(pos.0, pos.1, pos.2)).name;
    let title = match &**name {
        "tconstruct:part_builder" => "TConstruct Part Builder",
        "tconstruct:tinker_station" => "TConstruct Tinker Station",
        _ => return Continue(()),
    };
    let slots = horizon_client::lan::menu::Slots::Block(pos);
    let Some(items) = host.hub.guest(conn).and_then(|g| slots.read(&game.world, g)) else { return Break(false); };
    game.guest_container_closed(host, conn);
    game.lan_station_closed(host, conn, None);
    host.hub.open_container(conn, "minecraft:generic_9x3", horizon_protocol::text::literal(title), pos, slots, items);
    Break(true)
}
