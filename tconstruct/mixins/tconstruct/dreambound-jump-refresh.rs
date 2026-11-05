fn tconstruct_dreambound_jump_refresh(game: &mut crate::game::Game<'_>) {
    let capacity = if game.remote.is_some() {
        crate::game::tconstruct_dreambound_jump_wire::matching_capacity(
            game.world
                .chunks
                .__horizon_mixins_tconstruct_dreambound_jump_client()
                .as_ref(),
            &game.armor,
        )
    } else {
        crate::game::tconstruct_dreambound_jump_levels::capacity(&game.world, &game.armor)
    }
    .unwrap_or(0);
    if let Some(player) = game.player.as_mut() {
        *player
            .status
            .__horizon_mixins_tconstruct_dreambound_jump_capacity_mut() = capacity;
        // Removing/re-equipping armor in midair must not replenish spent jumps.
    }
}
