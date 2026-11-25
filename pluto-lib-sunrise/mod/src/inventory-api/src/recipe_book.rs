//! Scoped recipe-book presentation invalidation, independent of a native host.
//!
//! The caller decides whether changed definitions or display IDs need a refresh.
//! This capability does not edit recipes, grant knowledge or place ingredients.

/// A borrowed capability for the current recipe-book presentation cache.
///
/// Invalidation discards the derived model, knowledge comparison, visible view,
/// ghost/alternative previews and remembered placement. The next ordinary host
/// read rebuilds what it needs; this call does not force a rebuild or event.
/// Search/navigation preferences, saved knowledge and inventory are retained.
pub trait Invalidation {
    fn invalidate_definitions(&mut self);
}

pub mod preview;

/// Opt-in, one-shot saved knowledge handoff across native world opening.
pub mod handoff;

/// Explicit value-based current-index updates, without persistent ownership.
pub mod index;

pub mod placement;
