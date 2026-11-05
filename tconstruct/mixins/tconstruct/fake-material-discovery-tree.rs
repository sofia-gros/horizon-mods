fn tconstruct_fake_material_discovery_tree(value: Arc<Tree>, game: &mut crate::game::Game<'_>) -> Arc<Tree> {
    crate::game::tconstruct_source_fake_material_discovery_world::apply(game, value)
}
