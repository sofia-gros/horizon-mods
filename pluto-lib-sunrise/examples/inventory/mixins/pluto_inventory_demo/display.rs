use std::sync::atomic::{AtomicBool, Ordering};
static VERIFIED: AtomicBool = AtomicBool::new(false);
pub(super) fn mark() {
    VERIFIED.store(true, Ordering::Relaxed);
}
pub(super) fn verified() -> bool {
    VERIFIED.load(Ordering::Relaxed)
}
