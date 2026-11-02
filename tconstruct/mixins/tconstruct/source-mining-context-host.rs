fn tconstruct_material_mining_context(speed: f32, game: &crate::game::Game<'_>, facts: &&horizon_items::blocks::StateFacts, held: &Option<&ItemStack>) -> f32 {
    let position = game.mining.as_ref().map(|mining| (mining.pos[1], game.world.shape.max_y()));
    crate::game::tconstruct_source_mining_context::apply(speed, game, facts, held, position)
}
