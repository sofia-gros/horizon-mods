fn pluto_level_facts_head(game: &crate::game::Game<'_>, level: horizon_client::mod_api::LevelRef) {
    if let Some(world) = game.native_level_world(level) {
        crate::entry::pluto_level_facts::publish_logical_height(level.dimension(), world.shape.logical_height);
    }
}
