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
    let result_stack = template.map(|template| template.create());
    let result = result_stack.as_ref().map(ItemFacts::capture);
    let stored = StoredStacks { cells: &input.cells, result: result_stack.as_ref() };
    evaluation.evaluate_with_stored(&Request {
        recipe: &facts,
        input: InputFacts {
            width: input.width,
            height: input.height,
            left: input.left,
            top: input.top,
            cells: &cells,
        },
        result: result.as_ref().map(ItemFacts::view),
    }, &stored)
}

// Exact borrowed saved-field access; never derives an effective component patch
// and never rejects the reader merely because a stack retains opaque wire.
fn stored_value(value: &horizon_world::nbt::Tag) -> matching::StoredValue<'_> {
    use horizon_world::nbt::Tag as T;
    use matching::StoredValue as V;
    match value {
        T::End => V::End, T::Byte(v) => V::Byte(*v), T::Short(v) => V::Short(*v),
        T::Int(v) => V::Int(*v), T::Long(v) => V::Long(*v), T::Float(v) => V::Float(*v),
        T::Double(v) => V::Double(*v), T::ByteArray(v) => V::ByteArray(v),
        T::String(v) => V::String(&v.0), T::List(_) => V::List,
        T::Compound(_) => V::Compound, T::IntArray(v) => V::IntArray(v),
        T::LongArray(v) => V::LongArray(v),
    }
}
fn stored_path<'a>(stack: &'a crate::items::ItemStack, id: &str, path: &[&str]) -> Option<matching::StoredValue<'a>> {
    let mut value = stack.data()?.other(id)?;
    for key in path { value = value.get(key)?; }
    Some(stored_value(value))
}
struct StoredStacks<'a> {
    cells: &'a [Option<&'a crate::items::ItemStack>],
    result: Option<&'a crate::items::ItemStack>,
}
impl matching::StoredComponents for StoredStacks<'_> {
    fn input_other(&self, cell: usize, id: &str, path: &[&str]) -> Option<matching::StoredValue<'_>> {
        stored_path(self.cells.get(cell).copied().flatten()?, id, path)
    }
    fn result_other(&self, id: &str, path: &[&str]) -> Option<matching::StoredValue<'_>> {
        stored_path(self.result?, id, path)
    }
}
