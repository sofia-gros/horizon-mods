fn tconstruct_survival_melter_recipe(input: &Input<'_>, recipe: &Recipe) -> std::ops::ControlFlow<Option<Option<ItemStack>>> {
    use std::ops::ControlFlow::{Break, Continue};
    let Recipe::Shaped { result, .. } = recipe else { return Continue(()); };
    if &*result.create().item != "tconstruct:seared_melter" { return Continue(()); }
    for stack in input.stacks() {
        if matches!(&*stack.item, "tconstruct:seared_fuel_gauge" | "tconstruct:seared_ingot_gauge") {
            let tank = stack.data().and_then(|data| data.other("minecraft:custom_data")).and_then(|data| data.get("tconstruct:tank"));
            if !tconstruct_survival_empty_tank::empty(tank) { return Break(None); }
        }
    }
    Continue(())
}
