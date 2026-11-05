// update_world runs after the server enters each level and before its tick.
// Rebuild an invalidated parked level before it can send a stale tag snapshot.
fn tconstruct_recipes_after_level_enter(game: &mut crate::game::Game<'_>) {
    if game.remote.is_none() && !game.options.execution_role.is_client()
        && game.res.rest.get().is_some() && !game.world_root.as_os_str().is_empty()
        && game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().is_none() {
        game.ensure_datapacks();
    }
    if game.remote.is_none() && !game.options.execution_role.is_client() {
        crate::game::tconstruct_source_fake_material_world::restore_saved_book(game);
    }
}
