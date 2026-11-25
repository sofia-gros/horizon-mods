// Read-only native recipe facts and veto-only definition boundary.
use super::{Ingredient as NativeIngredient, Recipe, Recipes, Shape};
use pluto_inventory_api::recipe_book::placement::{
    self, Form, Ingredient, IngredientShape, Outcome, Phase, Request,
};
impl Ingredient for NativeIngredient {
    fn accepted_ids(&self) -> &std::collections::HashSet<String> {
        &self.items
    }
    fn display_order(&self) -> &[String] {
        &self.order
    }
    fn custom(&self) -> Option<(&str, &serde_json::Map<String, serde_json::Value>)> {
        self.custom
            .as_ref()
            .map(|(kind, data)| (kind.as_str(), data))
    }
    fn children(&self) -> Option<(IngredientShape, usize)> {
        self.children.as_ref().map(|(kind, children)| {
            (
                match kind {
                    Shape::Compound => IngredientShape::Compound,
                    Shape::Intersection => IngredientShape::Intersection,
                    Shape::Difference => IngredientShape::Difference,
                },
                children.len(),
            )
        })
    }
    fn child(&self, index: usize) -> Option<&dyn Ingredient> {
        self.children
            .as_ref()?
            .1
            .get(index)
            .map(|child| child as &dyn Ingredient)
    }
}
/// Mirrors native lookup order and name normalization only. No RecipeIndex,
/// ingredient schema, source-family marker or ownership rule is consulted.
pub fn definition(recipes: &Recipes, id: &str) -> Outcome {
    if !placement::has_providers() {
        return Outcome::Native;
    }
    let name = id.trim_start_matches("minecraft:");
    if let Some(cooking) = recipes.cooking.iter().find(|c| c.id == name) {
        let ingredients = [Some(&cooking.ingredient as &dyn Ingredient)];
        return placement::evaluate(&Request {
            requested_id: id,
            resolved_id: &cooking.id,
            phase: Phase::Definition,
            form: Form::Cooking,
            ingredients: Some(&ingredients),
            display: None,
        });
    }
    let Some((resolved, recipe)) = recipes.crafting.iter().find(|(n, _)| n == name) else {
        return Outcome::Native;
    };
    let (form, ingredients): (_, Option<Vec<Option<&dyn Ingredient>>>) = match recipe {
        Recipe::Shaped {
            width,
            height,
            cells,
            ..
        } => (
            Form::Shaped {
                width: *width,
                height: *height,
            },
            Some(
                cells
                    .iter()
                    .map(|cell| cell.as_ref().map(|i| i as &dyn Ingredient))
                    .collect(),
            ),
        ),
        Recipe::Shapeless { ingredients, .. } => (
            Form::Shapeless,
            Some(
                ingredients
                    .iter()
                    .map(|i| Some(i as &dyn Ingredient))
                    .collect(),
            ),
        ),
        Recipe::Special(_) => (Form::Special, None),
    };
    placement::evaluate(&Request {
        requested_id: id,
        resolved_id: resolved,
        phase: Phase::Definition,
        form,
        ingredients: ingredients.as_deref(),
        display: None,
    })
}
