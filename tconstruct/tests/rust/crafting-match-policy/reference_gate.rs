use crate::crafting::{Input,Recipe};
use horizon_items::items::ItemStack;
use crate::reference_plan as tconstruct_source_fake_material_crafting;
fn tconstruct_fake_material_recipe(input: &Input<'_>, recipe: &Recipe) -> std::ops::ControlFlow<Option<Option<ItemStack>>> {
    use horizon_world::nbt::Tag;
    use std::ops::ControlFlow::{Break, Continue};
    const MARKER: &str = "tconstruct:fake_metal_conversion";
    let Recipe::Shaped { width, height, cells, result } = recipe else { return Continue(()); };
    if !cells.iter().flatten().any(|ingredient| ingredient.custom.as_ref().is_some_and(|(id, _)| id == MARKER)) {
        return Continue(());
    }
    let output = result.create();
    let to_block = match &*output.item {
        "tconstruct:fake_storage_block" => true,
        "tconstruct:fake_ingot" => false,
        _ => return Break(None),
    };
    let side = if to_block { 3 } else { 1 };
    let input_id = if to_block { "tconstruct:fake_ingot" } else { "tconstruct:fake_storage_block" };
    let Some(Tag::String(expected)) = output.data().and_then(|data| data.other("minecraft:custom_data"))
        .and_then(|data| data.get("Material")) else { return Break(None); };
    let expected = expected.to_string_lossy();
    if *width != side || *height != side || cells.len() != side * side
        || cells.iter().any(|cell| !cell.as_ref().is_some_and(|ingredient|
            ingredient.items.len() == 1 && ingredient.items.contains(input_id) && ingredient.children.is_none()
            && ingredient.custom.as_ref().is_some_and(|(id, data)| id == MARKER && data.len() == 1
                && data.get("material").and_then(serde_json::Value::as_str) == Some(expected.as_str())))) {
        return Break(None);
    }
    let Some(planned) = tconstruct_source_fake_material_crafting::plan(
        &input.stacks(), input.width, input.height, to_block,
        |material, item| material == expected && item == &*output.item,
    ) else { return Break(None); };
    if planned.count != output.count || !planned.same_item_same_components(&output) { return Break(None); }
    // Native matching and consumption still run. This is a read-only material
    // gate for explicitly marked projections, not a second crafting transaction.
    Continue(())
}

pub fn denies(input: &Input<'_>, recipe: &Recipe) -> bool {matches!(tconstruct_fake_material_recipe(input,recipe), std::ops::ControlFlow::Break(_))}
