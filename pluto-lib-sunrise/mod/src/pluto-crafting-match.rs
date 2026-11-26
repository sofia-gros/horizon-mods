// Read-only admission and provider assembly before the native matching/assembly body.
use sunrise_client::crafting::{Context, Shape, Special};
use sunrise_client::crafting::ingredient::{Ingredient as NativeIngredient, Input};
use sunrise_items::items::ItemStack;

/// The provider-id qualification the native crafting table applies: an
/// unqualified id is read in the minecraft namespace.
fn qualified(id: &str) -> String {
    if id.contains(':') { id.to_string() } else { format!("minecraft:{id}") }
}
use sunrise_client::crafting::Recipe;
use sunrise_client::crafting::Recipes;
use crate::pluto_component_view::ItemFacts;
use sunrise_core::nbt::Tag;
use pluto_lib::pluto_inventory_api::{
    advancements::predicates::ComponentValue,
    crafting::{
        assembly::{self, Outcome as AssemblyOutcome},
        matching::{self, InputFacts, Outcome, RecipeFacts, Request, TemplateFacts},
        special,
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
    /// A special-recipe marker entry produces no result for this
    /// grid: its provider does not answer it, or a native entry
    /// matches the grid and stands instead of the marker.
    Unmatched,
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

pub fn evaluate(ctx: &Context, input: &Input<'_>, recipe: &Recipe) -> Verdict {
    // The recipe's loaded id: the tables store it beside the recipe, and
    // every caller reaches this function with the recipe borrowed from that
    // table, so the entry whose recipe is this very one names it. A recipe
    // the table does not hold (never the case from the match call sites)
    // reports no id, and identity-based admission passes it by.
    let id = ctx
        .recipes
        .crafting
        .iter()
        .find(|(_, stored)| std::ptr::eq(stored, recipe))
        .map(|(name, _)| name.as_str());
    // Special-recipe providers answer through marker entries planted
    // in the table under their own ids (see `evaluate_marker`); a
    // marker resolves here, before the admission/assembly flow, whose
    // Form::Special handling is the vanilla repair path a marker must
    // never reach.
    if special::has_providers() {
        if let Some(verdict) = evaluate_marker(ctx, input, recipe, id) {
            return verdict;
        }
    }
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
        id,
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

/// The special-recipe seam: the answer for a marker entry, or None
/// when `recipe` is not one.
///
/// A marker is a table entry planted for one registered provider:
/// `Recipe::Special(Special::RepairItem)` under the provider's own
/// id, contributed as a data-pack recipe file (and mirrored into
/// server-synced tables on receipt), so the host's own scan forms
/// the result pair with the provider's identity. The recipe book
/// assigns special entries no display number, so a marker shifts no
/// numbering anywhere. This replaces the retired find_with HEAD,
/// whose borrowed `(&str, ItemStack)` result no cancellable helper
/// signature can spell: the weaver requires the host's exact
/// spelling, lifetimes included, and rustc elision cannot produce
/// it from the helper's borrowed inputs.
///
/// A marker always resolves here, never through the native flow:
/// - when a native (non-marker) entry matches the grid, the marker
///   is Unmatched and the scan goes on to that entry: a native match
///   always stands, wherever the marker sorts in the table. The
///   replay skips marker entries, so it never re-enters this path.
/// - otherwise the provider registered under the entry's id is
///   asked: its stack is Assembled, no stack is Unmatched.
///
/// A genuine vanilla or data-pack repair entry — the same payload
/// under a name no provider registered — is not a marker and
/// returns None, so vanilla item repair keeps its native behaviour.
fn evaluate_marker(
    ctx: &Context,
    input: &Input<'_>,
    recipe: &Recipe,
    id: Option<&str>,
) -> Option<Verdict> {
    if !matches!(recipe, Recipe::Special(Special::RepairItem)) {
        return None;
    }
    let qualified = qualified(id?);
    let providers = special::provider_ids();
    if !providers.iter().any(|p| *p == qualified.as_str()) {
        return None;
    }
    let is_marker = |name: &str, stored: &Recipe| {
        matches!(stored, Recipe::Special(Special::RepairItem))
            && providers.iter().any(|p| *p == qualified(name).as_str())
    };
    for (name, stored) in &ctx.recipes.crafting {
        if std::ptr::eq(stored, recipe) || is_marker(name, stored) {
            continue;
        }
        if Recipes::assemble_if_matches(ctx, input, stored).is_some() {
            return Some(Verdict::Unmatched);
        }
    }
    let captures: Vec<_> = input
        .cells
        .iter()
        .map(|cell| cell.map(ItemFacts::capture_crafting))
        .collect();
    let cells: Vec<_> = captures
        .iter()
        .map(|cell| cell.as_ref().map(ItemFacts::view))
        .collect();
    let facts = InputFacts {
        width: input.width,
        height: input.height,
        left: input.left,
        top: input.top,
        cells: &cells,
    };
    let Some(spec) = special::evaluate_provider(&qualified, &facts) else {
        return Some(Verdict::Unmatched);
    };
    let stack = ItemStack {
        item: spec.item.into(),
        count: spec.count,
        damage: spec.damage,
        extra: None,
    };
    let stack = match spec.custom_data {
        Some(data) => with_custom_data(stack, data),
        None => stack,
    };
    Some(Verdict::Assembled(stack))
}
