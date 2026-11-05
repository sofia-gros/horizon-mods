fn tconstruct_dreambound_jump_send(
    hub: &mut crate::lan::hub::Hub,
    ctx: &mut crate::lan::hub::TickCtx<'_>,
) {
    // Do not advertise movement the dedicated prediction replay cannot yet
    // reproduce. This does not change or disable any anticheat setting.
    let prediction = hub.recording();
    ctx.world
        .chunks
        .__horizon_mixins_tconstruct_dreambound_jump_sent_mut()
        .retain(|(conn, joined), _| {
            hub.guests
                .iter()
                .any(|guest| guest.conn == *conn && guest.placed_at == *joined && hub.here(guest))
        });
    for guest in &hub.guests {
        if !hub.here(guest) {
            continue;
        }
        let capacity = if prediction {
            None
        } else {
            crate::tconstruct_dreambound_jump_levels::capacity(&*ctx.world, &guest.armor)
        };
        let data = crate::lan::hub::tconstruct_dreambound_jump_wire::encode(&guest.armor, capacity);
        let entry = ctx
            .world
            .chunks
            .__horizon_mixins_tconstruct_dreambound_jump_sent_mut()
            .entry((guest.conn, guest.placed_at))
            .or_default();
        if !crate::lan::hub::tconstruct_dreambound_jump_wire::needs_send(entry, &data) {
            continue;
        }
        guest.send(&C::CustomPayload(
            horizon_protocol::packets::configuration::CustomPayload {
                channel: "tconstruct:dreambound_jumps".into(),
                data,
            },
        ));
    }
}
