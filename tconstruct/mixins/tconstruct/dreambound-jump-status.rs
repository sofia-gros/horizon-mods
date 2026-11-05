// Effect refresh must not refill an airborne actor's spent jump budget.
fn tconstruct_dreambound_jump_status(
    mut status: horizon_client::effects::StatusFlags,
    game: &crate::game::Game<'_>,
) -> horizon_client::effects::StatusFlags {
    if let Some(player) = game.player.as_ref() {
        *status.__horizon_mixins_tconstruct_dreambound_jump_capacity_mut() = *player
            .status
            .__horizon_mixins_tconstruct_dreambound_jump_capacity();
        *status.__horizon_mixins_tconstruct_dreambound_jumps_used_mut() = *player
            .status
            .__horizon_mixins_tconstruct_dreambound_jumps_used();
        *status.__horizon_mixins_tconstruct_dreambound_jump_pending_mut() = *player
            .status
            .__horizon_mixins_tconstruct_dreambound_jump_pending();
    }
    status
}
