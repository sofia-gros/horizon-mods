fn smeltery_guest_button(game: &mut crate::game::Game<'_>, host: &mut crate::game::lan::LanHost, conn: u32, window: i32, id: i32) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    if !matches!(id, 32000 | 32001) { return Continue(()); }
    let Some(gi) = host.hub.guests.iter().position(|guest| guest.conn == conn) else { return Continue(()); };
    let Some(menu) = host.hub.guests[gi].menu.as_ref() else { return Continue(()); };
    if menu.window != window || !menu.slots.still_valid(&game.world, &host.hub.guests[gi], menu.pos) { return Continue(()); }
    let Some(pos) = menu.pos else { return Continue(()); };
    let Some(state) = crate::game::tconstruct_source_melter_store::read(&game.world, pos, "tconstruct:smeltery")
        .and_then(|tag| crate::game::tconstruct_source_smeltery_state::Smeltery::from_nbt(&tag)) else { return Continue(()); };
    let key = (conn, host.hub.guests[gi].placed_at, pos);
    let max = crate::game::tconstruct_source_smeltery_layout::max_offset(state.slots.len());
    let current = game.world.chunks.__horizon_mixins_tconstruct_smeltery_guest_pages().get(&key).copied().unwrap_or(0);
    let next = if id == 32000 { current.saturating_sub(1) } else { current.saturating_add(1).min(max) };
    game.world.chunks.__horizon_mixins_tconstruct_smeltery_guest_pages_mut().insert(key, next);
    host.hub.send_menu_content(gi, crate::game::tconstruct_source_smeltery_page::project(&state.slots, next));
    host.hub.guests[gi].send(&horizon_protocol::packets::play::Clientbound::ContainerSetData(horizon_protocol::packets::play::ContainerSetData { container: window, id: 30003, value: next as i16 }));
    Break(())
}
