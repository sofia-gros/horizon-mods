// Actual public API usage, not a simulated native Book or ItemStack.
// Source-only example: its own Cargo compilation is pending.
use pluto_inventory_api::recipe_book::presentation::{Error, Presentation, Replacement};

/// The consumer selects the entry and prepares the full display-only values.
/// This short example intentionally drops rejected proposals and returns the
/// error classification. Use the original Rejected value to retain payloads.
pub fn show_first_ingredient<S>(
    book: &mut dyn Presentation<Stack = S>,
    entry_index: usize,
    alternatives: Vec<S>,
) -> Result<(), Error> {
    let entry = book.entry(entry_index).ok_or(Error::MissingEntry(entry_index))?;
    let proposal = Replacement::for_entry(entry).with_ingredient(0, alternatives);
    book.replace(vec![proposal]).map_err(|rejected| rejected.error)
}

// No entry-point or registration side effect. A supported native construction
// scope supplies this capability; the example does not install that scope.
fn main() {}
