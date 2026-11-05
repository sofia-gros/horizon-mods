fn tconstruct_offhand_death(game: &mut crate::game::Game<'_>, player: u128) {
    game.world
        .chunks
        .__horizon_mixins_tconstruct_offhand_cooldown_mut()
        .remove(&horizon_client::lan::playerdata::uuid_ints(player));
}
