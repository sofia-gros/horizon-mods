fn tconstruct_area_guest_left(game: &mut crate::game::Game<'_>, guest: &mut horizon_client::lan::guest::Guest) {
    game.world.chunks.__horizon_mixins_tconstruct_area_guest_hits_mut().remove(&guest.conn);
    game.world.chunks.__horizon_mixins_tconstruct_transform_guest_hits_mut().remove(&guest.conn);
}
