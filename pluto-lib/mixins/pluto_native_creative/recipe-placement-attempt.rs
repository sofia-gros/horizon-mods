// UI veto precedes the native local menu operation and remote packet send.
use super::Game;
use horizon_client::{
    pluto_inventory::recipe_book::{
        placement::{self, DisplayFacts, Form, Ingredient, Outcome, Phase, PreviewMode, Request},
        preview::{BookKind, Preview},
    },
    recipe_book::{BookType, Display},
};
pub(crate) fn attempt(
    game: &mut Game<'_>,
    book: BookType,
    entry_index: usize,
) -> std::ops::ControlFlow<()> {
    if !placement::has_providers() {
        return std::ops::ControlFlow::Continue(());
    }
    let Some(entry) = game
        .recipe_ui
        .book
        .as_ref()
        .and_then(|book| book.entries.get(entry_index))
    else {
        return std::ops::ControlFlow::Continue(());
    };
    let kind = match book {
        BookType::Crafting => BookKind::Crafting,
        BookType::Furnace => BookKind::Furnace,
        BookType::BlastFurnace => BookKind::BlastFurnace,
        BookType::Smoker => BookKind::Smoker,
    };
    let (form, ingredients): (_, Vec<Option<&dyn Ingredient>>) = match &entry.display {
        Display::Shaped {
            width,
            height,
            cells,
        } => (
            Form::Shaped {
                width: *width,
                height: *height,
            },
            cells
                .iter()
                .map(|cell| cell.as_ref().map(|i| i as &dyn Ingredient))
                .collect(),
        ),
        Display::Shapeless(ingredients) => (
            Form::Shapeless,
            ingredients
                .iter()
                .map(|i| Some(i as &dyn Ingredient))
                .collect(),
        ),
        Display::Furnace(ingredient) => (Form::Cooking, vec![Some(ingredient as &dyn Ingredient)]),
    };
    let outcome = placement::evaluate(&Request {
        requested_id: &entry.id,
        resolved_id: &entry.id,
        phase: if game.remote.is_some() {
            Phase::RemoteAttempt
        } else {
            Phase::LocalAttempt
        },
        form,
        ingredients: Some(&ingredients),
        display: Some(DisplayFacts {
            book: kind,
            display_id: entry.display_id,
            has_ingredient_previews: !entry.stacks.is_empty(),
        }),
    });
    match outcome {
        Outcome::Native => std::ops::ControlFlow::Continue(()),
        Outcome::Denied { preview, .. } => {
            if preview == PreviewMode::IfLocal && game.remote.is_none() {
                // Presentation is best-effort and never turns denial into permission.
                super::pluto_preview::with_game(game, |capability| {
                    if let Ok(selected) = capability.select(kind, entry_index) {
                        let _ = capability.show(&selected);
                    }
                });
            }
            std::ops::ControlFlow::Break(())
        }
    }
}
