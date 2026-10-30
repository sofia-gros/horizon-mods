fn material_block_wire_prune(hub: &mut crate::lan::hub::Hub, ctx: &mut crate::lan::hub::TickCtx<'_>) {
    use horizon_world::nbt::Tag;
    use std::collections::HashSet;
    ctx.world.chunks.__horizon_mixins_tconstruct_builder_preview_sent_mut().retain(|(conn, joined, window, pos), _|
        hub.guests.iter().any(|guest| guest.conn == *conn && guest.placed_at == *joined && hub.here(guest)
            && guest.menu.as_ref().is_some_and(|menu| menu.window == *window && menu.pos == Some(*pos))));
    ctx.world.chunks.__horizon_mixins_tconstruct_station_preview_sent_mut().retain(|(conn, joined, window, pos), _|
        hub.guests.iter().any(|guest| guest.conn == *conn && guest.placed_at == *joined && hub.here(guest)
            && guest.menu.as_ref().is_some_and(|menu| menu.window == *window && menu.pos == Some(*pos))));
    ctx.world.chunks.__horizon_mixins_tconstruct_builder_cast_wire_sent_mut().retain(|(conn, joined, window, pos), _|
        hub.guests.iter().any(|guest| guest.conn == *conn && guest.placed_at == *joined && hub.here(guest)
            && guest.menu.as_ref().is_some_and(|menu| menu.window == *window && menu.pos == Some(*pos))));
    let block = hub.sync.builtin.get("minecraft:block").and_then(|r| r.id("tconstruct:fake_storage_block"));
    let kind = hub.sync.builtin.get("minecraft:block_entity_type").and_then(|r| r.id("tconstruct:material_block"));
    let ids = block.zip(kind);
    *ctx.world.chunks.__horizon_mixins_tconstruct_material_wire_ids_mut() = ids;
    if ids.is_none() || !hub.guests.iter().any(|guest| hub.here(guest)) {
        // The released prototype does not register the block yet: do not
        // copy guest chunk sets or scan world data in the common case.
        ctx.world.chunks.__horizon_mixins_tconstruct_material_wire_before_mut().clear();
        ctx.world.chunks.__horizon_mixins_tconstruct_material_wire_sent_mut().clear();
        return;
    }
    // Only membership of chunks containing an original material entity can
    // affect whether its first update was already part of the chunk packet.
    let material_id = Tag::String("tconstruct:material_block".into());
    let relevant: HashSet<_> = ctx.world.chunks.iter().filter_map(|(key, chunk)| {
        let saved = chunk.saved.as_deref()?;
        saved.block_entities.iter().any(|raw|
            raw.get("id") == Some(&material_id))
            .then_some(*key)
    }).collect();
    let before = ctx.world.chunks.__horizon_mixins_tconstruct_material_wire_before_mut();
    before.clear();
    for guest in &hub.guests {
        if hub.here(guest) {
            before.insert((guest.conn, guest.placed_at), guest.sent_chunks.iter()
                .filter(|chunk| relevant.contains(*chunk)).copied().collect());
        }
    }
    // tick_level may return before track_entities when nobody is present.
    // Prune there too so a disconnected connection cannot retain cache state.
    ctx.world.chunks.__horizon_mixins_tconstruct_material_wire_sent_mut().retain(|(conn, joined, pos), _|
        hub.guests.iter().any(|g| g.conn == *conn && g.placed_at == *joined && hub.here(g)
            && g.sent_chunks.contains(&(pos[0] >> 4, pos[2] >> 4))));
}
