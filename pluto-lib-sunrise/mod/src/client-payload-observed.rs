fn after_native_packet(value: (), game: &Game<'_>, packet: &ClientPacket<'_>) {
    if game.options.execution_role.is_client() {
        // ClientPacket is a view: only custom payloads expose data, and
        // observe() only ever reads those — forward exactly them.
        if let Some(cp) = packet.custom_payload {
            crate::pluto_client_payload::observe(&sunrise_protocol::packets::play::Clientbound::CustomPayload(
                sunrise_protocol::packets::custom_payload::CustomPayload {
                    channel: cp.channel.to_string(),
                    data: cp.data.to_vec(),
                },
            ));
        }
    }
    value
}
