// Read-only admission before the native matching/assembly body.
use super::{Input, Recipe};
use crate::pluto_component_view::ItemFacts;
use pluto_inventory_api::{
    crafting::matching::{self, InputFacts, Outcome, RecipeFacts, Request, TemplateFacts},
    recipe_book::placement::{Form, Ingredient},
};

pub fn evaluate(input: &Input<'_>, recipe: &Recipe) -> Outcome {
    if !matching::has_providers() {
        return Outcome::Native;
    }
    let (form, ingredients, template): (_, Option<Vec<Option<&dyn Ingredient>>>, _) = match recipe {
        Recipe::Shaped {
            width,
            height,
            cells,
            result,
        } => (
            Form::Shaped {
                width: *width,
                height: *height,
            },
            Some(
                cells
                    .iter()
                    .map(|c| c.as_ref().map(|i| i as &dyn Ingredient))
                    .collect(),
            ),
            Some(result),
        ),
        Recipe::Shapeless {
            ingredients,
            result,
        } => (
            Form::Shapeless,
            Some(
                ingredients
                    .iter()
                    .map(|i| Some(i as &dyn Ingredient))
                    .collect(),
            ),
            Some(result),
        ),
        Recipe::Special(_) => (Form::Special, None, None),
    };
    let facts = RecipeFacts {
        form,
        ingredients: ingredients.as_deref(),
        template: template.map(|t| TemplateFacts {
            item: t.item.as_deref(),
            count: t.count,
            damage: t.damage,
            has_components: t.extra.is_some(),
        }),
    };
    let Some(evaluation) = matching::prepare(&facts) else {
        return Outcome::Native;
    };
    // Inspect exact cropped cells, not a compressed stack list or synthetic grid.
    // No component capture occurs until at least one policy requests inspection.
    let captures: Vec<_> = input
        .cells
        .iter()
        .map(|cell| cell.map(ItemFacts::capture))
        .collect();
    let cells: Vec<_> = captures
        .iter()
        .map(|cell| cell.as_ref().map(ItemFacts::view))
        .collect();
    let result = template.map(|template| template.create());
    let result = result.as_ref().map(ItemFacts::capture);
    evaluation.evaluate(&Request {
        recipe: &facts,
        input: InputFacts {
            width: input.width,
            height: input.height,
            left: input.left,
            top: input.top,
            cells: &cells,
        },
        result: result.as_ref().map(ItemFacts::view),
    })
}
