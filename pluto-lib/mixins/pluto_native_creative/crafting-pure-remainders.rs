fn pluto_crafting_pure_remainders(
    recipe: &str,
    grid: &[Option<ItemStack>],
    size: usize,
) -> std::ops::ControlFlow<Vec<Option<ItemStack>>> {
    crate::crafting::pluto_derivations::remainders(recipe, grid, size)
}
