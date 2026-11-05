fn tconstruct_travelers_crafting_preview(
    value: Option<(String, horizon_client::items::ItemStack)>,
    game: &crate::game::Game<'_>,
) -> Option<(String, horizon_client::items::ItemStack)> {
    if game.remote.is_some() || game.world_tables.is_none() {
        return value;
    }
    let size = game.craft_size;
    let grid: Vec<_> = (0..size * size)
        .map(|slot| game.craft_grid[slot].clone())
        .collect();
    let scale = |id: i32| game.maps.data.get(&id).map(|map| map.scale);
    game.recipes()
        .find_with(&grid, size, &scale)
        .map(|(name, result)| (name.to_owned(), result))
}
