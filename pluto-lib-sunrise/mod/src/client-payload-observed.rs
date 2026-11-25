fn after_native_packet(value: (), game: &Game<'_>, packet: &ClientboundPlayPacket) {
    if game.options.execution_role.is_client() {
        sunrise_client::pluto_client_payload::observe(packet);
    }
    value
}
