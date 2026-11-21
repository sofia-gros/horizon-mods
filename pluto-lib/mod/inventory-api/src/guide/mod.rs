//! Bounded read-only guide content. Content, layout, ownership, sessions and
//! transport authorization belong to separately validated adapters/consumers.
pub mod document;

/// Synchronous stored-book facts and conditional native page-turn contract.
pub mod current_book;

/// Explicit in-process correlation, not native session or screen ownership.
pub mod requests;
