fn tconstruct_crafting_lan_recipe_guard(
    game: &mut crate::game::Game<'_>,
    host: &mut LanHost,
    conn: u32,
    packet: &horizon_protocol::packets::recipes::PlaceRecipe,
) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(guest) = host.hub.guests.iter().find(|guest| guest.conn == conn) else { return Continue(()); };
    let tracked = game.world.chunks.__horizon_mixins_tconstruct_crafting_guest_sessions()
        .contains_key(&(conn, guest.placed_at, packet.container));
    let station = host.play.get(&conn).and_then(|play| play.crafting.table.as_ref())
        .filter(|(window, _, _, _)| *window == packet.container)
        .is_some_and(|(_, _, _, pos)| &*game.res.facts().get(game.world.block(pos[0], pos[1], pos[2])).name == "tconstruct:crafting_station");
    if !tracked && !station { return Continue(()); }
    // Vanilla's LAN recipe placement only updates its transient menu, not
    // this station's authoritative block entity. Refuse before it consumes.
    game.lan_send_table_content(host, conn);
    Break(())
}
