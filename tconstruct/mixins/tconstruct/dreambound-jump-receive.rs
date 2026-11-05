fn tconstruct_dreambound_jump_receive(
    game: &mut Game<'_>,
    packet: &C,
) -> std::ops::ControlFlow<Result<()>> {
    use std::ops::ControlFlow::{Break, Continue};
    if matches!(packet, C::UpdateTags(_)) {
        *game
            .world
            .chunks
            .__horizon_mixins_tconstruct_dreambound_jump_client_mut() = None;
        return Continue(());
    }
    let C::CustomPayload(message) = packet else {
        return Continue(());
    };
    if message.channel != "tconstruct:dreambound_jumps" {
        return Continue(());
    }
    *game
        .world
        .chunks
        .__horizon_mixins_tconstruct_dreambound_jump_client_mut() =
        crate::game::tconstruct_dreambound_jump_wire::decode(&message.data);
    Break(Ok(()))
}
