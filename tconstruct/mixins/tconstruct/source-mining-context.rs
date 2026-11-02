fn tconstruct_material_mining_context(speed: f32, game: &crate::game::Game<'_>, facts: &&horizon_items::blocks::StateFacts, held: &Option<&ItemStack>, pos: [i32; 3]) -> f32 {
    crate::game::tconstruct_source_mining_context::apply(speed, game, facts, held, Some((pos[1], game.world.shape.max_y())))
}
