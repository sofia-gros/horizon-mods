fn tconstruct_travelers_crafter_result(
    value: Option<(String, horizon_client::items::ItemStack)>,
    game: &crate::game::Game<'_>,
    grid: &[Option<horizon_client::items::ItemStack>],
) -> Option<(String, horizon_client::items::ItemStack)> {
    if game.world_tables.is_none() {
        return value;
    }
    game.recipes()
        .find(grid, 3)
        .map(|(name, result)| (name.to_owned(), result))
        .filter(|(_, result)| result.count > 0)
}
