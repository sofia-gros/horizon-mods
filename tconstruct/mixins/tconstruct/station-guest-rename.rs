fn station_guest_rename(game: &mut crate::game::Game<'_>, host: &mut crate::game::lan::LanHost, conn: u32, name: &str) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(guest) = host.hub.guests.iter().find(|guest| guest.conn == conn) else { return Continue(()); };
    let Some(menu) = guest.menu.as_ref() else { return Continue(()); };
    let Some(pos) = menu.pos else { return Continue(()); };
    let state = game.world.block(pos.0, pos.1, pos.2).0 as usize;
    let station = host.hub.sync.builtin.get("minecraft:block").and_then(|r| r.id("tconstruct:tinker_station"));
    if station.is_none() || host.hub.encoder.block_ids.get(state).copied() != station { return Continue(()); }
    if guest.spectator() || !menu.slots.still_valid(&game.world, guest, menu.pos) { return Break(()); }
    let can_name = game.world.block_entities.get(&pos).is_some_and(|entity| {
        !crate::game::tconstruct_source_station_world::has_operation_tool(entity.slots())
            && crate::game::tconstruct_source_station_world::building_plan(&game.world, entity.slots()).is_some()
    });
    if !can_name { return Break(()); }
    let normalized: String = horizon_client::clipboard::filter_text(name, false).chars().take(50).collect();
    game.world.chunks.__horizon_mixins_tconstruct_station_guest_names_mut().insert((conn, menu.window, pos), normalized);
    Break(())
}
