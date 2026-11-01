fn tconstruct_anvil_material_recipe(input: &Input<'_>, recipe: &Recipe) -> std::ops::ControlFlow<Option<Option<ItemStack>>> {
    use horizon_world::nbt::Tag;
    use std::ops::ControlFlow::{Break, Continue};
    let Recipe::Shaped { result, .. } = recipe else { return Continue(()); };
    let output = result.create();
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
