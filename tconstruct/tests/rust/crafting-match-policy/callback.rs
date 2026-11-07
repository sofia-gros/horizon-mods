// Test-only consumer policy for bounded source-generated fake-metal forms.
// This is not production activation or a general full native identity check.
use pluto_inventory_api::{
    advancements::predicates::{ComponentText, ComponentValue, ItemSnapshot},
    crafting::matching::{Decision, RecipeFacts, Request},
    recipe_book::placement::Form,
};
const MARKER: &str = "tconstruct:fake_metal_conversion";
pub fn interest(recipe: &RecipeFacts<'_>) -> bool {
    matches!(recipe.form, Form::Shaped { .. })
        && recipe.ingredients.is_some_and(|cells| {
            cells
                .iter()
                .flatten()
                .any(|i| i.custom().is_some_and(|(kind, _)| kind == MARKER))
        })
}
fn material(item: ItemSnapshot<'_>) -> Option<String> {
    let ComponentValue::String(value) = item
        .components?
        .get("minecraft:custom_data")?
        .get("Material")?
    else {
        return None;
    };
    Some(String::from_utf16_lossy(&value.0))
}
pub fn policy(request: &Request<'_>) -> Decision {
    let Some(output) = request.result else {
        return Decision::Deny;
    };
    let (side, input, count) = match output.id {
        "tconstruct:fake_storage_block" => (3, "tconstruct:fake_ingot", 1),
        "tconstruct:fake_ingot" => (1, "tconstruct:fake_storage_block", 9),
        _ => return Decision::Deny,
    };
    let Some(expected) = material(output) else {
        return Decision::Deny;
    };
    let Some(cells) = request.recipe.ingredients else {
        return Decision::Deny;
    };
    if request.recipe.form
        != (Form::Shaped {
            width: side,
            height: side,
        })
        || cells.len() != side * side
        || cells.iter().any(|cell| {
            !cell.is_some_and(|i| {
                i.accepted_ids().len() == 1
                    && i.accepted_ids().contains(input)
                    && i.children().is_none()
                    && i.custom().is_some_and(|(kind, data)| {
                        kind == MARKER
                            && data.len() == 1
                            && data.get("material").and_then(serde_json::Value::as_str)
                                == Some(expected.as_str())
                    })
            })
        })
    {
        return Decision::Deny;
    }
    if request.input.width != side
        || request.input.height != side
        || request.input.cells.len() != side * side
        || request.input.cells.iter().any(|s| {
            !s.is_some_and(|s| {
                s.id == input && s.count > 0 && material(s).as_deref() == Some(expected.as_str())
            })
        })
    {
        return Decision::Deny;
    }
    let expected_components = ComponentValue::Compound(std::collections::BTreeMap::from([(
        ComponentText::from("minecraft:custom_data"),
        ComponentValue::Compound(std::collections::BTreeMap::from([(
            ComponentText::from("Material"),
            ComponentValue::String(ComponentText::from(expected.as_str())),
        )])),
    )]));
    if output.count != count
        || output.damage != 0
        || output.components != Some(&expected_components)
    {
        return Decision::Deny;
    }
    Decision::Pass
}
