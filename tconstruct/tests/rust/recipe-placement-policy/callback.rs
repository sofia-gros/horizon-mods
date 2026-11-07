// Test-only consumer policy. No production registration or shared content rule.
use pluto_inventory_api::recipe_book::placement::{Decision, Form, Phase, PreviewMode, Request};
pub fn policy(request: &Request<'_>) -> Decision {
    let marked = matches!(request.form, Form::Shaped { .. })
        && request.ingredients.is_some_and(|cells| {
            cells.iter().flatten().any(|ingredient| {
                ingredient
                    .custom()
                    .is_some_and(|(kind, _)| kind == "tconstruct:fake_metal_conversion")
            })
        });
    if !marked {
        return Decision::Pass;
    }
    let preview = request.phase == Phase::LocalAttempt
        && request
            .resolved_id
            .starts_with("tconstruct:fake_metal_native/")
        && request
            .display
            .is_some_and(|display| display.has_ingredient_previews);
    Decision::Deny(if preview {
        PreviewMode::IfLocal
    } else {
        PreviewMode::Silent
    })
}
