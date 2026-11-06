fn tconstruct_broad_wire_receive(game: &mut Game<'_>, packet: &C) {
    // A reload invalidates the old classification until a fresh host frame.
    if matches!(packet, C::UpdateTags(_)) {
        *game.world.chunks.__horizon_mixins_tconstruct_broad_wire_client_mut() = None;
        return;
    }
    let C::CustomPayload(message) = packet else { return; };
    if message.channel != "tconstruct:overwield_broad" { return; }
    *game.world.chunks.__horizon_mixins_tconstruct_broad_wire_client_mut() =
        crate::game::tconstruct_source_broad_wire::decode(&message.data);
}
