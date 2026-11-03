fn tconstruct_anvil_material_recipe(input: &Input<'_>, recipe: &Recipe) -> std::ops::ControlFlow<Option<Option<ItemStack>>> {
    use horizon_world::nbt::Tag;
    use std::ops::ControlFlow::{Break, Continue};
    let Recipe::Shaped { width, height, cells, result } = recipe else { return Continue(()); };
    let output = result.create();
    // The source recipe uses forge:difference: a workbench other than the
    // crafting station itself. The shared shaped matcher handles its tag;
    // retain the subtraction before accepting its result.
    if &*output.item == "tconstruct:crafting_station" && *width == 1 && *height == 2
        && cells.first().and_then(Option::as_ref).is_some_and(|ingredient| ingredient.items.len() == 1 && ingredient.items.contains("tconstruct:pattern"))
        && cells.get(1).and_then(Option::as_ref).is_some_and(|ingredient| ingredient.items.contains("tconstruct:crafting_station")) {
        if input.stacks().into_iter().any(|stack| &*stack.item == "tconstruct:crafting_station") { return Break(None); }
        return Continue(());
    }
    if !matches!(&*output.item, "tconstruct:tinkers_anvil" | "tconstruct:scorched_anvil") { return Continue(()); }
    let expected = output.data().and_then(|data| data.other("minecraft:custom_data"))
        .and_then(|data| data.get("tconstruct:anvil")).and_then(|data| data.get("RequiredMaterial"));
    let Some(Tag::String(expected)) = expected else { return Continue(()); };
    let expected = expected.to_string_lossy();
    let fake: Vec<_> = input.stacks().into_iter().filter(|stack| &*stack.item == "tconstruct:fake_storage_block").collect();
    if fake.len() != 3 || fake.iter().any(|stack| {
        !matches!(stack.data().and_then(|data| data.other("minecraft:custom_data")).and_then(|data| data.get("Material")),
            Some(Tag::String(material)) if material.to_string_lossy() == expected)
    }) { return Break(None); }
    Continue(())
}
