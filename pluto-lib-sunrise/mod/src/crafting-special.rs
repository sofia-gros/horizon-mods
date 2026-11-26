// Special recipes fill a grid the native lookup left unmatched: the
// adapter replays the native scan first, so a native match continues into
// the real body, and only a native miss breaks with a provider's answer.
fn pluto_crafting_special(
    recipes: &Recipes,
    grid: &[Option<ItemStack>],
    size: usize,
    map_scale: &dyn Fn(i32) -> Option<u8>,
) -> std::ops::ControlFlow<Option<(&str, ItemStack)>> {
    match sunrise_client_lib::pluto_matching::evaluate_special(recipes, grid, size, map_scale) {
        Some(result) => std::ops::ControlFlow::Break(result),
        None => std::ops::ControlFlow::Continue(()),
    }
}
