// Native menu transaction authority; separate from placement admission.
// Excluded candidate. An interested handler owns this invocation completely.
use super::{BookType, Game};
use std::collections::BTreeMap;
use std::ops::ControlFlow;
use std::sync::{LazyLock, RwLock};

pub type Interest = for<'g> fn(&Game<'g>) -> bool;
pub type Handle = for<'g> fn(&mut Game<'g>, BookType, usize, bool) -> ControlFlow<()>;

#[derive(Clone, Copy)]
pub struct Transaction {
    pub id: &'static str,
    pub interest: Interest,
    pub handle: Handle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistrationError { EmptyId, DuplicateId }

#[derive(Default)]
struct Registry { entries: RwLock<BTreeMap<&'static str, Transaction>> }
impl Registry {
    fn insert(&self, entry: Transaction) -> Result<(), RegistrationError> {
        if entry.id.is_empty() { return Err(RegistrationError::EmptyId); }
        let mut entries = self.entries.write().expect("recipe transaction registry poisoned");
        if entries.contains_key(entry.id) { return Err(RegistrationError::DuplicateId); }
        entries.insert(entry.id, entry);
        Ok(())
    }
    fn snapshot(&self) -> Vec<Transaction> {
        self.entries.read().expect("recipe transaction registry poisoned").values().copied().collect()
    }
}
static TRANSACTIONS: LazyLock<Registry> = LazyLock::new(Registry::default);

/// Register before the consumer publishes a session handled by this callback.
/// Selectors must be read-only; handlers must not reenter Game::place_recipe.
/// Neither signature nor this registry proves those semantic obligations.
pub fn register(entry: Transaction) -> Result<(), RegistrationError> {
    TRANSACTIONS.insert(entry)
}

fn dispatch(registry: &Registry, game: &mut Game<'_>, book: BookType,
    entry_index: usize, use_max: bool) -> ControlFlow<()> {
    let entries = registry.snapshot();
    let mut selected = None;
    let mut conflict = false;
    // No registry lock is held during interest or the transaction callback.
    for entry in entries {
        if (entry.interest)(game) {
            if selected.is_some() { conflict = true; }
            else { selected = Some(entry.handle); }
        }
    }
    if conflict { return ControlFlow::Break(()); }
    let Some(handle) = selected else { return ControlFlow::Continue(()); };
    // A claimed handler is the only transaction. Even its Continue cannot run
    // the native body afterwards. No rollback of callback effects is claimed.
    let _ = handle(game, book, entry_index, use_max);
    ControlFlow::Break(())
}

/// The proposed single HEAD keeps veto/preview first. Only admission Continue
/// may select a transaction; no matching transaction preserves native execution.
pub fn attempt(game: &mut Game<'_>, book: BookType,
    entry_index: usize, use_max: bool) -> ControlFlow<()> {
    match super::pluto_placement::attempt(game, book, entry_index) {
        ControlFlow::Break(()) => ControlFlow::Break(()),
        ControlFlow::Continue(()) => dispatch(&TRANSACTIONS, game, book, entry_index, use_max),
    }
}
