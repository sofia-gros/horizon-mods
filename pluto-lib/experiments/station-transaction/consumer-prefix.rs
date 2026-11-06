// Excluded candidate. This module lives under game::recipe_book, beside the
// unchanged transaction body, not in the lower client library Recipes layer.
use super::pluto_transactions::{self, RegistrationError, Transaction};
use std::sync::OnceLock;

static REGISTERED: OnceLock<Result<(), RegistrationError>> = OnceLock::new();

fn interest(game: &crate::game::Game<'_>) -> bool {
    // Preserve stale/invalid sessions as claimed: the old transaction body
    // rejects them. Filtering them here would incorrectly enable native fallback.
    crate::game::tconstruct_crafting_station::position(game).is_some()
}

pub(crate) fn ensure() {
    let result = REGISTERED.get_or_init(|| pluto_transactions::register(Transaction {
        id: "tconstruct:crafting-station-transaction",
        interest,
        handle: tconstruct_crafting_recipe_guard,
    }));
    if let Err(error) = result {
        panic!("TConstruct station transaction registration failed: {error:?}");
    }
}

// The legacy function follows with its complete signature and body unchanged.
