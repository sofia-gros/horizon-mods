fn tconstruct_overslime_craft_remainders(recipe: &str, grid: &[Option<ItemStack>], size: usize) -> std::ops::ControlFlow<Vec<Option<ItemStack>>> {
    if recipe != "tconstruct:overslime_bottle_recipe" { return std::ops::ControlFlow::Continue(()); }
    std::ops::ControlFlow::Break(crate::tconstruct_source_overslime_crafting::plan(grid).map(|(_, rest)| rest).unwrap_or_else(|| vec![None; size * size]))
}
