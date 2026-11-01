fn tconstruct_armor_wear_start(game: &mut crate::game::Game<'_>) {
    game.world.chunks.__horizon_mixins_tconstruct_armor_wear_context_mut()
        .insert(game.lan_acting, Vec::new());
}
