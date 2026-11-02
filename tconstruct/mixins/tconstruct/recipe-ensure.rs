fn tconstruct_ensure_recipes(value: (), game: &mut crate::game::Game<'_>) -> () {
    if game.fns.loaded && game.remote.is_none() && game.res.rest.get().is_some() {
        let current_mods = tconstruct_recipe_reload::mod_snapshot(game);
        if game.world.chunks.__horizon_mixins_tconstruct_recipe_snapshot().is_none()
            || game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().is_none()
            || game.world.chunks.__horizon_mixins_tconstruct_recipe_mod_snapshot().as_ref() != Some(&current_mods) {
            tconstruct_recipe_reload::capture(game);
        }
    }
    value
}
