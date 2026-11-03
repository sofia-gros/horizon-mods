// Send changes immediately and periodically refresh guests whose world was reset.
fn tconstruct_broad_wire_send(hub: &mut crate::lan::hub::Hub, ctx: &mut crate::lan::hub::TickCtx<'_>) {
    let tags = ctx.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()
        .and_then(|result| result.as_ref().ok()).and_then(|tags| tags.get("tconstruct:modifiable/broad"));
    let data = crate::lan::hub::tconstruct_source_broad_wire::encode(tags);
    let cache = ctx.world.chunks.__horizon_mixins_tconstruct_broad_wire_sent_mut();
    cache.retain(|(conn, joined), _| hub.guests.iter().any(|guest| guest.conn == *conn && guest.placed_at == *joined && hub.here(guest)));
    for guest in &hub.guests {
        if !hub.here(guest) { continue; }
        let key = (guest.conn, guest.placed_at);
        let entry = cache.entry(key).or_default();
        if !crate::lan::hub::tconstruct_source_broad_wire::needs_send(entry, &data) { continue; }
        guest.send(&C::CustomPayload(horizon_protocol::packets::configuration::CustomPayload {
            channel: "tconstruct:overwield_broad".into(), data: data.clone(),
        }));
    }
}
