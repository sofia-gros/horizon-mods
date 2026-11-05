fn tconstruct_dreambound_jump_request(game: &mut crate::game::Game<'_>) {
    let pending = game.player.as_mut().is_some_and(|player| {
        std::mem::take(
            player
                .status
                .__horizon_mixins_tconstruct_dreambound_jump_pending_mut(),
        )
    });
    if pending && game.remote.is_some() {
        // Called before native input and position packets for this client tick.
        game.send(S::CustomPayload(
            horizon_protocol::packets::configuration::CustomPayload {
                channel: "tconstruct:dreambound_jump".into(),
                data: vec![1],
            },
        ));
    }
}
