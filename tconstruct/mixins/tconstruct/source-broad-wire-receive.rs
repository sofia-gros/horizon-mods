fn tconstruct_broad_wire_receive(game: &mut Game<'_>, packet: &C) -> std::ops::ControlFlow<Result<()>> {
    use std::ops::ControlFlow::{Break, Continue};
    // A reload invalidates the old classification until a fresh host frame.
    if matches!(packet, C::UpdateTags(_)) {
        *game.world.chunks.__horizon_mixins_tconstruct_broad_wire_client_mut() = None;
        return Continue(());
    }
    let C::CustomPayload(message) = packet else { return Continue(()); };
    if message.channel != "tconstruct:overwield_broad" { return Continue(()); }
    *game.world.chunks.__horizon_mixins_tconstruct_broad_wire_client_mut() =
        crate::game::tconstruct_source_broad_wire::decode(&message.data);
    Break(Ok(()))
}
