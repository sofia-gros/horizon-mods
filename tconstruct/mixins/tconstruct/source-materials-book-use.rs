fn tconstruct_use_materials_book(game: &mut Game<'_>, stack: &ItemStack) -> std::ops::ControlFlow<bool> {
    if tconstruct_materials_book::open(game, stack, false) { std::ops::ControlFlow::Break(true) }
    else { std::ops::ControlFlow::Continue(()) }
}
