// The creative inventory is extended from PlutoLib's API crate (item_group_api_v1); this module is how the injection inside the game
// reaches that code. The API crate registers its function once when PlutoLib starts.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

static DOCUMENT: OnceLock<fn(&[String], usize) -> serde_json::Value> = OnceLock::new();
static BUSY: AtomicBool = AtomicBool::new(false);

pub fn set_document(document: fn(&[String], usize) -> serde_json::Value) {
    let _ = DOCUMENT.set(document);
}

/// The tabs and entries to add, in `creative-inventory.json` form, given the ids of the tabs already there and how many are mod tabs.
pub fn document(existing: &[String], mod_tabs: usize) -> Option<serde_json::Value> {
    DOCUMENT.get().map(|document| document(existing, mod_tabs))
}

/// True for the caller that may build the extra tabs now; false while it is already being done (building them builds a creative
/// inventory, which would ask again).
pub fn begin() -> bool {
    !BUSY.swap(true, Ordering::SeqCst)
}

pub fn end() {
    BUSY.store(false, Ordering::SeqCst);
}
