// Candidate policy port only. No module/startup hook installs this file yet.
use crate::pluto_inventory::{
    crafting::matching::{self, Decision, RecipeFacts, RegistrationError, StoredRegistration, StoredRequest, StoredValue},
    recipe_book::placement::Form,
};

pub(super) fn register() -> Result<(), RegistrationError> {
    matching::register_stored(StoredRegistration {
        id: "tconstruct:crafting-stored-denials",
        interest,
        callback: evaluate,
    })
}
fn interest(recipe: &RecipeFacts<'_>) -> bool {
    matches!(recipe.form, Form::Shaped { .. }) && recipe.template.is_some_and(|result| {
        matches!(result.item, Some("tconstruct:crafting_station" | "tconstruct:tinkers_anvil" | "tconstruct:scorched_anvil" | "tconstruct:seared_melter" | "tconstruct:seared_fluid_cannon"))
    })
}
fn evaluate(request: &StoredRequest<'_>) -> Decision {
    // Preserve the old anvil/subtraction policy before the empty-tank policy.
    if anvil(request) == Decision::Deny { return Decision::Deny; }
    empty_tank(request)
}
fn anvil(request: &StoredRequest<'_>) -> Decision {
    let r = request.matching;
    let Form::Shaped { width, height } = r.recipe.form else { return Decision::Pass; };
    let Some(output) = r.result else { return Decision::Deny; };
    let Some(cells) = r.recipe.ingredients else { return Decision::Deny; };
    if output.id == "tconstruct:crafting_station" && width == 1 && height == 2
        && cells.first().and_then(|i| *i).is_some_and(|ingredient| ingredient.accepted_ids().len() == 1 && ingredient.accepted_ids().contains("tconstruct:pattern"))
        && cells.get(1).and_then(|i| *i).is_some_and(|ingredient| ingredient.accepted_ids().contains("tconstruct:crafting_station")) {
        if r.input.cells.iter().flatten().any(|stack| stack.id == "tconstruct:crafting_station") { return Decision::Deny; }
        return Decision::Pass;
    }
    if !matches!(output.id, "tconstruct:tinkers_anvil" | "tconstruct:scorched_anvil") { return Decision::Pass; }
    let expected = request.stored.result_other("minecraft:custom_data", &["tconstruct:anvil", "RequiredMaterial"])
        .and_then(StoredValue::string_units);
    let Some(expected) = expected else { return Decision::Pass; };
    let expected = String::from_utf16_lossy(expected);
    let fake: Vec<_> = r.input.cells.iter().enumerate().filter_map(|(i, stack)| {
        stack.as_ref().filter(|s| s.id == "tconstruct:fake_storage_block").map(|_| i)
    }).collect();
    if fake.len() != 3 || fake.iter().any(|&i| {
        !matches!(request.stored.input_other(i, "minecraft:custom_data", &["Material"]).and_then(StoredValue::string_units),
            Some(material) if String::from_utf16_lossy(material) == expected)
    }) { return Decision::Deny; }
    Decision::Pass
}
fn empty_tank(request: &StoredRequest<'_>) -> Decision {
    let r = request.matching;
    if !matches!(r.recipe.form, Form::Shaped { .. }) { return Decision::Pass; }
    let Some(output) = r.result else { return Decision::Deny; };
    if !matches!(output.id, "tconstruct:seared_melter" | "tconstruct:seared_fluid_cannon") { return Decision::Pass; }
    for (i, stack) in r.input.cells.iter().enumerate() {
        if stack.as_ref().is_some_and(|s| matches!(s.id, "tconstruct:seared_fuel_gauge" | "tconstruct:seared_ingot_gauge"))
            && request.stored.input_other(i, "minecraft:custom_data", &["tconstruct:tank"]).is_some() {
            return Decision::Deny;
        }
    }
    Decision::Pass
}
