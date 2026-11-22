// Read-only admission and provider assembly before the native matching/assembly body.
use super::{Ingredient as NativeIngredient, Input, ItemStack, Recipe, Shape};
use crate::pluto_component_view::ItemFacts;
use horizon_world::nbt::Tag;
use pluto_inventory_api::{
    advancements::predicates::ComponentValue,
    crafting::{
        assembly::{self, Outcome as AssemblyOutcome},
        matching::{self, InputFacts, Outcome, RecipeFacts, Request, TemplateFacts},
    },
    recipe_book::placement::{Form, Ingredient, IngredientShape},
};

// The placement crate's Ingredient facts over the native crafting
// ingredient, verbatim from the retired package's recipe-placement adapter.
// Only this trait implementation is composed here: that file's definition()
// provider is the recipe-placement veto, which stays native-preserved.
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

pub enum Verdict {
    Native,
    Denied,
    Assembled(ItemStack),
}

/// The provider's compound as the stack's saved custom data: the template
/// stack's own component patch with that one component replaced, decoded
/// back into stack form by the game's own component codec.
fn with_custom_data(stack: ItemStack, custom_data: ComponentValue) -> ItemStack {
    let mut components = match crate::item_data::encode_components(&stack) {
        Some(Tag::Compound(map)) => map,
        _ => Default::default(),
    };
    components.insert(
        "minecraft:custom_data".into(),
        crate::pluto_component_view::to_nbt(custom_data),
    );
    let (damage, extra) = crate::item_data::decode_components(Some(&Tag::Compound(components)));
    let mut stack = stack;
    stack.damage = damage;
    stack.extra = extra;
    stack
}

pub fn evaluate(input: &Input<'_>, recipe: &Recipe) -> Verdict {
    if !matching::has_providers() && !assembly::has_providers() {
        return Verdict::Native;
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
    let admission = matching::prepare(&facts);
    let assemblers = assembly::prepare(&facts);
    if admission.is_none() && assemblers.is_none() {
        return Verdict::Native;
    }
    // Inspect exact cropped cells, not a compressed stack list or synthetic grid.
    // No component capture occurs until at least one policy requests inspection.
    // Crafting capture reads the stacks' current decoded components even when
    // they retain wire: grid stacks in real gameplay always do, and the native
    // ingredient test the veto defers to reads the same data.
    let captures: Vec<_> = input
        .cells
        .iter()
        .map(|cell| cell.map(ItemFacts::capture_crafting))
        .collect();
    let cells: Vec<_> = captures
        .iter()
        .map(|cell| cell.as_ref().map(ItemFacts::view))
        .collect();
    let result = template.map(|template| template.create());
    let result = result.as_ref().map(ItemFacts::capture_crafting);
    let request = Request {
        recipe: &facts,
        input: InputFacts {
            width: input.width,
            height: input.height,
            left: input.left,
            top: input.top,
            cells: &cells,
        },
        result: result.as_ref().map(ItemFacts::view),
    };
    if let Some(evaluation) = admission {
        if matches!(evaluation.evaluate(&request), Outcome::Denied { .. }) {
            return Verdict::Denied;
        }
    }
    if let Some(evaluation) = assemblers {
        match evaluation.evaluate(&request) {
            AssemblyOutcome::Denied { .. } => return Verdict::Denied,
            AssemblyOutcome::Assembled { custom_data, .. } => {
                let Some(template) = template else {
                    return Verdict::Native;
                };
                return Verdict::Assembled(with_custom_data(template.create(), custom_data));
            }
            AssemblyOutcome::Native => {}
        }
    }
    Verdict::Native
}
