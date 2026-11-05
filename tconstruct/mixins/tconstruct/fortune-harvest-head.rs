fn tconstruct_fortune_harvest_head(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    state: horizon_world::registry::BlockStateId,
    held: Option<&horizon_items::items::ItemStack>,
    block_entity: &Option<horizon_world::nbt::Tag>,
) -> std::ops::ControlFlow<Vec<horizon_items::items::ItemStack>> {
    crate::game::tconstruct_fortune_harvest_world::drops(game, pos, state, held, block_entity)
}
