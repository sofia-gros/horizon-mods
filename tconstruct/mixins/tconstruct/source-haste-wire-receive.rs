fn tconstruct_haste_wire_receive(game: &mut Game<'_>, packet: &C) -> std::ops::ControlFlow<Result<()>> {
    use std::ops::ControlFlow::{Break, Continue};
    if matches!(packet, C::UpdateTags(_)) {
        *game.world.chunks.__horizon_mixins_tconstruct_haste_wire_client_mut() = None;
        return Continue(());
    }
    let C::CustomPayload(message) = packet else { return Continue(()); };
    if message.channel != "tconstruct:haste_chest" { return Continue(()); }
    *game.world.chunks.__horizon_mixins_tconstruct_haste_wire_client_mut() =
        crate::game::tconstruct_source_haste_wire::decode(&message.data);
    Break(Ok(()))
}
