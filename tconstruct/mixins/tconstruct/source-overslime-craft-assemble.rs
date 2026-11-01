fn tconstruct_overslime_craft_assemble(input: &Input<'_>, recipe: &Recipe) -> std::ops::ControlFlow<Option<Option<ItemStack>>> {
    let Recipe::Shapeless { result, .. } = recipe else { return std::ops::ControlFlow::Continue(()); };
    if result.item.as_deref() != Some("tconstruct:overslime_bottle_recipe") { return std::ops::ControlFlow::Continue(()); }
    let grid: Vec<Option<ItemStack>> = input.stacks().into_iter().map(|stack| Some(stack.clone())).collect();
    std::ops::ControlFlow::Break(crate::tconstruct_source_overslime_crafting::plan(&grid).map(|(output, _)| Some(output)))
}
