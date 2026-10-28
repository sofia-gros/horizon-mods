fn material_block_wire_prune(hub: &mut crate::lan::hub::Hub, ctx: &mut crate::lan::hub::TickCtx<'_>) {
    // tick_level may return before track_entities when nobody is present.
    // Prune there too so a disconnected connection cannot retain cache state.
    ctx.world.chunks.__horizon_mixins_tconstruct_material_wire_sent_mut().retain(|(conn, joined, pos), _|
        hub.guests.iter().any(|g| g.conn == *conn && g.placed_at == *joined && hub.here(g)
            && g.sent_chunks.contains(&(pos[0] >> 4, pos[2] >> 4))));
}
