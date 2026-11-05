fn tconstruct_fake_material_book_overlay(
    value: Vec<(i32, i32, Vec<ItemStack>)>, entry: &Entry,
) -> Vec<(i32, i32, Vec<ItemStack>)> {
    if !entry.id.starts_with("tconstruct:fake_metal_native/") || entry.stacks.is_empty() { return value; }
    entry.grid_cells(3, 3).into_iter().filter_map(|(ingredient, cell)| {
        let i = ingredient?;
        let (_, stacks) = entry.stacks.iter().find(|(index, _)| *index == i)?;
        Some((cell as i32 % 3, cell as i32 / 3, stacks.clone()))
    }).collect()
}
